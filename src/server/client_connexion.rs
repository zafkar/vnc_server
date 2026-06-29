use std::sync::Arc;

use crate::{
    auth_provider::AuthProvider,
    capture::Frame,
    input_controller::KeyEvent,
    protocol::{
        RecvFrom, SendInto,
        client_msg::{ClientMessage, MouseButtonMask},
        encodings::{Encoder, EncodingType, raw::RawEncoder},
        handshake::{
            init::{ClientInit, ServerInit},
            security::SecurityType,
            version::Version,
        },
        pixel_format::PixelFormat,
        primitives::{Flag, Pos},
        pseudo_encodings::cursor_alpha::AlphaCursorPseudoEncodings,
        server_msg::ServerMessage,
    },
};
use anyhow::{Result, anyhow};
use tokio::{net::TcpStream, spawn, sync};
use tracing::{debug, info};

#[cfg(feature = "management")]
use crate::mgmt_server::{ClientInfo, ClientStatus};

pub(super) struct ClientConnexion {
    pub width: u16,
    pub height: u16,
    pub pixel_format: PixelFormat,
    pub available_security: Vec<SecurityType>,
    pub receive_screen_frame: sync::watch::Receiver<Frame>,
    pub mouse_pos_sender: sync::watch::Sender<Pos>,
    pub mouse_buttons_sender: sync::mpsc::Sender<MouseButtonMask>,
    pub keyboard_sender: sync::mpsc::Sender<KeyEvent>,
    pub auth_provider: Arc<dyn AuthProvider>,
    #[cfg(feature = "management")]
    pub info: sync::watch::Sender<ClientInfo>,
}

impl ClientConnexion {
    pub async fn start(&mut self, mut stream: TcpStream) -> Result<()> {
        Version::default().send(&mut stream).await?;
        let requested_version = Version::recv(&mut stream).await?;
        debug!("Requested version is {requested_version:?}");

        self.available_security.send(&mut stream).await?;

        let requested_security = SecurityType::recv(&mut stream).await?;
        info!("Requested security is {requested_security:?}");
        #[cfg(feature = "management")]
        self.info
            .send_modify(|info| info.auth_type = Some(requested_security));

        let user_permissions = {
            let security_result = requested_security
                .check_password(&mut stream, self.auth_provider.clone())
                .await?;

            security_result.send(&mut stream).await?;
            if security_result.is_denied() {
                return Err(anyhow!("Authentication denied"));
            }
            security_result.get_permissions()
        };
        info!("Client connected with permissions {user_permissions:?}");
        #[cfg(feature = "management")]
        self.info.send_modify(|info| {
            info.permissions = Some(user_permissions);
            info.status = ClientStatus::Authorized;
        });

        let client_init = ClientInit::recv(&mut stream).await?;
        debug!("{client_init:?}");

        ServerInit {
            fb_width: self.width,
            fb_height: self.height,
            pixel_format: self.pixel_format,
            name: String::from("Test server"),
        }
        .send(&mut stream)
        .await?;
        #[cfg(feature = "management")]
        self.info.send_modify(|info| {
            info.pixel_format = Some(self.pixel_format);
            info.status = ClientStatus::Initialized;
        });

        let (mut read_stream, mut write_stream) = stream.into_split();
        let (sender_to_client, mut receiver_to_client) = sync::mpsc::channel::<ServerMessage>(128);
        spawn(async move {
            while let Some(message) = receiver_to_client.recv().await {
                message.send(&mut write_stream).await?;
            }

            Ok::<_, anyhow::Error>(())
        });

        let mut encoder: Box<dyn Encoder> = Box::new(RawEncoder);
        let mut prev_mouse_buttons = MouseButtonMask::default();
        let mut target_pixel_format = None;
        #[cfg(feature = "management")]
        self.info.send_modify(|info| {
            info.status = ClientStatus::Running;
        });

        while let Ok(client_msg) = ClientMessage::recv(&mut read_stream).await {
            match client_msg {
                ClientMessage::SetPixelFormat(pixel_format) => {
                    info!("Client asks for {pixel_format:?}");
                    target_pixel_format = Some(pixel_format);
                    info!("Setting pixel_format in encoding");
                    encoder.set_pixel_format(pixel_format);
                    #[cfg(feature = "management")]
                    self.info.send_modify(|info| {
                        info.pixel_format = Some(pixel_format);
                    });
                }
                ClientMessage::SetEncodings(items) => {
                    info!("Client propose {items:?} as encodings");
                    let encoding_type = EncodingType::pick_encoder(&items);
                    info!("{encoding_type:?} choosen");
                    encoder = encoding_type.init_encoder(
                        self.width,
                        self.height,
                        target_pixel_format.unwrap_or(self.pixel_format),
                    )?;
                    if items.contains(&EncodingType::CursorWithAlpha) {
                        sender_to_client
                            .send(AlphaCursorPseudoEncodings.get_message()?)
                            .await?;
                    }
                    #[cfg(feature = "management")]
                    self.info.send_modify(|info| {
                        info.encoding = Some(encoding_type);
                    });
                }
                ClientMessage::FramebufferUpdateRequest { incremental, rect } => {
                    debug!("Client asks for {rect:?}, incremental {incremental:?}");
                    if !user_permissions.view {
                        debug!("Client forbidden from view");
                        continue;
                    }
                    if self.receive_screen_frame.has_changed()? || incremental == Flag::No {
                        let data = self.receive_screen_frame.borrow().clone();
                        self.receive_screen_frame.mark_unchanged();
                        let dest_pixel_format_data = match &target_pixel_format {
                            // Some(dest_format) if *dest_format == data.format => {
                            //     data.get_src_rect(rect, self.height as usize)
                            // }
                            Some(dest_format) => data.format.convert_data_to_pixel_format(
                                dest_format,
                                &data.get_src_rect(rect, self.height as usize),
                            )?,
                            None => data.get_src_rect(rect, self.height as usize),
                        };
                        sender_to_client
                            .send(ServerMessage::FramebufferUpdate(
                                encoder.encode(rect, &dest_pixel_format_data)?,
                            ))
                            .await?;
                    } else {
                        sender_to_client
                            .send(ServerMessage::FramebufferUpdate(vec![]))
                            .await?;
                    }
                }
                ClientMessage::KeyEvent { pressed, key } => {
                    debug!("Client send key {pressed:?}, {key:?}");
                    if !user_permissions.control {
                        debug!("Client forbidden from control");
                        continue;
                    }
                    self.keyboard_sender.send((pressed, key)).await?;
                }
                ClientMessage::PointerEvent { buttons, pos } => {
                    debug!("Client send mouse {buttons:?}, {pos:?}");
                    if !user_permissions.control {
                        debug!("Client forbidden from control");
                        continue;
                    }
                    self.mouse_pos_sender.send_replace(pos);

                    if buttons != prev_mouse_buttons {
                        self.mouse_buttons_sender.send(buttons).await?;
                        prev_mouse_buttons = buttons;
                    }
                }
                ClientMessage::ClientCutText(text) => {
                    if !user_permissions.control {
                        debug!("Client forbidden from control");
                        continue;
                    }
                    debug!("Client send clipboard {text}");
                }
            }
        }

        #[cfg(feature = "management")]
        self.info.send_modify(|info| {
            info.status = ClientStatus::Dead;
        });

        Ok(())
    }
}
