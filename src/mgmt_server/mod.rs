use anyhow::{Context, Result};
use serde::{Serialize, Serializer};
use std::{collections::HashMap, net::SocketAddr};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream, tcp::OwnedWriteHalf},
    spawn, sync,
    task::AbortHandle,
};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    auth_provider::UserPermissions,
    config::ManagmentServerConfig,
    protocol::{
        encodings::EncodingType, handshake::security::SecurityType, pixel_format::PixelFormat,
    },
};

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
    ListClients,
    ListAliveClients,
    GetClient(Uuid),
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
        from_client_buffer = String::new();
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

    let ron_text = match client_message {
        ManagmentClientMessage::ListClients => {
            ron::to_string(&clients.borrow().values().collect::<Vec<_>>())?
        }
        ManagmentClientMessage::GetClient(uuid) => ron::to_string(
            clients
                .borrow()
                .get(&uuid)
                .context(format!("This UUID doesn't exists {uuid}"))?,
        )?,
        ManagmentClientMessage::ListAliveClients => ron::to_string(
            &clients
                .borrow()
                .values()
                .filter(|client| client.client_info.borrow().status != ClientStatus::Dead)
                .collect::<Vec<_>>(),
        )?,
    };
    write_stream.write_all(ron_text.as_bytes()).await?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Client {
    pub uuid: Uuid,
    #[serde(skip_serializing)]
    pub abort_handle: AbortHandle,
    #[serde(serialize_with = "serialize_watch_client")]
    pub client_info: sync::watch::Receiver<ClientInfo>,
}

fn serialize_watch_client<S>(
    watcher: &sync::watch::Receiver<ClientInfo>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    watcher.borrow().serialize(serializer)
}

impl Client {
    pub fn new(abort_handle: AbortHandle, client_info: sync::watch::Receiver<ClientInfo>) -> Self {
        Self {
            uuid: Uuid::now_v7(),
            abort_handle,
            client_info,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ClientInfo {
    pub status: ClientStatus,
    pub addr: SocketAddr,
    pub auth_type: Option<SecurityType>,
    pub permissions: Option<UserPermissions>,
    pub encoding: Option<EncodingType>,
    pub pixel_format: Option<PixelFormat>,
}

impl ClientInfo {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            status: ClientStatus::default(),
            addr,
            auth_type: None,
            permissions: None,
            encoding: None,
            pixel_format: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub enum ClientStatus {
    #[default]
    Starting,
    Authorized,
    Initialized,
    Running,
    Dead,
}
