use bytes::BytesMut;

use crate::protocol::{pixel_format::PixelFormat, primitives::Rect};

#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub data: BytesMut,
    pub format: PixelFormat,
}

impl Frame {
    pub fn get_src_rect(&self, rect: Rect, height: usize) -> BytesMut {
        let bpp = self.format.bits_per_pixel.bytes_size();
        let stride = self.data.len() / height;

        let start_x = rect.x_pos as usize * bpp;
        let row_size = rect.width as usize * bpp;

        let mut result = BytesMut::with_capacity(row_size * rect.height as usize);

        let mut y_index = stride * rect.y_pos as usize;

        for _ in 0..rect.height {
            let start = y_index + start_x;
            let end = start + row_size;

            if end <= self.data.len() {
                result.extend_from_slice(&self.data[start..end]);
            } else {
                result.extend(std::iter::repeat_n(0, row_size));
            }

            y_index += stride;
        }

        result
    }
}

impl From<Frame> for Vec<u8> {
    fn from(value: Frame) -> Self {
        value.data.to_vec()
    }
}
