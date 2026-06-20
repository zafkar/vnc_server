use std::time::Duration;

use anyhow::Result;
use enigo::{Keyboard, Mouse};
use tokio::{select, sync, time::sleep};

use crate::{
    input_controller::keyboard::xkeysym_into_enigo,
    protocol::{
        client_msg::MouseButtonMask,
        primitives::{Flag, Pos},
    },
};

mod keyboard;

pub type KeyEvent = (Flag, xkeysym::Keysym);

pub struct ControllerChannels {
    pub mouse_pos_sender: sync::watch::Sender<Pos>,
    pub mouse_buttons_sender: sync::mpsc::Sender<MouseButtonMask>,
    pub keyboard_sender: sync::mpsc::Sender<KeyEvent>,
}

pub struct Controller {
    mouse_pos: sync::watch::Receiver<Pos>,
    mouse_buttons_receiver: sync::mpsc::Receiver<MouseButtonMask>,
    keyboard_receiver: sync::mpsc::Receiver<KeyEvent>,
}

impl Controller {
    pub fn new(channel_size: usize) -> (Self, ControllerChannels) {
        let (mouse_pos_sender, mouse_pos_receiver) = sync::watch::channel(Pos::default());
        let (mouse_buttons_sender, mouse_buttons_receiver) = sync::mpsc::channel(channel_size);
        let (keyboard_sender, keyboard_receiver) = sync::mpsc::channel(channel_size);

        (
            Self {
                mouse_pos: mouse_pos_receiver,
                mouse_buttons_receiver,
                keyboard_receiver,
            },
            ControllerChannels {
                mouse_pos_sender,
                mouse_buttons_sender,
                keyboard_sender,
            },
        )
    }

    pub async fn start(&mut self) -> Result<()> {
        let minimal_time_between_pos_update = Duration::from_millis(25);

        let mut enigo = enigo::Enigo::new(&enigo::Settings::default())?;

        loop {
            select! {
                _ =  self.mouse_pos.changed() => {
                    let Pos { x_pos, y_pos } = self.mouse_pos.borrow().clone();
                    enigo.move_mouse(x_pos as i32, y_pos as i32, enigo::Coordinate::Abs)?;
                    sleep(minimal_time_between_pos_update).await;
                }
                Some(mask) = self.mouse_buttons_receiver.recv() => {
                    for (button, direction) in mask.into_enigo() {
                        enigo.button(button, direction)?;
                    }
                }
                Some((flag, key)) = self.keyboard_receiver.recv() => {
                    enigo.key(xkeysym_into_enigo(key), flag.into())?;
                }
            }
        }
    }
}
