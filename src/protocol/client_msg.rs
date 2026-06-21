use tokio::io::AsyncReadExt;
use xkeysym::Keysym;

use crate::protocol::{
    RecvFrom,
    encodings::EncodingType,
    pixel_format::PixelFormat,
    primitives::{Flag, Pos, Rect},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMessage {
    SetPixelFormat(PixelFormat),
    SetEncodings(Vec<EncodingType>),
    FramebufferUpdateRequest { incremental: Flag, rect: Rect },
    KeyEvent { pressed: Flag, key: Keysym },
    PointerEvent { buttons: MouseButtonMask, pos: Pos },
    ClientCutText(String),
}

impl RecvFrom for ClientMessage {
    async fn recv<S: tokio::io::AsyncRead + Unpin>(mut stream: S) -> anyhow::Result<Self> {
        let msg = match stream.read_u8().await? {
            0 => {
                let mut buf = [0u8; 3];
                stream.read_exact(&mut buf).await?;
                Self::SetPixelFormat(PixelFormat::recv(&mut stream).await?)
            }
            2 => {
                stream.read_u8().await?;
                let num_encodings = stream.read_u16().await?;
                let mut encodings = vec![];
                for _ in 0..num_encodings {
                    encodings.push(stream.read_i32().await?.into());
                }
                Self::SetEncodings(encodings)
            }
            3 => {
                let incremental = stream.read_u8().await?.into();
                let rect = Rect::recv(&mut stream).await?;
                Self::FramebufferUpdateRequest { incremental, rect }
            }
            4 => {
                let pressed = stream.read_u8().await?.into();
                let mut buf = [0u8; 2];
                stream.read_exact(&mut buf).await?;
                let key = stream.read_u32().await?.into();
                Self::KeyEvent { pressed, key }
            }
            5 => {
                let buttons = stream.read_u8().await?.into();
                let pos = Pos::recv(&mut stream).await?;
                Self::PointerEvent { buttons, pos }
            }
            6 => {
                let mut trash_buf = [0u8; 3];
                stream.read_exact(&mut trash_buf).await?;

                let len = stream.read_u32().await?;
                let text = vec![0u8; len as usize];

                //Todo: decode with ISO 8859-1 (Latin-1) instead
                Self::ClientCutText(String::from_utf8_lossy(&text).to_string())
            }
            msg => unimplemented!("Client message {msg}"),
        };

        Ok(msg)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct MouseButtonMask(u8);

impl From<u8> for MouseButtonMask {
    fn from(value: u8) -> Self {
        MouseButtonMask(value)
    }
}

impl MouseButtonMask {
    pub fn into_enigo(&self) -> Vec<(enigo::Button, enigo::Direction)> {
        vec![
            (enigo::Button::Left, bit_to_dir(self.0, 0)),
            (enigo::Button::Middle, bit_to_dir(self.0, 1)),
            (enigo::Button::Right, bit_to_dir(self.0, 2)),
            (enigo::Button::ScrollUp, bit_to_dir(self.0, 3)),
            (enigo::Button::ScrollDown, bit_to_dir(self.0, 4)),
        ]
    }
}

fn bit_to_dir(mask: u8, shift: u8) -> enigo::Direction {
    if (mask >> shift & 1) > 0 {
        enigo::Direction::Press
    } else {
        enigo::Direction::Release
    }
}
