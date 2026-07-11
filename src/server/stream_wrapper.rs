use std::pin::Pin;

use anyhow::anyhow;
use bytes::{BufMut, Bytes, BytesMut};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    select, spawn, sync,
};
use tokio_rustls::server;

pub enum TcpStreamWrapper {
    Raw(TcpStream),
    TlsServer(server::TlsStream<TcpStream>),
}

impl TcpStreamWrapper {
    pub fn into_split(self) -> (TcpStreamWrapperReadHalf, TcpStreamWrapperWriteHalf) {
        match self {
            TcpStreamWrapper::Raw(tcp_stream) => {
                let (read, write) = tcp_stream.into_split();
                (
                    TcpStreamWrapperReadHalf::TcpStream(read),
                    TcpStreamWrapperWriteHalf::TcpStream(write),
                )
            }
            TcpStreamWrapper::TlsServer(mut tls_stream) => {
                let (tx_read, rx_read) = sync::mpsc::channel(128);
                let (tx_write, mut rx_write) = sync::mpsc::channel(128);

                spawn(async move {
                    let mut read_buffer = BytesMut::with_capacity(8192);
                    loop {
                        select! {
                            Some(message_to_write) = rx_write.recv() => {
                                match message_to_write {
                                    TlsWriterWrapperMessage::Write(bytes) => {
                                        tls_stream.write_all(&bytes).await?
                                    }
                                    TlsWriterWrapperMessage::Flush => tls_stream.flush().await?,
                                    TlsWriterWrapperMessage::Shutdown => tls_stream.shutdown().await?,
                                }
                            }

                            Ok(_) = tls_stream.read_buf(&mut read_buffer) => {
                                tx_read.send(read_buffer.split().freeze()).await?;
                                read_buffer.reserve(8192);
                            }

                            else => {
                                break;
                            }
                        }
                    }
                    Ok::<_, anyhow::Error>(())
                });
                (
                    TcpStreamWrapperReadHalf::TlsStream(rx_read),
                    TcpStreamWrapperWriteHalf::TlsStream(tx_write),
                )
            }
        }
    }
}

impl AsyncRead for TcpStreamWrapper {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match this {
            TcpStreamWrapper::Raw(tcp_stream) => Pin::new(tcp_stream).poll_read(cx, buf),
            TcpStreamWrapper::TlsServer(tls_stream) => Pin::new(tls_stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for TcpStreamWrapper {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        match this {
            TcpStreamWrapper::Raw(tcp_stream) => Pin::new(tcp_stream).poll_write(cx, buf),
            TcpStreamWrapper::TlsServer(tls_stream) => Pin::new(tls_stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match this {
            TcpStreamWrapper::TlsServer(tls_stream) => Pin::new(tls_stream).poll_flush(cx),
            TcpStreamWrapper::Raw(tcp_stream) => Pin::new(tcp_stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match this {
            TcpStreamWrapper::Raw(tcp_stream) => Pin::new(tcp_stream).poll_shutdown(cx),
            TcpStreamWrapper::TlsServer(tls_stream) => Pin::new(tls_stream).poll_shutdown(cx),
        }
    }
}

pub enum TcpStreamWrapperReadHalf {
    TcpStream(OwnedReadHalf),
    TlsStream(sync::mpsc::Receiver<Bytes>),
}

impl AsyncRead for TcpStreamWrapperReadHalf {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match this {
            TcpStreamWrapperReadHalf::TcpStream(owned_read_half) => {
                Pin::new(owned_read_half).poll_read(cx, buf)
            }
            TcpStreamWrapperReadHalf::TlsStream(receiver) => {
                let std::task::Poll::Ready(data) = receiver.poll_recv(cx) else {
                    return std::task::Poll::Pending;
                };

                std::task::Poll::Ready(match data {
                    Some(val) => {
                        buf.put(val);
                        Ok(())
                    }
                    None => Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        anyhow!("Channel was closed"),
                    )),
                })
            }
        }
    }
}

pub enum TcpStreamWrapperWriteHalf {
    TcpStream(OwnedWriteHalf),
    #[allow(private_interfaces)]
    TlsStream(sync::mpsc::Sender<TlsWriterWrapperMessage>),
}

enum TlsWriterWrapperMessage {
    Write(Bytes),
    Flush,
    Shutdown,
}

impl AsyncWrite for TcpStreamWrapperWriteHalf {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        match this {
            TcpStreamWrapperWriteHalf::TcpStream(owned_write_half) => {
                Pin::new(owned_write_half).poll_write(cx, buf)
            }
            TcpStreamWrapperWriteHalf::TlsStream(sender) => {
                match sender.try_send(TlsWriterWrapperMessage::Write(Bytes::copy_from_slice(buf))) {
                    Ok(_) => std::task::Poll::Ready(Ok(buf.len())),
                    Err(err) => match err {
                        sync::mpsc::error::TrySendError::Full(_) => std::task::Poll::Pending,
                        sync::mpsc::error::TrySendError::Closed(_) => {
                            std::task::Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::BrokenPipe,
                                anyhow!("Channel was closed"),
                            )))
                        }
                    },
                }
            }
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match this {
            TcpStreamWrapperWriteHalf::TcpStream(owned_write_half) => {
                Pin::new(owned_write_half).poll_flush(cx)
            }
            TcpStreamWrapperWriteHalf::TlsStream(sender) => {
                match sender.try_send(TlsWriterWrapperMessage::Flush) {
                    Ok(_) => std::task::Poll::Ready(Ok(())),
                    Err(err) => match err {
                        sync::mpsc::error::TrySendError::Full(_) => std::task::Poll::Pending,
                        sync::mpsc::error::TrySendError::Closed(_) => {
                            std::task::Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::BrokenPipe,
                                anyhow!("Channel was closed"),
                            )))
                        }
                    },
                }
            }
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match this {
            TcpStreamWrapperWriteHalf::TcpStream(owned_write_half) => {
                Pin::new(owned_write_half).poll_shutdown(cx)
            }
            TcpStreamWrapperWriteHalf::TlsStream(sender) => {
                match sender.try_send(TlsWriterWrapperMessage::Shutdown) {
                    Ok(_) => std::task::Poll::Ready(Ok(())),
                    Err(err) => match err {
                        sync::mpsc::error::TrySendError::Full(_) => std::task::Poll::Pending,
                        sync::mpsc::error::TrySendError::Closed(_) => {
                            std::task::Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::BrokenPipe,
                                anyhow!("Channel was closed"),
                            )))
                        }
                    },
                }
            }
        }
    }
}
