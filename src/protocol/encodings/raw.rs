use anyhow::Result;

use crate::protocol::{encodings::Encoder, server_msg::UpdateRect};

pub struct RawEncoder;

impl Encoder for RawEncoder {
    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::Raw
    }

    fn encode(
        &mut self,
        requested_rect: crate::protocol::primitives::Rect,
        data: &[u8],
    ) -> Result<Vec<UpdateRect>> {
        Ok(vec![UpdateRect {
            rect: requested_rect,
            encoding_type: self.encoding_type(),
            data: data.to_vec(),
        }])
    }

    fn set_pixel_format(&mut self, _format: crate::protocol::pixel_format::PixelFormat) {}
}
