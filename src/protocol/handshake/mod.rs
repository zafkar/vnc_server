use anyhow::Result;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub mod init;
pub mod security;
pub mod version;

pub async fn write_handshake_error<S: AsyncWrite + Unpin>(mut stream: S, msg: &str) -> Result<()> {
    let bytes = msg.bytes();

    stream.write_u32(bytes.len() as u32).await?;
    Ok(stream.write_all(&bytes.collect::<Vec<_>>()).await?)
}
