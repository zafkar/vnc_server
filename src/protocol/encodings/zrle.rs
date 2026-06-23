use anyhow::Result;

use crate::protocol::encodings::Encoder;

pub struct ZRLEEncoder {
    pub width: u16,
    pub height: u16,
    pub compressor: flate2::Compress,
    pub pixel_format: rfb_encodings::PixelFormat,
}

impl Encoder for ZRLEEncoder {
    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::ZRLE
    }

    fn encode(
        &mut self,
        requested_rect: crate::protocol::primitives::Rect,
        data: &[u8],
    ) -> Result<Vec<crate::protocol::server_msg::UpdateRect>> {
        let encoded_data = rfb_encodings::zrle::encode_zrle_persistent(
            data,
            self.width,
            self.height,
            &self.pixel_format,
            &mut self.compressor,
        )?;

        Ok(vec![crate::protocol::server_msg::UpdateRect {
            rect: requested_rect,
            encoding_type: self.encoding_type(),
            data: encoded_data,
        }])
    }
}
