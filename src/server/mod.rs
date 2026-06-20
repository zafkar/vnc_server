use std::time::Duration;

use crate::{
    capture::Capturer,
    input_controller::{Controller, ControllerChannels},
    server::client_connexion::ClientConnexion,
};
use anyhow::Result;
use tokio::{net::TcpListener, spawn, task::spawn_blocking};
use tracing::{debug, error, info, warn};

mod client_connexion;

#[derive(Debug)]
pub struct VNCServer {
    pub bind_address: String,
    pub channel_size: usize,
    pub time_between_frame: Duration,
}

impl Default for VNCServer {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:5900".to_string(),
            channel_size: 128,
            time_between_frame: Duration::from_millis(15),
        }
    }
}

impl VNCServer {
    pub async fn start(&mut self) -> Result<()> {
        info!("VNC Server started");

        let (width, height) = Capturer::get_screen_size()?;

        let (mut capturer, receive_screen_frame) = Capturer::new(self.time_between_frame);
        spawn_blocking(move || capturer.start());
        debug!("Display Capture started");

        let (
            mut controller,
            ControllerChannels {
                mouse_pos_sender,
                mouse_buttons_sender,
                keyboard_sender,
            },
        ) = Controller::new(self.channel_size);
        spawn(async move {
            match controller.start().await {
                Ok(_) => warn!("Enigo controller closed"),
                Err(err) => error!("Enigo controller crashed with {err}"),
            }
        });

        let listener = TcpListener::bind(self.bind_address.clone()).await?;

        while let Ok((stream, addr)) = listener.accept().await {
            let mut client = ClientConnexion {
                width: width as u16,
                height: height as u16,
                receive_screen_frame: receive_screen_frame.clone(),
                mouse_pos_sender: mouse_pos_sender.clone(),
                mouse_buttons_sender: mouse_buttons_sender.clone(),
                keyboard_sender: keyboard_sender.clone(),
            };
            spawn(async move {
                match client.start(stream).await {
                    Ok(_) => info!("Client {addr:?} disconnected"),
                    Err(err) => warn!("Client thread for {addr:?} failed : {err}"),
                }
            });
        }

        Ok(())
    }
}
