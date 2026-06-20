use crate::{
    capture::Frame,
    input_controller::KeyEvent,
    protocol::{
        RecvFrom, SendInto,
        client_msg::{ClientMessage, MouseButtonMask},
        handshake::{
            init::{ClientInit, ServerInit},
            security::{SecurityResult, SecurityType},
            version::Version,
        },
        pixel_format::PixelFormat,
        primitives::Pos,
        server_msg::{ServerMessage, UpdateRect},
    },
};
use anyhow::Result;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync,
};
use tracing::debug;

#[derive(Debug)]
pub(super) struct ClientConnexion {
    pub width: u16,
    pub height: u16,
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
        debug!("Requested security is {requested_security:?}");

        SecurityResult::Ok.send(&mut stream).await?;

        let client_init = ClientInit::recv(&mut stream).await?;
        debug!("{client_init:?}");

        ServerInit {
            fb_width: self.width,
            fb_height: self.height,
            pixel_format: PixelFormat::default(),
            name: String::from("Test server"),
        }
        .send(&mut stream)
        .await?;

        let mut prev_mouse_buttons = MouseButtonMask::default();

        while let Ok(client_msg) = ClientMessage::recv(&mut stream).await {
            match client_msg {
                ClientMessage::SetPixelFormat(pixel_format) => {
                    debug!("Client asks for {pixel_format:?}");
                }
                ClientMessage::SetEncodings(items) => {
                    debug!("Client asks for {items:?}");
                }
                ClientMessage::FramebufferUpdateRequest {
                    incremental: _,
                    rect,
                } => {
                    debug!("Client asks for {rect:?}");
                    let data = self.receive_screen_frame.borrow().clone();
                    let stride = data.len() / self.height as usize;
                    let mut result = vec![];
                    for y in rect.y_pos..rect.height {
                        for x in rect.x_pos..rect.width {
                            let i = stride * y as usize + 4 * x as usize;
                            result.extend_from_slice(&data[i..i + 4]);
                        }
                    }

                    ServerMessage::FramebufferUpdate(vec![UpdateRect {
                        rect,
                        encoding_type: 0,
                        data,
                    }])
                    .send(&mut stream)
                    .await?;
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
