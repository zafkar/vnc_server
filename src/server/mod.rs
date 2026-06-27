use std::collections::HashMap;

use crate::{
    capture::Capturer,
    config::Config,
    input_controller::{Controller, ControllerChannels},
    mgmt_server::{Client, ManagmentServer},
    server::client_connexion::ClientConnexion,
};
use anyhow::Result;
use tokio::{net::TcpListener, spawn, sync, task::spawn_blocking};
use tracing::{error, info, warn};

mod client_connexion;

#[derive(Debug)]
pub struct VNCServer {
    pub config: Config,
}

impl Default for VNCServer {
    fn default() -> Self {
        Self {
            config: Config::default(),
        }
    }
}

impl VNCServer {
    pub async fn start(&mut self) -> Result<()> {
        info!("VNC Server started");

        let (width, height) = Capturer::get_screen_size()?;

        let (mut capturer, receive_screen_frame) = Capturer::new(self.config.capture.clone());
        spawn_blocking(move || match capturer.start() {
            Ok(_) => warn!("Capture thread closed"),
            Err(err) => error!("Capture thread crashed with {err}"),
        });
        info!("Display Capture started");

        let (
            mut controller,
            ControllerChannels {
                mouse_pos_sender,
                mouse_buttons_sender,
                keyboard_sender,
            },
        ) = Controller::new(self.config.server.channel_size);
        spawn(async move {
            match controller.start().await {
                Ok(_) => warn!("Enigo controller closed"),
                Err(err) => error!("Enigo controller crashed with {err}"),
            }
        });

        let auth_provider = self.config.auth_provider.init().await?;

        let (client_updater, client_watcher) = sync::watch::channel(HashMap::new());

        let mgmt_config = self.config.management.clone();
        spawn(async {
            if let Some(mgmt_config) = mgmt_config {
                let mut management_server =
                    ManagmentServer::new(mgmt_config.clone(), client_watcher);
                match management_server.start().await {
                    Ok(_) => warn!("Management server stopped"),
                    Err(err) => error!("Management server crashed : {err}"),
                }
            }
        });

        let listener = TcpListener::bind(self.config.server.bind_address.clone()).await?;

        while let Ok((stream, addr)) = listener.accept().await {
            let mut client_connexion = ClientConnexion {
                width: width as u16,
                height: height as u16,
                receive_screen_frame: receive_screen_frame.clone(),
                mouse_pos_sender: mouse_pos_sender.clone(),
                mouse_buttons_sender: mouse_buttons_sender.clone(),
                keyboard_sender: keyboard_sender.clone(),
                pixel_format: self.config.server.pixel_format,
                auth_provider: auth_provider.clone(),
                available_security: self.config.server.auth_protocols.clone(),
            };
            let handle = spawn(async move {
                match client_connexion.start(stream).await {
                    Ok(_) => info!("Client {addr:?} disconnected"),
                    Err(err) => warn!("Client thread for {addr:?} failed : {err}"),
                }
            });
            let client = Client::new(addr, handle.abort_handle());
            client_updater.send_modify(|clients| {
                clients.insert(client.uuid.clone(), client);
            });
        }

        Ok(())
    }
}
