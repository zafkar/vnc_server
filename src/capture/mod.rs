use std::time::Instant;

use anyhow::{Result, anyhow};
use bytes::{BufMut, BytesMut};
use scrap::Display;
use tokio::sync;
use tracing::debug;
use xxhash_rust::xxh3::xxh3_128;

use crate::{config::CaptureConfig, protocol::pixel_format::PixelFormat};

pub mod frame;

pub type Frame = frame::Frame;

pub struct Capturer {
    send_screen_frame: sync::watch::Sender<Frame>,
    config: CaptureConfig,
}

impl Capturer {
    pub fn new(config: CaptureConfig) -> (Self, sync::watch::Receiver<Frame>) {
        let (send_screen_frame, receive_screen_frame) = sync::watch::channel(Frame::default());
        (
            Self {
                send_screen_frame,
                config,
            },
            receive_screen_frame,
        )
    }
    pub fn start(&mut self) -> Result<()> {
        let display = match Display::primary() {
            Ok(d) => d,
            Err(err) => return Err(anyhow!("Can't get Display : {err}")),
        };

        debug!("Getting Capture for display");

        let mut recorder = scrap::Capturer::new(display)?;

        debug!("Starting capture loop");
        let mut prev_data_hash = 0;
        loop {
            let start_time = Instant::now();
            if self.send_screen_frame.receiver_count() > 1 {
                let frame = match recorder.frame() {
                    Ok(frame) => frame,
                    Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(self.config.time_between_frame);
                        continue;
                    }
                    Err(err) => return Err(err.into()),
                };
                let mut data = BytesMut::with_capacity(frame.len());
                data.put_slice(&frame);
                let data_hash = xxh3_128(&data);
                if prev_data_hash != data_hash {
                    self.send_screen_frame.send_replace(frame::Frame {
                        data,
                        format: self.get_pixel_format(),
                    });
                    prev_data_hash = data_hash
                }
            }
            let elasped_duration = start_time.elapsed();
            if elasped_duration < self.config.time_between_frame {
                std::thread::sleep(self.config.time_between_frame - elasped_duration);
            }
        }
    }

    pub fn get_screen_size() -> Result<(usize, usize)> {
        let display = match Display::primary() {
            Ok(d) => d,
            Err(err) => return Err(anyhow!("Can't get Display : {err}")),
        };
        Ok((display.width(), display.height()))
    }

    pub fn get_pixel_format(&self) -> PixelFormat {
        PixelFormat {
            bits_per_pixel: crate::protocol::pixel_format::BitsPerPixel::U32,
            depth: 24,
            big_endian: crate::protocol::primitives::Flag::No,
            true_color: crate::protocol::primitives::Flag::Yes,
            red_max: 255,
            green_max: 255,
            blue_max: 255,
            red_shift: 16,
            green_shift: 8,
            blue_shift: 0,
        }
    }
}
