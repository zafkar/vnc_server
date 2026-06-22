use crate::protocol::{pixel_format::PixelFormat, primitives::Rect};

#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub data: Vec<u8>,
    pub format: PixelFormat,
}

impl Frame {
    pub fn get_src_rect(&self, rect: Rect, height: usize) -> Vec<u8> {
        let stride = self.data.len() / height;
        let mut result = vec![];
        for y in rect.y_pos..rect.height {
            for x in rect.x_pos..rect.width {
                let i = stride * y as usize + 4 * x as usize;
                result.extend_from_slice(&self.data[i..i + 4]);
            }
        }
        result
    }
}

impl From<Frame> for Vec<u8> {
    fn from(value: Frame) -> Self {
        value.data
    }
}
