use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use zbus::{proxy, Connection};

#[derive(Debug, Clone)]
pub enum LockEvent {
    Locked,
    Unlocked,
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1"
)]
trait Session {
    #[zbus(signal)]
    fn lock(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unlock(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
}

pub struct LockMonitor {
    tx: mpsc::Sender<LockEvent>,
}

impl LockMonitor {
    pub fn new(tx: mpsc::Sender<LockEvent>) -> Self {
        Self { tx }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connection = Connection::system().await?;

        // Get the current session path
        let session_path = get_session_path(&connection).await?;
        debug!("Monitoring session: {}", session_path);

        let proxy = SessionProxy::builder(&connection)
            .path(session_path.as_str())?
            .build()
            .await?;

        // Check initial lock state
        match proxy.locked_hint().await {
            Ok(locked) => {
                if locked {
                    info!("Session is currently locked");
                    let _ = self.tx.send(LockEvent::Locked).await;
                }
            }
            Err(e) => {
                warn!("Could not get initial lock state: {}", e);
            }
        }

        let tx_lock = self.tx.clone();
        let tx_unlock = self.tx.clone();

        // Subscribe to Lock signal
        let mut lock_stream = proxy.receive_lock().await?;
        let lock_handle = tokio::spawn(async move {
            while let Some(_) = lock_stream.next().await {
                info!("Screen locked");
                if tx_lock.send(LockEvent::Locked).await.is_err() {
                    break;
                }
            }
        });

        // Subscribe to Unlock signal
        let mut unlock_stream = proxy.receive_unlock().await?;
        let unlock_handle = tokio::spawn(async move {
            while let Some(_) = unlock_stream.next().await {
                info!("Screen unlocked");
                if tx_unlock.send(LockEvent::Unlocked).await.is_err() {
                    break;
                }
            }
        });

        // Wait for either to complete (shouldn't happen unless connection drops)
        tokio::select! {
            _ = lock_handle => {
                error!("Lock signal stream ended unexpectedly");
            }
            _ = unlock_handle => {
                error!("Unlock signal stream ended unexpectedly");
            }
        }

        Ok(())
    }
}

async fn get_session_path(connection: &Connection) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Try to get XDG_SESSION_ID first
    if let Ok(session_id) = std::env::var("XDG_SESSION_ID") {
        return Ok(format!("/org/freedesktop/login1/session/{}", session_id));
    }

    // Fall back to getting the session by PID
    let reply = connection
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "GetSessionByPID",
            &(std::process::id()),
        )
        .await?;

    let session_path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize()?;
    Ok(session_path.to_string())
}

/// Handle for the lock monitor that can be used to abort its tasks on shutdown
pub struct LockMonitorHandle {
    _task: JoinHandle<()>,
}

impl LockMonitorHandle {
    /// Abort the lock monitor tasks
    pub fn abort(&self) {
        self._task.abort();
    }
}

/// Start the lock monitor in a background task
pub fn start_lock_monitor() -> (mpsc::Receiver<LockEvent>, LockMonitorHandle) {
    let (tx, rx) = mpsc::channel(10);

    let task = tokio::spawn(async move {
        let monitor = LockMonitor::new(tx);
        if let Err(e) = monitor.run().await {
            error!("Lock monitor error: {}", e);
        }
    });

    (rx, LockMonitorHandle { _task: task })
}
