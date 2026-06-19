use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::protocol::{RecvFrom, SendInto};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version(u8, u8);

impl Default for Version {
    fn default() -> Self {
        Self(3, 8)
    }
}

impl SendInto for Version {
    async fn send<S: AsyncWrite + Unpin>(&self, mut stream: S) -> Result<()> {
        let text = format!("RFB {:0>3}.{:0>3}\n", self.0, self.1);
        Ok(stream.write_all(text.as_bytes()).await?)
    }
}

impl RecvFrom for Version {
    async fn recv<S: tokio::io::AsyncRead + Unpin>(mut stream: S) -> Result<Self> {
        let mut version_bytes = [0u8; 12];
        stream.read_exact(&mut version_bytes).await?;

        let major = String::from_utf8_lossy(&version_bytes[4..7]).parse()?;
        let minor = String::from_utf8_lossy(&version_bytes[8..11]).parse()?;

        Ok(Version(minor, major))
    }
}
