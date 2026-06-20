use std::time::Duration;

use anyhow::{Result, anyhow};
use scrap::{Capturer, Display};
use tokio::sync;

pub type Frame = Vec<u8>;

pub fn capture(send_screen_frame: sync::watch::Sender<Frame>) -> Result<()> {
    let display = match Display::primary() {
        Ok(d) => d,
        Err(err) => return Err(anyhow!("Can't get Display : {err}")),
    };

    let time_between_frame = Duration::from_millis(100);
    let mut recorder = Capturer::new(display)?;

    loop {
        if send_screen_frame.receiver_count() > 0 {
            let frame = recorder.frame()?;
            send_screen_frame.send_replace(frame.to_vec());
        }
        std::thread::sleep(time_between_frame);
    }
}
