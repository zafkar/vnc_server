use anyhow::Result;

use crate::protocol::encodings::Encoder;

pub struct ZRLEEncoder {
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
            requested_rect.width,
            requested_rect.height,
            &self.pixel_format,
            &mut self.compressor,
        )?;

        Ok(vec![crate::protocol::server_msg::UpdateRect {
            rect: requested_rect,
            encoding_type: self.encoding_type(),
            data: encoded_data,
        }])
    }

    fn set_pixel_format(&mut self, format: crate::protocol::pixel_format::PixelFormat) {
        self.pixel_format = format.into();
    }
}
