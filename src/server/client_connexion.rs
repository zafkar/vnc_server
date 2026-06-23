use crate::{
    capture::Frame,
    input_controller::KeyEvent,
    protocol::{
        RecvFrom, SendInto,
        client_msg::{ClientMessage, MouseButtonMask},
        encodings::{Encoder, EncodingType, raw::RawEncoder},
        handshake::{
            init::{ClientInit, ServerInit},
            security::{SecurityResult, SecurityType},
            version::Version,
            write_handshake_error,
        },
        pixel_format::PixelFormat,
        primitives::{Flag, Pos},
        server_msg::ServerMessage,
    },
};
use anyhow::Result;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync,
};
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub(super) struct ClientConnexion {
    pub width: u16,
    pub height: u16,
    pub pixel_format: PixelFormat,
    pub receive_screen_frame: sync::watch::Receiver<Frame>,
    pub mouse_pos_sender: sync::watch::Sender<Pos>,
    pub mouse_buttons_sender: sync::mpsc::Sender<MouseButtonMask>,
    pub keyboard_sender: sync::mpsc::Sender<KeyEvent>,
}

impl ClientConnexion {
    pub async fn start<S: AsyncRead + AsyncWrite + Unpin>(&mut self, mut stream: S) -> Result<()> {
        Version::default().send(&mut stream).await?;
        let requested_version = Version::recv(&mut stream).await?;
        debug!("Requested version is {requested_version:?}");

        let available_security = vec![SecurityType::None];
        available_security.send(&mut stream).await?;

        let requested_security = SecurityType::recv(&mut stream).await?;
        info!("Requested security is {requested_security:?}");
        match requested_security
            .check_password(&mut stream, "password")
            .await
        {
            Ok(true) => SecurityResult::Ok.send(&mut stream).await?,
            Ok(false) => {
                SecurityResult::Failed.send(&mut stream).await?;
                write_handshake_error(&mut stream, "Wrong password").await?;
                warn!("Client failed to authenticate");
                return Ok(());
            }
            Err(err) => {
                SecurityResult::Failed.send(&mut stream).await?;
                write_handshake_error(&mut stream, &format!("Authentication failed : {err}"))
                    .await?;
                error!("Authentication failed : {err}");
                return Ok(());
            }
        }

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

        let mut encoder: Box<dyn Encoder> = Box::new(RawEncoder);
        let mut prev_mouse_buttons = MouseButtonMask::default();
        let mut target_pixel_format = None;

        while let Ok(client_msg) = ClientMessage::recv(&mut stream).await {
            match client_msg {
                ClientMessage::SetPixelFormat(pixel_format) => {
                    info!("Client asks for {pixel_format:?}");
                    target_pixel_format = Some(pixel_format);
                    info!("Reinitializing encoder");
                    encoder =
                        encoder
                            .encoding_type()
                            .init_encoder(self.width, self.height, pixel_format);
                }
                ClientMessage::SetEncodings(items) => {
                    info!("Client propose {items:?} as encodings");
                    let encoding_type = EncodingType::pick_encoder(&items);
                    info!("{encoding_type:?} choosen");
                    encoder = encoding_type.init_encoder(
                        self.width,
                        self.height,
                        target_pixel_format.unwrap_or(self.pixel_format),
                    );
                }
                ClientMessage::FramebufferUpdateRequest { incremental, rect } => {
                    debug!("Client asks for {rect:?}, incremental {incremental:?}");
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
                        ServerMessage::FramebufferUpdate(
                            encoder.encode(rect, &dest_pixel_format_data)?,
                        )
                        .send(&mut stream)
                        .await?;
                    } else {
                        ServerMessage::FramebufferUpdate(vec![])
                            .send(&mut stream)
                            .await?;
                    }
                }
                ClientMessage::KeyEvent { pressed, key } => {
                    debug!("Client send key {pressed:?}, {key:?}");
                    self.keyboard_sender.send((pressed, key)).await?;
                }
                ClientMessage::PointerEvent { buttons, pos } => {
                    debug!("Client send mouse {buttons:?}, {pos:?}");
                    self.mouse_pos_sender.send_replace(pos);

                    if buttons != prev_mouse_buttons {
                        self.mouse_buttons_sender.send(buttons).await?;
                        prev_mouse_buttons = buttons;
                    }
                }
                ClientMessage::ClientCutText(text) => {
                    debug!("Client send clipboard {text}");
                }
            }
        }

        Ok(())
    }
}
