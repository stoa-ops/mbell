use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

static SOCKET_PATH: OnceLock<PathBuf> = OnceLock::new();

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("Failed to create socket: {0}")]
    SocketError(#[from] std::io::Error),
    #[error("Failed to serialize message: {0}")]
    SerializeError(#[from] serde_json::Error),
    #[error("Daemon is not running")]
    DaemonNotRunning,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Command {
    Pause,
    Resume,
    Stop,
    Status,
    Ring,
    Reload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Response {
    Ok,
    Status(StatusInfo),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInfo {
    pub state: String,
    pub next_bell_secs: Option<u64>,
    pub interval_mins: u64,
    pub total_bells_session: u64,
}

pub fn socket_path() -> &'static PathBuf {
    SOCKET_PATH.get_or_init(|| {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let uid = unsafe { libc::getuid() };
                PathBuf::from(format!("/run/user/{}", uid))
            });
        runtime_dir.join("mbell.sock")
    })
}

/// Server side - runs in the daemon
pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub async fn new() -> Result<Self, IpcError> {
        let path = socket_path();

        // Remove existing socket, ignoring NotFound error (avoids TOCTOU race)
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }

        let listener = UnixListener::bind(path)?;
        info!("IPC server listening on {:?}", path);

        Ok(Self { listener })
    }

    pub async fn accept(&self) -> Result<UnixStream, IpcError> {
        let (stream, _) = self.listener.accept().await?;
        Ok(stream)
    }

    pub async fn handle_connection(
        stream: UnixStream,
        cmd_tx: mpsc::Sender<(Command, mpsc::Sender<Response>)>,
    ) {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        match reader.read_line(&mut line).await {
            Ok(0) => return, // Connection closed
            Ok(_) => {}
            Err(e) => {
                error!("Failed to read from socket: {}", e);
                return;
            }
        }

        let command: Command = match serde_json::from_str(&line) {
            Ok(cmd) => cmd,
            Err(e) => {
                error!("Failed to parse command: {}", e);
                let response = Response::Error(format!("Invalid command: {}", e));
                if let Err(e) = write_json_response(&mut writer, &response).await {
                    error!("Failed to send error response: {}", e);
                }
                return;
            }
        };

        debug!("Received command: {:?}", command);

        // Create response channel
        let (resp_tx, mut resp_rx) = mpsc::channel(1);

        // Send command to daemon
        if cmd_tx.send((command, resp_tx)).await.is_err() {
            let response = Response::Error("Daemon not responding".to_string());
            if let Err(e) = write_json_response(&mut writer, &response).await {
                error!("Failed to send error response: {}", e);
            }
            return;
        }

        // Wait for response
        if let Some(response) = resp_rx.recv().await {
            if let Err(e) = write_json_response(&mut writer, &response).await {
                error!("Failed to send response: {}", e);
            }
        }
    }
}

async fn write_json_response<W: tokio::io::AsyncWriteExt + Unpin>(
    writer: &mut W,
    response: &Response,
) -> Result<(), IpcError> {
    let json = serde_json::to_string(response)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Remove socket, ignoring errors (avoids TOCTOU race)
        let _ = std::fs::remove_file(socket_path());
    }
}

/// Client side - used by CLI commands
pub struct IpcClient;

impl IpcClient {
    pub async fn send_command(command: Command) -> Result<Response, IpcError> {
        let path = socket_path();

        if !path.exists() {
            return Err(IpcError::DaemonNotRunning);
        }

        let stream = UnixStream::connect(&path)
            .await
            .map_err(|e| IpcError::ConnectionFailed(e.to_string()))?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Send command
        let json = serde_json::to_string(&command)?;
        writer.write_all(format!("{}\n", json).as_bytes()).await?;

        // Read response
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: Response = serde_json::from_str(&line)?;
        Ok(response)
    }

    pub fn is_daemon_running() -> bool {
        socket_path().exists()
    }
}
