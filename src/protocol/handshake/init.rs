use num_enum::{FromPrimitive, IntoPrimitive};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::{RecvFrom, SendInto, pixel_format::PixelFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ClientInit {
    #[default]
    Share = 0,
    Exclusive = 1,
}

impl RecvFrom for ClientInit {
    async fn recv<S: tokio::io::AsyncRead + Unpin>(mut stream: S) -> anyhow::Result<Self> {
        Ok(stream.read_u8().await?.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInit {
    pub fb_width: u16,
    pub fb_height: u16,
    pub pixel_format: PixelFormat,
    pub name: String,
}

impl SendInto for ServerInit {
    async fn send<S: tokio::io::AsyncWrite + Unpin>(&self, mut stream: S) -> anyhow::Result<()> {
        stream.write_u16(self.fb_width).await?;
        stream.write_u16(self.fb_height).await?;
        self.pixel_format.send(&mut stream).await?;

        let str_bytes = self.name.as_bytes();
        stream.write_u32(str_bytes.len() as u32).await?;
        stream.write_all(str_bytes).await?;

        Ok(())
    }
}
