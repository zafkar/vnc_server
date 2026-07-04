use anyhow::{Result, anyhow};
use bytes::{Buf, BufMut, BytesMut};
use num_enum::{FromPrimitive, IntoPrimitive};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::{RecvFrom, SendInto, primitives::Flag};

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
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
    pub fn convert_data(&self, target_format: &PixelFormat, data: BytesMut) -> Result<BytesMut> {
        if self.true_color == Flag::No || target_format.true_color == Flag::No {
            return Err(anyhow!(
                "Conversion from true_color to palette not implemented"
            ));
        }

        if self == target_format {
            return Ok(data);
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
                let dest_color = self.convert_u32(target_format, *src_color);
                match target_format.bits_per_pixel {
                    BitsPerPixel::U8 => vec![dest_color as u8],
                    BitsPerPixel::U16 => match target_format.big_endian {
                        Flag::No => (dest_color as u16).to_le_bytes().to_vec(),
                        Flag::Yes => (dest_color as u16).to_be_bytes().to_vec(),
                    },
                    BitsPerPixel::U32 => match target_format.big_endian {
                        Flag::No => dest_color.to_le_bytes().to_vec(),
                        Flag::Yes => dest_color.to_be_bytes().to_vec(),
                    },
                    BitsPerPixel::Invalid => unimplemented!("Invalid BitsPerPixel in PixelFormat"),
                }
            })
            .collect())
    }

    /// Convert a normalized u32 color from self to target pixel format
    #[inline]
    fn convert_u32(&self, target_format: &PixelFormat, value: u32) -> u32 {
        let red = (value >> self.red_shift & self.red_max as u32) * target_format.red_max as u32
            / self.red_max as u32;
        let green = (value >> self.green_shift & self.green_max as u32)
            * target_format.green_max as u32
            / self.green_max as u32;
        let blue = (value >> self.blue_shift & self.blue_max as u32)
            * target_format.blue_max as u32
            / self.blue_max as u32;

        red << target_format.red_shift
            | green << target_format.green_shift
            | blue << target_format.blue_shift
    }

    pub fn convert_data_in_place(
        &self,
        target_format: &PixelFormat,
        data: &mut BytesMut,
    ) -> Result<()> {
        if self.true_color == Flag::No || target_format.true_color == Flag::No {
            return Err(anyhow!(
                "Conversion from true_color to palette not implemented"
            ));
        }

        if self == target_format {
            return Ok(());
        }

        if self.bits_per_pixel < target_format.bits_per_pixel {
            return Err(anyhow!(
                "Target format is wider than src, would overwrite read, cannot convert in place"
            ));
        }

        let step = self.bits_per_pixel.bytes_size();
        let dest_step = target_format.bits_per_pixel.bytes_size();
        let mut j = 0;
        for i in (0..data.len()).step_by(step) {
            let pixel_color = match self.bits_per_pixel {
                BitsPerPixel::U8 => data[i] as u32,
                BitsPerPixel::U16 => match self.big_endian {
                    Flag::No => (&data[i..]).get_u16_le() as u32,
                    Flag::Yes => (&data[i..]).get_u16() as u32,
                },
                BitsPerPixel::U32 => match self.big_endian {
                    Flag::No => (&data[i..]).get_u32_le(),
                    Flag::Yes => (&data[i..]).get_u32(),
                },
                BitsPerPixel::Invalid => return Err(anyhow!("Invalid src pixel_format")),
            };

            let dest_color = self.convert_u32(target_format, pixel_color);
            match target_format.bits_per_pixel {
                BitsPerPixel::U8 => (&mut data[j..]).put_u8(dest_color as u8),
                BitsPerPixel::U16 => match target_format.big_endian {
                    Flag::No => (&mut data[j..]).put_u16_le(dest_color as u16),
                    Flag::Yes => (&mut data[j..]).put_u16(dest_color as u16),
                },
                BitsPerPixel::U32 => match target_format.big_endian {
                    Flag::No => (&mut data[j..]).put_u32_le(dest_color),
                    Flag::Yes => (&mut data[j..]).put_u32(dest_color),
                },
                BitsPerPixel::Invalid => return Err(anyhow!("Invalid dest pixel_format")),
            }
            j += dest_step;
        }

        data.truncate(j);

        Ok(())
    }
}

#[cfg(any(feature = "encoding_zrle", feature = "encoding_tight"))]
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
        //Format that the scrap::Capturer outputs
        PixelFormat {
            bits_per_pixel: crate::protocol::pixel_format::BitsPerPixel::U32,
            depth: 24,
            big_endian: crate::protocol::primitives::Flag::No,
            true_color: crate::protocol::primitives::Flag::Yes,
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

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FromPrimitive,
    IntoPrimitive,
    serde::Deserialize,
    serde::Serialize,
)]
#[repr(u8)]
pub enum BitsPerPixel {
    U8 = 8,
    U16 = 16,
    U32 = 32,

    #[default]
    Invalid = 0xff,
}

impl BitsPerPixel {
    pub fn bytes_size(&self) -> usize {
        match self {
            BitsPerPixel::U8 => 1,
            BitsPerPixel::U16 => 2,
            BitsPerPixel::U32 => 4,
            BitsPerPixel::Invalid => unimplemented!("BitsPerPixel Invalid"),
        }
    }
}
