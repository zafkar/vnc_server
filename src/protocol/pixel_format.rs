use num_enum::{FromPrimitive, IntoPrimitive};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::{RecvFrom, SendInto, primitives::Flag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelFormat {
    bits_per_pixel: BitsPerPixel,
    depth: u8,
    big_endian: Flag,
    true_color: Flag,
    red_max: u16,
    green_max: u16,
    blue_max: u16,
    red_shift: u8,
    green_shift: u8,
    blue_shift: u8,
}

impl From<PixelFormat> for rfb_encodings::PixelFormat {
    fn from(value: PixelFormat) -> Self {
        rfb_encodings::PixelFormat {
            bits_per_pixel: value.bits_per_pixel.into(),
            depth: value.depth.into(),
            big_endian_flag: value.big_endian.flip().into(),
            true_colour_flag: value.true_color.into(),
            red_max: value.red_max.into(),
            green_max: value.green_max.into(),
            blue_max: value.blue_max.into(),
            red_shift: value.red_shift.into(),
            green_shift: value.green_shift.into(),
            blue_shift: value.blue_shift.into(),
        }
    }
}

impl Default for PixelFormat {
    fn default() -> Self {
        Self {
            bits_per_pixel: BitsPerPixel::U32,
            depth: 24,
            big_endian: Flag::Yes,
            true_color: Flag::Yes,
            red_max: 255,
            green_max: 255,
            blue_max: 255,
            red_shift: 16,
            green_shift: 8,
            blue_shift: 0,
        }
    }
}

impl SendInto for PixelFormat {
    async fn send<S: tokio::io::AsyncWrite + Unpin>(&self, mut stream: S) -> anyhow::Result<()> {
        stream.write_u8(self.bits_per_pixel.into()).await?;
        stream.write_u8(self.depth).await?;
        stream.write_u8(self.big_endian.into()).await?;
        stream.write_u8(self.true_color.into()).await?;
        stream.write_u16(self.red_max).await?;
        stream.write_u16(self.green_max).await?;
        stream.write_u16(self.blue_max).await?;
        stream.write_u8(self.red_shift).await?;
        stream.write_u8(self.green_shift).await?;
        stream.write_u8(self.blue_shift).await?;
        stream.write_all(&[0u8; 3]).await?;

        Ok(())
    }
}

impl RecvFrom for PixelFormat {
    async fn recv<S: tokio::io::AsyncRead + Unpin>(mut stream: S) -> anyhow::Result<Self> {
        let bits_per_pixel = stream.read_u8().await?.into();
        let depth = stream.read_u8().await?;
        let big_endian = stream.read_u8().await?.into();
        let true_color = stream.read_u8().await?.into();
        let red_max = stream.read_u16().await?;
        let green_max = stream.read_u16().await?;
        let blue_max = stream.read_u16().await?;
        let red_shift = stream.read_u8().await?;
        let green_shift = stream.read_u8().await?;
        let blue_shift = stream.read_u8().await?;

        let mut buf = [0u8; 3];
        stream.read_exact(&mut buf).await?;

        Ok(Self {
            bits_per_pixel,
            depth,
            big_endian,
            true_color,
            red_max,
            green_max,
            blue_max,
            red_shift,
            green_shift,
            blue_shift,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum BitsPerPixel {
    U8 = 8,
    U16 = 16,
    U32 = 32,

    #[default]
    Invalid = 0xff,
}
