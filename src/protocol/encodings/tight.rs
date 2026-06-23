use crate::protocol::{encodings::Encoder, primitives::Rect, server_msg::UpdateRect};

pub struct TightEncoder {
    pub width: u16,
    pub height: u16,
    pub compressor: rfb_encodings::tight::SimpleTightCompressor,
    pub pixel_format: rfb_encodings::PixelFormat,
    pub quality: u8,
    pub compression_level: u8,
}

impl Encoder for TightEncoder {
    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::Tight
    }

    fn encode(&mut self, _requested_rect: Rect, data: &[u8]) -> anyhow::Result<Vec<UpdateRect>> {
        let encoded_data = rfb_encodings::tight::encode_tight_rects(
            data,
            self.width,
            self.height,
            self.quality,
            self.compression_level,
            &self.pixel_format,
            &mut self.compressor,
        );

        Ok(encoded_data
            .iter()
            .map(|u| UpdateRect {
                rect: Rect {
                    x_pos: u.0,
                    y_pos: u.1,
                    width: u.2,
                    height: u.3,
                },
                encoding_type: self.encoding_type(),
                data: u.4.iter().cloned().collect(),
            })
            .collect())
    }
}
