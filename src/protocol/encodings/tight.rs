use bytes::BytesMut;

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

    fn encode(
        &mut self,
        _requested_rect: Rect,
        mut data: BytesMut,
    ) -> anyhow::Result<Vec<UpdateRect>> {
        self.src_pixel_format.convert_data_in_place(
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
            &mut data,
        )?;
        let encoded_data = rfb_encodings::tight::encode_tight_rects(
            &data,
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
                data: u.4.clone().freeze(),
            })
            .collect())
    }

    fn set_pixel_format(&mut self, format: crate::protocol::pixel_format::PixelFormat) {
        self.client_pixel_format = format.into();
    }
}
