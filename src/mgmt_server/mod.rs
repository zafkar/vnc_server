use anyhow::Result;
use std::{collections::HashMap, net::SocketAddr};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream, tcp::OwnedWriteHalf},
    spawn, sync,
    task::AbortHandle,
};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::ManagmentServerConfig;

type ClientsHandle = sync::watch::Receiver<HashMap<uuid::Uuid, Client>>;

#[derive(Debug)]
pub struct ManagmentServer {
    config: ManagmentServerConfig,
    clients: ClientsHandle,
}

impl ManagmentServer {
    pub fn new(config: ManagmentServerConfig, clients: ClientsHandle) -> Self {
        Self { config, clients }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Management started");
        let listener = TcpListener::bind(self.config.bind_address.clone()).await?;

        while let Ok((client_stream, client_addr)) = listener.accept().await {
            info!("New management connexion from {client_addr}");
            spawn(handle_client(client_stream, self.clients.clone()));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
enum ManagmentClientMessage {
    ListClient,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct ManagementClientError(String);

impl From<anyhow::Error> for ManagementClientError {
    fn from(value: anyhow::Error) -> Self {
        ManagementClientError(value.to_string())
    }
}

async fn handle_client(client_stream: TcpStream, clients: ClientsHandle) -> Result<()> {
    let (read_stream, mut write_stream) = client_stream.into_split();

    let mut reader = BufReader::new(read_stream);

    let mut from_client_buffer = String::new();
    while let Ok(read_len) = reader.read_line(&mut from_client_buffer).await
        && read_len != 0
    {
        match process_request(&from_client_buffer, &mut write_stream, &clients).await {
            Ok(_) => (),
            Err(err) => {
                warn!("Client thread for management encountered an error : {err}");
                let ron_text = ron::to_string(&ManagementClientError::from(err))?;
                write_stream.write_all(ron_text.as_bytes()).await?;
            }
        }
    }
    Ok(())
}

async fn process_request(
    request: &str,
    write_stream: &mut OwnedWriteHalf,
    clients: &ClientsHandle,
) -> Result<()> {
    let client_message = ron::from_str::<ManagmentClientMessage>(request)?;
    debug!("Management received from client : {client_message:?}");

    match client_message {
        ManagmentClientMessage::ListClient => {
            let ron_text = ron::to_string(&clients.borrow().values().collect::<Vec<_>>())?;
            write_stream.write_all(ron_text.as_bytes()).await?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Client {
    pub uuid: Uuid,
    pub addr: SocketAddr,
    #[serde(skip_serializing)]
    pub abort_handle: AbortHandle,
}

impl Client {
    pub fn new(addr: SocketAddr, abort_handle: AbortHandle) -> Self {
        Self {
            uuid: Uuid::now_v7(),
            addr,
            abort_handle,
        }
    }
}
