use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::{RecvFrom, SendInto};

pub type EncodingType = i32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    x_pos: u16,
    y_pos: u16,
    width: u16,
    height: u16,
}

impl RecvFrom for Rect {
    async fn recv<S: tokio::io::AsyncRead + Unpin>(mut stream: S) -> anyhow::Result<Self> {
        let x_pos = stream.read_u16().await?;
        let y_pos = stream.read_u16().await?;
        let width = stream.read_u16().await?;
        let height = stream.read_u16().await?;

        Ok(Self {
            x_pos,
            y_pos,
            width,
            height,
        })
    }
}

impl SendInto for Rect {
    async fn send<S: tokio::io::AsyncWrite + Unpin>(&self, mut stream: S) -> anyhow::Result<()> {
        stream.write_u16(self.x_pos).await?;
        stream.write_u16(self.y_pos).await?;
        stream.write_u16(self.width).await?;
        stream.write_u16(self.height).await?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pos {
    x_pos: u16,
    y_pos: u16,
}

impl RecvFrom for Pos {
    async fn recv<S: tokio::io::AsyncRead + Unpin>(mut stream: S) -> anyhow::Result<Self> {
        let x_pos = stream.read_u16().await?;
        let y_pos = stream.read_u16().await?;

        Ok(Self { x_pos, y_pos })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ColorMapEntry {
    red: u16,
    green: u16,
    blue: u16,
}

impl SendInto for ColorMapEntry {
    async fn send<S: tokio::io::AsyncWrite + Unpin>(&self, mut stream: S) -> anyhow::Result<()> {
        stream.write_u16(self.red).await?;
        stream.write_u16(self.green).await?;
        stream.write_u16(self.blue).await?;

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Color8888 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl SendInto for Color8888 {
    async fn send<S: tokio::io::AsyncWrite + Unpin>(&self, mut stream: S) -> anyhow::Result<()> {
        stream.write_u8(0).await?;
        stream.write_u8(self.r).await?;
        stream.write_u8(self.g).await?;
        stream.write_u8(self.b).await?;

        Ok(())
    }
}
