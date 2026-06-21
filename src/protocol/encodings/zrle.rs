use anyhow::Result;

use crate::protocol::encodings::Encoder;

pub struct ZRLEEncoder {
    pub width: u16,
    pub height: u16,
    pub compressor: flate2::Compress,
    pub pixel_format: rfb_encodings::PixelFormat,
}

impl Encoder for ZRLEEncoder {
    fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(rfb_encodings::zrle::encode_zrle_persistent(
            data,
            self.width,
            self.height,
            &self.pixel_format,
            &mut self.compressor,
        )?)
    }

    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::ZRLE
    }
}
