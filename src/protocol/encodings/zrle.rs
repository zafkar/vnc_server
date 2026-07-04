use anyhow::Result;
use bytes::BytesMut;

use crate::protocol::{encodings::Encoder, pixel_format::PixelFormat};

pub struct ZRLEEncoder {
    pub compressor: flate2::Compress,
    pub src_pixel_format: PixelFormat,
    pub client_pixel_format: PixelFormat,
}

impl Encoder for ZRLEEncoder {
    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::ZRLE
    }

    fn encode(
        &mut self,
        requested_rect: crate::protocol::primitives::Rect,
        mut data: BytesMut,
    ) -> Result<Vec<crate::protocol::server_msg::UpdateRect>> {
        self.src_pixel_format
            .convert_data_in_place(&self.client_pixel_format, &mut data)?;
        let encoded_data = rfb_encodings::zrle::encode_zrle_persistent(
            &data,
            requested_rect.width,
            requested_rect.height,
            &self.client_pixel_format.into(),
            &mut self.compressor,
        )?;

        Ok(vec![crate::protocol::server_msg::UpdateRect {
            rect: requested_rect,
            encoding_type: self.encoding_type(),
            data: encoded_data.into(),
        }])
    }

    fn set_pixel_format(&mut self, format: crate::protocol::pixel_format::PixelFormat) {
        self.client_pixel_format = format.into();
    }
}
