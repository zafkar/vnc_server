use anyhow::Result;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use vnc_server::protocol::{
    RecvFrom, SendInto,
    client_msg::ClientMessage,
    handshake::{
        init::{ClientInit, ServerInit},
        security::{SecurityResult, SecurityType},
        version::Version,
    },
    pixel_format::PixelFormat,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer())
        .init();

    info!("Application started");

    let listener = TcpListener::bind("127.0.0.1:5900").await?;

    while let Ok((stream, _addr)) = listener.accept().await {
        handle_connexion(stream).await?;
    }

    Ok(())
}

async fn handle_connexion(mut stream: TcpStream) -> Result<()> {
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
        fb_width: 640,
        fb_height: 480,
        pixel_format: PixelFormat::default(),
        name: String::from("Test server"),
    }
    .send(&mut stream)
    .await?;

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
            }
            ClientMessage::KeyEvent { pressed, key } => {
                debug!("Client send key {pressed:?}, {key:?}");
            }
            ClientMessage::PointerEvent { buttons, pos } => {
                debug!("Client send mouse {buttons:?}, {pos:?}");
            }
            ClientMessage::ClientCutText(text) => {
                debug!("Client send clipboard {text}");
            }
        }
    }

    Ok(())
}
