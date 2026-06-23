use crate::protocol::encodings::Encoder;

pub struct TightEncoder {
    pub width: u16,
    pub height: u16,
    pub compressor: rfb_encodings::tight::SimpleTightCompressor,
    pub pixel_format: rfb_encodings::PixelFormat,
    pub quality: u8,
    pub compression_level: u8,
}

impl Encoder for TightEncoder {
    // fn encode(&mut self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
    //     let a = rfb_encodings::tight::encode_tight_rects(
    //         data,
    //         self.width,
    //         self.height,
    //         self.quality,
    //         self.compression_level,
    //         &self.pixel_format,
    //         &mut self.compressor,
    //     );
    //     let b = a[0].4.iter().cloned().collect::<Vec<u8>>();

    //     todo!()
    // }

    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::Tight
    }

    fn encode(
        &mut self,
        requested_rect: crate::protocol::primitives::Rect,
        data: &[u8],
    ) -> anyhow::Result<Vec<crate::protocol::server_msg::UpdateRect>> {
        let encoded_data = rfb_encodings::tight::encode_tight_rects(
            data,
            self.width,
            self.height,
            self.quality,
            self.compression_level,
            &self.pixel_format,
            &mut self.compressor,
        );

        Ok(vec![crate::protocol::server_msg::UpdateRect {
            rect: requested_rect,
            encoding_type: self.encoding_type(),
            data: encoded_data[0].4.iter().cloned().collect::<Vec<u8>>(),
        }])
    }
}
