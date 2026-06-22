use anyhow::Result;

use num_enum::{FromPrimitive, IntoPrimitive};
use tracing::warn;

use crate::protocol::{encodings::raw::RawEncoder, pixel_format::PixelFormat};

pub mod raw;

#[cfg(feature = "encoding_zrle")]
pub mod zrle;
#[cfg(feature = "encoding_zrle")]
use crate::protocol::encodings::zrle::ZRLEEncoder;
#[cfg(feature = "encoding_zrle")]
use flate2::{Compress, Compression};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(i32)]
pub enum EncodingType {
    #[default]
    Raw = 0,
    // CopyRect = 1,
    // RRE = 2,
    // CoRRE = 4,
    // Hextile = 5,
    // Zlib = 6,
    // Tight = 7,
    // ZLibHex = 8,
    #[cfg(feature = "encoding_zrle")]
    ZRLE = 16,
    // JPEG = 21,
    // OpenH264 = 50,
    // TightPNG = -260,
}

impl EncodingType {
    pub fn is_pseudo(&self) -> bool {
        i32::from(*self) < 0i32
    }

    pub fn init_encoder(
        &self,
        width: u16,
        height: u16,
        pixel_format: PixelFormat,
    ) -> Box<dyn Encoder> {
        match self {
            EncodingType::Raw => Box::new(RawEncoder),
            #[cfg(feature = "encoding_zrle")]
            EncodingType::ZRLE => Box::new(ZRLEEncoder {
                width,
                height,
                compressor: Compress::new(Compression::fast(), true),
                pixel_format: pixel_format.into(),
            }),
        }
    }

    pub fn pick_encoder(available_list: &[EncodingType]) -> EncodingType {
        for enc in available_list {
            if !enc.is_pseudo() && *enc != EncodingType::Raw {
                return *enc;
            }
        }
        warn!("Fallback to RawEncoder");
        EncodingType::Raw
    }
}

pub trait Encoder: Send {
    fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>>;
    fn encoding_type(&self) -> EncodingType;
}
