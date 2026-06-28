use anyhow::{Result, anyhow};

use num_enum::{FromPrimitive, IntoPrimitive};
use tracing::warn;

use crate::protocol::{
    encodings::raw::RawEncoder, pixel_format::PixelFormat, primitives::Rect, server_msg::UpdateRect,
};

pub mod raw;

#[cfg(feature = "encoding_tight")]
pub mod tight;
#[cfg(feature = "encoding_zrle")]
pub mod zrle;
#[cfg(feature = "encoding_zrle")]
use crate::protocol::encodings::zrle::ZRLEEncoder;
#[cfg(feature = "encoding_zrle")]
use flate2::{Compress, Compression};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive, serde::Serialize)]
#[repr(i32)]
pub enum EncodingType {
    #[default]
    Raw = 0,
    // CopyRect = 1,
    // RRE = 2,
    // CoRRE = 4,
    // Hextile = 5,
    // Zlib = 6,
    #[cfg(feature = "encoding_tight")]
    Tight = 7,
    // ZLibHex = 8,
    #[cfg(feature = "encoding_zrle")]
    ZRLE = 16,
    // JPEG = 21,
    // OpenH264 = 50,
    // TightPNG = -260,
    CursorWithAlpha = -314,
}

impl EncodingType {
    pub fn is_pseudo(&self) -> bool {
        i32::from(*self) < 0i32
    }

    pub fn init_encoder(
        &self,
        #[allow(unused)] width: u16,
        #[allow(unused)] height: u16,
        #[allow(unused)] pixel_format: PixelFormat,
    ) -> Result<Box<dyn Encoder>> {
        match self {
            EncodingType::Raw => Ok(Box::new(RawEncoder)),
            #[cfg(feature = "encoding_zrle")]
            EncodingType::ZRLE => Ok(Box::new(ZRLEEncoder {
                width,
                height,
                compressor: Compress::new(Compression::fast(), true),
                pixel_format: pixel_format.into(),
            })),
            #[cfg(feature = "encoding_tight")]
            EncodingType::Tight => {
                let compression_level = 6;
                Ok(Box::new(crate::protocol::encodings::tight::TightEncoder {
                    width,
                    height,
                    compressor: rfb_encodings::tight::SimpleTightCompressor::new(compression_level),
                    pixel_format: pixel_format.into(),
                    quality: 6,
                    compression_level,
                }))
            }
            _ => Err(anyhow!("Not an encoding")),
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
    fn encode(&mut self, requested_rect: Rect, data: &[u8]) -> Result<Vec<UpdateRect>>;
    fn encoding_type(&self) -> EncodingType;
}
