use std::time::Duration;

use anyhow::{Result, anyhow};
use enigo::{Keyboard, Mouse};
use scrap::{Capturer, Display};
use tokio::{
    net::{TcpListener, TcpStream},
    select, spawn, sync,
    task::spawn_blocking,
    time::sleep,
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use vnc_server::{
    input_controller::{KeyEvent, enigo_controller_start},
    protocol::{
        RecvFrom, SendInto,
        client_msg::{ClientMessage, MouseButtonMask},
        handshake::{
            init::{ClientInit, ServerInit},
            security::{SecurityResult, SecurityType},
            version::Version,
        },
        pixel_format::PixelFormat,
        primitives::{Flag, Pos},
        server_msg::{ServerMessage, UpdateRect},
    },
};

pub type Frame = Vec<u8>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer())
        .init();

    info!("Application started");

    let (width, height) = {
        let display = match Display::primary() {
            Ok(d) => d,
            Err(err) => return Err(anyhow!("Can't get Display : {err}")),
        };
        (display.width(), display.height())
    };

    let (send_screen_frame, receive_screen_frame) = sync::watch::channel(Frame::default());
    spawn_blocking(|| capture(send_screen_frame));
    debug!("Display Capture started");

    let (mouse_pos_sender, mouse_pos_receiver) = sync::watch::channel(Pos::default());
    let (mouse_buttons_sender, mouse_buttons_receiver) = sync::mpsc::channel(128);
    let (keyboard_sender, keyboard_receiver) = sync::mpsc::channel(128);
    spawn(async {
        match enigo_controller_start(
            mouse_pos_receiver,
            mouse_buttons_receiver,
            keyboard_receiver,
        )
        .await
        {
            Ok(_) => warn!("Enigo controller closed"),
            Err(err) => error!("Enigo controller crashed with {err}"),
        }
    });

    let listener = TcpListener::bind("127.0.0.1:5900").await?;

    while let Ok((stream, addr)) = listener.accept().await {
        let receive_screen_frame = receive_screen_frame.clone();
        let mouse_pos_sender = mouse_pos_sender.clone();
        let mouse_buttons_sender = mouse_buttons_sender.clone();
        let keyboard_sender = keyboard_sender.clone();
        spawn(async move {
            match handle_connexion(
                stream,
                width as u16,
                height as u16,
                receive_screen_frame,
                mouse_pos_sender,
                mouse_buttons_sender,
                keyboard_sender,
            )
            .await
            {
                Ok(_) => info!("Client {addr:?} disconnected"),
                Err(err) => warn!("Client thread failed : {err}"),
            }
        });
    }

    Ok(())
}

fn capture(send_screen_frame: sync::watch::Sender<Frame>) -> Result<()> {
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

async fn handle_connexion(
    mut stream: TcpStream,
    width: u16,
    height: u16,
    receive_screen_frame: sync::watch::Receiver<Frame>,
    mouse_pos_sender: sync::watch::Sender<Pos>,
    mouse_buttons_sender: sync::mpsc::Sender<MouseButtonMask>,
    keyboard_sender: sync::mpsc::Sender<KeyEvent>,
) -> Result<()> {
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
        fb_width: width,
        fb_height: height,
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
                let data = receive_screen_frame.borrow().clone();
                let stride = data.len() / height as usize;
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
                keyboard_sender.send((pressed, key)).await?;
            }
            ClientMessage::PointerEvent { buttons, pos } => {
                debug!("Client send mouse {buttons:?}, {pos:?}");
                mouse_pos_sender.send_replace(pos);

                if buttons != prev_mouse_buttons {
                    mouse_buttons_sender.send(buttons).await?;
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
