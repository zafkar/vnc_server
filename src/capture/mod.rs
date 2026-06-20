use std::time::Duration;

use anyhow::{Result, anyhow};
use scrap::Display;
use tokio::sync;
use xxhash_rust::xxh3::xxh3_128;

pub mod frame;

pub type Frame = frame::Frame;

pub struct Capturer {
    send_screen_frame: sync::watch::Sender<Frame>,
}

impl Capturer {
    pub fn new() -> (Self, sync::watch::Receiver<Frame>) {
        let (send_screen_frame, receive_screen_frame) = sync::watch::channel(Frame::default());
        (Self { send_screen_frame }, receive_screen_frame)
    }
    pub fn start(&mut self) -> Result<()> {
        let display = match Display::primary() {
            Ok(d) => d,
            Err(err) => return Err(anyhow!("Can't get Display : {err}")),
        };

        let time_between_frame = Duration::from_millis(100);
        let mut recorder = scrap::Capturer::new(display)?;

        let mut prev_data_hash = 0;
        loop {
            if self.send_screen_frame.receiver_count() > 0 {
                let frame = recorder.frame()?;
                let data = frame.to_vec();
                let data_hash = xxh3_128(&data);
                if prev_data_hash != data_hash {
                    self.send_screen_frame.send_replace(frame::Frame(data));
                    prev_data_hash = data_hash
                }
            }
            std::thread::sleep(time_between_frame);
        }
    }

    pub fn get_screen_size() -> Result<(usize, usize)> {
        let display = match Display::primary() {
            Ok(d) => d,
            Err(err) => return Err(anyhow!("Can't get Display : {err}")),
        };
        Ok((display.width(), display.height()))
    }
}
