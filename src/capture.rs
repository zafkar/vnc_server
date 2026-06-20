use std::time::Duration;

use anyhow::{Result, anyhow};
use scrap::Display;
use tokio::sync;

pub type Frame = Vec<u8>;

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

        loop {
            if self.send_screen_frame.receiver_count() > 0 {
                let frame = recorder.frame()?;
                self.send_screen_frame.send_replace(frame.to_vec());
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
