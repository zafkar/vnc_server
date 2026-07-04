use crate::protocol::{
    encodings::Encoder,
    pixel_format::{BitsPerPixel, PixelFormat},
    primitives::{Flag, Rect},
    server_msg::UpdateRect,
};

pub struct TightEncoder {
    pub width: u16,
    pub height: u16,
    pub compressor: rfb_encodings::tight::SimpleTightCompressor,
    pub src_pixel_format: PixelFormat,
    pub client_pixel_format: rfb_encodings::PixelFormat,
    pub quality: u8,
    pub compression_level: u8,
}

impl Encoder for TightEncoder {
    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::Tight
    }

    fn encode(&mut self, _requested_rect: Rect, data: &[u8]) -> anyhow::Result<Vec<UpdateRect>> {
        let data_in_encoder_format = self.src_pixel_format.convert_data_to_pixel_format(
            &PixelFormat {
                bits_per_pixel: BitsPerPixel::U32,
                depth: 24,
                big_endian: Flag::Yes,
                true_color: Flag::Yes,
                red_max: 255,
                green_max: 255,
                blue_max: 255,
                red_shift: 24,
                green_shift: 16,
                blue_shift: 8,
            },
            data,
        )?;
        let encoded_data = rfb_encodings::tight::encode_tight_rects(
            &data_in_encoder_format,
            self.width,
            self.height,
            self.quality,
            self.compression_level,
            &self.client_pixel_format,
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

    fn set_pixel_format(&mut self, format: crate::protocol::pixel_format::PixelFormat) {
        self.client_pixel_format = format.into();
    }
}
