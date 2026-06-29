use crate::protocol::{
    encodings::EncodingType,
    primitives::Rect,
    server_msg::{ServerMessage, UpdateRect},
};
use anyhow::{Context, Result};
use png::OutputInfo;
use std::io::BufReader;

pub struct AlphaCursorPseudoEncodings;

impl AlphaCursorPseudoEncodings {
    pub fn get_message(&self) -> anyhow::Result<ServerMessage> {
        let (_cursor_info, cursor_data) = load_png()?;
        let mut data = Vec::from_iter(i32::from(EncodingType::Raw).to_be_bytes().into_iter());
        data.extend_from_slice(&cursor_data);
        Ok(ServerMessage::FramebufferUpdate(vec![UpdateRect {
            rect: Rect {
                x_pos: 0,
                y_pos: 0,
                width: 32,
                height: 32,
            },
            encoding_type: EncodingType::CursorWithAlpha,
            data: data,
        }]))
    }
}

fn load_png() -> Result<(OutputInfo, Vec<u8>)> {
    let data = std::io::Cursor::new(include_bytes!("assets/cursor.png").to_vec());

    let decoder = png::Decoder::new(BufReader::new(data));
    let mut png_reader = decoder.read_info()?;
    let mut buf = vec![
        0;
        png_reader
            .output_buffer_size()
            .context("No PNG buffer size on cursor")?
    ];
    let info = png_reader.next_frame(&mut buf)?;

    Ok((info, buf))
}
