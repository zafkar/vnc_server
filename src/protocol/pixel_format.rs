use anyhow::{Result, anyhow};
use num_enum::{FromPrimitive, IntoPrimitive};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::{RecvFrom, SendInto, primitives::Flag};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PixelFormat {
    pub bits_per_pixel: BitsPerPixel,
    pub depth: u8,
    pub big_endian: Flag,
    pub true_color: Flag,
    pub red_max: u16,
    pub green_max: u16,
    pub blue_max: u16,
    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
}

impl PixelFormat {
    pub fn convert_data_to_pixel_format(
        &self,
        target_format: &PixelFormat,
        data: &[u8],
    ) -> Result<Vec<u8>> {
        if self.true_color == Flag::No || target_format.true_color == Flag::No {
            return Err(anyhow!(
                "Conversion from true_color to palette not implemented"
            ));
        }

        let grouped_data = match self.bits_per_pixel {
            BitsPerPixel::U8 => data
                .iter()
                .map(|b| Ok(*b as u32))
                .collect::<Result<Vec<u32>>>(),
            BitsPerPixel::U16 => data
                .chunks(2)
                .map(|group| match self.big_endian {
                    Flag::No => Ok(u16::from_le_bytes(group.try_into()?) as u32),
                    Flag::Yes => Ok(u16::from_be_bytes(group.try_into()?) as u32),
                })
                .collect::<Result<Vec<u32>>>(),
            BitsPerPixel::U32 => data
                .chunks(4)
                .map(|group| match self.big_endian {
                    Flag::No => Ok(u32::from_le_bytes(group.try_into()?)),
                    Flag::Yes => Ok(u32::from_be_bytes(group.try_into()?)),
                })
                .collect::<Result<Vec<u32>>>(),
            BitsPerPixel::Invalid => Err(anyhow!("Invalid encoding size")),
        }?;

        Ok(grouped_data
            .iter()
            .flat_map(|src_color| {
                let red = (src_color >> self.red_shift & self.red_max as u32)
                    * target_format.red_max as u32
                    / self.red_max as u32;
                let green = (src_color >> self.green_shift & self.green_max as u32)
                    * target_format.green_max as u32
                    / self.green_max as u32;
                let blue = (src_color >> self.blue_shift & self.blue_max as u32)
                    * target_format.blue_max as u32
                    / self.blue_max as u32;

                let dest_color = red << target_format.red_shift
                    | green << target_format.green_shift
                    | blue << target_format.blue_shift;

                match target_format.big_endian {
                    Flag::No => dest_color.to_le_bytes(),
                    Flag::Yes => dest_color.to_be_bytes(),
                }
                .into_iter()
            })
            .collect())
    }
}

#[cfg(feature = "encoding_zrle")]
impl From<PixelFormat> for rfb_encodings::PixelFormat {
    fn from(value: PixelFormat) -> Self {
        rfb_encodings::PixelFormat {
            bits_per_pixel: value.bits_per_pixel.into(),
            depth: value.depth,
            big_endian_flag: value.big_endian.into(),
            true_colour_flag: value.true_color.into(),
            red_max: value.red_max,
            green_max: value.green_max,
            blue_max: value.blue_max,
            red_shift: value.red_shift,
            green_shift: value.green_shift,
            blue_shift: value.blue_shift,
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
