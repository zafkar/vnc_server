use crate::protocol::primitives::Rect;

#[derive(Debug, Clone, Default)]
pub struct Frame(pub Vec<u8>);

impl Frame {
    pub fn get_src_rect(&self, rect: Rect, height: usize) -> Vec<u8> {
        let stride = self.0.len() / height;
        let mut result = vec![];
        for y in rect.y_pos..rect.height {
            for x in rect.x_pos..rect.width {
                let i = stride * y as usize + 4 * x as usize;
                result.extend_from_slice(&self.0[i..i + 4]);
            }
        }
        result
    }
}

impl From<Frame> for Vec<u8> {
    fn from(value: Frame) -> Self {
        value.0
    }
}
