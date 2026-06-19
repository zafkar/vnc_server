use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};

pub mod client_msg;
pub mod handshake;
pub mod pixel_format;
pub mod primitives;
pub mod server_msg;

pub trait SendInto: Sized {
    #[allow(async_fn_in_trait)]
    async fn send<S: AsyncWrite + Unpin>(&self, stream: S) -> Result<()>;
}

pub trait RecvFrom: Sized {
    #[allow(async_fn_in_trait)]
    async fn recv<S: AsyncRead + Unpin>(stream: S) -> Result<Self>;
}
