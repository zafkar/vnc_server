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
        primitives::{Flag, Pos, Rect},
        pseudo_encodings::cursor_alpha::AlphaCursorPseudoEncodings,
        server_msg::ServerMessage,
    },
};
use anyhow::{Result, anyhow};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::TcpStream,
    select, spawn,
    sync::{self, Mutex},
    task::JoinSet,
    time::Instant,
};
use tracing::{debug, info, trace};

#[cfg(feature = "management")]
use crate::mgmt_server::client::{ClientInfo, ClientStatus};

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

        let (mut read_stream, write_stream) = stream.into_split();
        let (sender_drain_request, mut receiver_drain_request) =
            sync::mpsc::channel::<sync::oneshot::Sender<()>>(128);
        let (sender_to_client, mut receiver_to_client) = sync::mpsc::channel::<ServerMessage>(128);
        spawn(async move {
            let mut buffer = BufWriter::new(write_stream);
            loop {
                select! {
                    Some(drained_tx) = receiver_drain_request.recv() => {
                        while let Ok(_) = receiver_to_client.try_recv(){}
                        drained_tx.send(()).map_err(|_| anyhow!("Unwaited drained_tx"))?;
                    }

                    Some(message) = receiver_to_client.recv() => {
                        message.send(&mut buffer).await?;
                        buffer.flush().await?;
                    }

                    else => {
                        break;
                    }
                }
            }

            Ok::<_, anyhow::Error>(())
        });

        let mut encoder: Arc<Mutex<dyn Encoder>> = Arc::new(Mutex::new(RawEncoder {
            src_pixel_format: self.pixel_format,
            dest_pixel_format: self.pixel_format,
        }));
        let mut prev_mouse_buttons = MouseButtonMask::default();
        let mut target_pixel_format = None;
        #[cfg(feature = "management")]
        self.info.send_modify(|info| {
            info.status = ClientStatus::Running;
        });

        let mut encoding_joinset = JoinSet::new();

        while let Ok(client_msg) = ClientMessage::recv(&mut read_stream).await {
            match client_msg {
                ClientMessage::SetPixelFormat(pixel_format) => {
                    info!("Client asks for {pixel_format:?}");
                    encoding_joinset.abort_all();
                    let (wait_for_drain_tx, wait_for_drain_rx) = sync::oneshot::channel::<()>();
                    sender_drain_request.send(wait_for_drain_tx).await?;
                    wait_for_drain_rx.await?;
                    target_pixel_format = Some(pixel_format);
                    info!("Setting pixel_format in encoding");
                    encoder.lock().await.set_pixel_format(pixel_format);
                    #[cfg(feature = "management")]
                    self.info.send_modify(|info| {
                        info.pixel_format = Some(pixel_format);
                    });
                }
                ClientMessage::SetEncodings(items) => {
                    info!("Client propose {items:?} as encodings");
                    encoding_joinset.abort_all();
                    let (wait_for_drain_tx, wait_for_drain_rx) = sync::oneshot::channel::<()>();
                    sender_drain_request.send(wait_for_drain_tx).await?;
                    wait_for_drain_rx.await?;
                    let encoding_type = EncodingType::pick_encoder(&items);
                    info!("{encoding_type:?} choosen");
                    encoder = encoding_type.init_encoder(
                        self.width,
                        self.height,
                        self.pixel_format,
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
                    trace!("Client asks for {rect:?}, incremental {incremental:?}");
                    if !user_permissions.view {
                        trace!("Client forbidden from view");
                        continue;
                    }
                    encoding_joinset.spawn(send_framebuffer_update(
                        sender_to_client.clone(),
                        self.receive_screen_frame.clone(),
                        self.info.clone(),
                        rect,
                        incremental,
                        encoder.clone(),
                        self.height as usize,
                    ));
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

async fn send_framebuffer_update(
    sender_to_client: sync::mpsc::Sender<ServerMessage>,
    mut receive_screen_frame: sync::watch::Receiver<Frame>,
    info: sync::watch::Sender<ClientInfo>,
    rect: Rect,
    incremental: Flag,
    encoder: Arc<Mutex<dyn Encoder>>,
    height: usize,
) -> Result<()> {
    if receive_screen_frame.has_changed()? || incremental == Flag::No {
        let start_time = Instant::now();
        let data = receive_screen_frame.borrow().clone();
        receive_screen_frame.mark_unchanged();
        sender_to_client
            .send(ServerMessage::FramebufferUpdate(
                encoder
                    .lock()
                    .await
                    .encode(rect, data.get_src_rect(rect, height))?,
            ))
            .await?;
        info.send_modify(|info| {
            info.time_for_frame_stats
                .add(start_time.elapsed().as_secs_f32());
        });
    } else {
        sender_to_client
            .send(ServerMessage::FramebufferUpdate(vec![]))
            .await?;
    }

    Ok(())
}
