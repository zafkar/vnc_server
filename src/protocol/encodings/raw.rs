use anyhow::Result;

use crate::protocol::encodings::Encoder;

pub struct RawEncoder;

impl Encoder for RawEncoder {
    fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn encoding_type(&self) -> super::EncodingType {
        super::EncodingType::Raw
    }
}
