use anyhow::Result;
use bytes::BytesMut;

use crate::protocol::{encodings::Encoder, pixel_format::PixelFormat, server_msg::UpdateRect};

pub struct RawEncoder {
    pub src_pixel_format: PixelFormat,
    pub dest_pixel_format: PixelFormat,
}

impl Encoder for RawEncoder {
    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::Raw
    }

    fn encode(
        &mut self,
        requested_rect: crate::protocol::primitives::Rect,
        mut data: BytesMut,
    ) -> Result<Vec<UpdateRect>> {
        self.src_pixel_format
            .convert_data_in_place(&self.dest_pixel_format, &mut data)?;
        Ok(vec![UpdateRect {
            rect: requested_rect,
            encoding_type: self.encoding_type(),
            data: data.freeze(),
        }])
    }

    fn set_pixel_format(&mut self, _format: crate::protocol::pixel_format::PixelFormat) {}
}
