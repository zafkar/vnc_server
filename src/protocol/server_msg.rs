use bytes::Bytes;
use tokio::io::AsyncWriteExt;

use crate::protocol::{
    SendInto,
    encodings::EncodingType,
    primitives::{ColorMapEntry, Rect},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    FramebufferUpdate(Vec<UpdateRect>),
    SetColorMapEntries {
        first_color: u16,
        colors: Vec<ColorMapEntry>,
    },
    Bell,
    ServerCutText(String),
}

impl ServerMessage {
    fn get_type(&self) -> u8 {
        match self {
            ServerMessage::FramebufferUpdate(..) => 0,
            ServerMessage::SetColorMapEntries { .. } => 1,
            ServerMessage::Bell => 2,
            ServerMessage::ServerCutText(..) => 3,
        }
    }
}

impl SendInto for ServerMessage {
    async fn send<S: tokio::io::AsyncWrite + Unpin>(&self, mut stream: S) -> anyhow::Result<()> {
        stream.write_u8(self.get_type()).await?;
        match self {
            ServerMessage::FramebufferUpdate(update_rects) => {
                stream.write_all(&[0u8; 1]).await?;
                stream.write_u16(update_rects.len() as u16).await?;
                for rect in update_rects.iter() {
                    rect.send(&mut stream).await?;
                }
            }
            ServerMessage::SetColorMapEntries {
                first_color,
                colors,
            } => {
                stream.write_all(&[0u8; 1]).await?;
                stream.write_u16(*first_color).await?;
                stream.write_u16(colors.len() as u16).await?;
                for color in colors.iter() {
                    color.send(&mut stream).await?;
                }
            }
            ServerMessage::Bell => (),
            ServerMessage::ServerCutText(text) => {
                stream.write_all(&[0u8; 3]).await?;
                let str_bytes = text.as_bytes();
                stream.write_u32(str_bytes.len() as u32).await?;
                stream.write_all(str_bytes).await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateRect {
    pub rect: Rect,
    pub encoding_type: EncodingType,
    pub data: Bytes,
}

impl SendInto for UpdateRect {
    async fn send<S: tokio::io::AsyncWrite + Unpin>(&self, mut stream: S) -> anyhow::Result<()> {
        self.rect.send(&mut stream).await?;
        stream.write_i32(self.encoding_type.into()).await?;
        stream.write_all(&self.data).await?;

        Ok(())
    }
}
