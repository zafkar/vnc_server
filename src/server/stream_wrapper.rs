use std::pin::Pin;

use anyhow::{Result, anyhow};
use bytes::{BufMut, Bytes, BytesMut};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    select, spawn, sync,
};
use tokio_rustls::{TlsAcceptor, server};

pub enum TcpStreamWrapper {
    Raw(TcpStream),
    TlsServer(server::TlsStream<TcpStream>),
}

impl TcpStreamWrapper {
    pub async fn start_tls(self, acceptor: TlsAcceptor) -> Result<TcpStreamWrapper> {
        match self {
            TcpStreamWrapper::Raw(tcp_stream) => Ok(TcpStreamWrapper::TlsServer(
                acceptor.accept(tcp_stream).await?,
            )),
            TcpStreamWrapper::TlsServer(tls_stream) => Ok(TcpStreamWrapper::TlsServer(tls_stream)),
        }
    }

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

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use rcgen::generate_simple_self_signed;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        spawn,
    };
    use tokio_rustls::{
        TlsAcceptor, TlsConnector,
        rustls::{
            self, RootCertStore,
            pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
        },
    };

    use crate::server::stream_wrapper::TcpStreamWrapper;

    fn test_cert() -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
        let cert = generate_simple_self_signed(vec!["localhost".into()]).unwrap();

        let cert_der = cert.cert.der().clone();

        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()));

        (cert_der, key_der)
    }

    #[tokio::test]
    async fn split_tls_test() {
        let (cert, key_der) = test_cert();
        let tls_server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.clone()], key_der)
            .unwrap();

        let acceptor = TlsAcceptor::from(Arc::new(tls_server_config));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        let addr = listener.local_addr().unwrap();

        spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut wrapper = TcpStreamWrapper::Raw(stream);
            wrapper = wrapper.start_tls(acceptor).await.unwrap();

            let (mut read, mut write) = wrapper.into_split();
            write.write_u16(0xDEAD).await.unwrap();

            assert_eq!(read.read_u16().await.unwrap(), 0xBEEF)
        });

        let mut client_roots = RootCertStore::empty();
        client_roots.add(cert).unwrap();

        let tls_client_config = rustls::ClientConfig::builder()
            .with_root_certificates(client_roots)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(tls_client_config));

        let stream = TcpStream::connect(addr).await.unwrap();

        let mut tls_client_stream = connector
            .connect(
                rustls::pki_types::ServerName::try_from("localhost").unwrap(),
                stream,
            )
            .await
            .unwrap();

        assert_eq!(tls_client_stream.read_u16().await.unwrap(), 0xDEAD);
        tls_client_stream.write_u16(0xBEEF).await.unwrap();
    }
}
