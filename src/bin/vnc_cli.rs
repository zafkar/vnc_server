use std::time::Duration;

use anyhow::Result;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufStream},
    net::TcpStream,
    time::sleep,
};
use uuid::Uuid;
use vnc_server::mgmt_server::{ManagmentClientMessage, client::ClientInfo};

#[tokio::main]
async fn main() -> Result<()> {
    let stream = TcpStream::connect("127.0.0.1:5899").await?;

    let mut buffer = BufStream::new(stream);

    make_request(&mut buffer, &ManagmentClientMessage::ListAliveClients).await?;

    let client_list: Vec<VNCClient> = read_ron_line(&mut buffer).await?;
    let uuid = client_list[0].uuid;
    println!("Target uuid {uuid}");

    loop {
        make_request(&mut buffer, &ManagmentClientMessage::GetClient(uuid)).await?;
        let client: VNCClient = read_ron_line(&mut buffer).await?;

        println!("{client:#?}");
        sleep(Duration::from_secs_f32(0.5)).await;
    }
}

async fn make_request<RW: AsyncRead + AsyncWrite + Unpin>(
    buffer: &mut BufStream<RW>,
    message: &ManagmentClientMessage,
) -> Result<()> {
    buffer
        .write_all((ron::to_string(message)? + "\n").as_bytes())
        .await?;
    Ok(buffer.flush().await?)
}

async fn read_ron_line<T: serde::de::DeserializeOwned, RW: AsyncRead + AsyncWrite + Unpin>(
    buffer: &mut BufStream<RW>,
) -> Result<T> {
    let mut text = String::new();

    buffer.read_line(&mut text).await?;

    let result = ron::from_str(&text)?;
    Ok(result)
}

#[derive(Debug, Clone, serde::Deserialize)]
struct VNCClient {
    uuid: Uuid,
    #[allow(unused)]
    client_info: ClientInfo,
}
