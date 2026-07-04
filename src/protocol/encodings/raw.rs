use anyhow::Result;

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
        data: &[u8],
    ) -> Result<Vec<UpdateRect>> {
        let dest_format_pixel_data = self
            .src_pixel_format
            .convert_data_to_pixel_format(&self.dest_pixel_format, data)?;
        Ok(vec![UpdateRect {
            rect: requested_rect,
            encoding_type: self.encoding_type(),
            data: dest_format_pixel_data,
        }])
    }

    fn set_pixel_format(&mut self, _format: crate::protocol::pixel_format::PixelFormat) {}
}
