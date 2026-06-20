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

pub async fn enigo_controller_start(
    mut mouse_pos: sync::watch::Receiver<Pos>,
    mut mouse_buttons_receiver: sync::mpsc::Receiver<MouseButtonMask>,
    mut keyboard_receiver: sync::mpsc::Receiver<KeyEvent>,
) -> Result<()> {
    let minimal_time_between_pos_update = Duration::from_millis(25);

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default())?;

    loop {
        select! {
            _ =  mouse_pos.changed() => {
                let Pos { x_pos, y_pos } = mouse_pos.borrow().clone();
                enigo.move_mouse(x_pos as i32, y_pos as i32, enigo::Coordinate::Abs)?;
                sleep(minimal_time_between_pos_update).await;
            }
            Some(mask) = mouse_buttons_receiver.recv() => {
                for (button, direction) in mask.into_enigo() {
                    enigo.button(button, direction)?;
                }
            }
            Some((flag, key)) = keyboard_receiver.recv() => {
                enigo.key(xkeysym_into_enigo(key), flag.into())?;
            }
        }
    }
}
