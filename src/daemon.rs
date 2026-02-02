use crate::audio;
use crate::config::Config;
use crate::ipc::{Command, IpcServer, Response, StatusInfo};
use crate::lock::{start_lock_monitor, LockEvent};
use crate::stats::Stats;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DaemonState {
    Running,
    Paused,
    Locked,
}

impl std::fmt::Display for DaemonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonState::Running => write!(f, "running"),
            DaemonState::Paused => write!(f, "paused"),
            DaemonState::Locked => write!(f, "locked"),
        }
    }
}

pub struct Daemon {
    config: Config,
    state: DaemonState,
    stats: Stats,
    bells_this_session: u64,
    last_bell: Instant,
    was_paused_before_lock: bool,
}

impl Daemon {
    pub fn new(config: Config) -> Self {
        let stats = Stats::load().unwrap_or_default();

        Self {
            config,
            state: DaemonState::Running,
            stats,
            bells_this_session: 0,
            last_bell: Instant::now(),
            was_paused_before_lock: false,
        }
    }

    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "Daemon starting with interval of {} minutes",
            self.config.interval
        );

        // Start IPC server
        let ipc_server = IpcServer::new().await?;
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<(Command, mpsc::Sender<Response>)>(32);

        // Start lock monitor
        let mut lock_rx = start_lock_monitor();

        // Set up signal handlers
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

        // Timer for bell scheduling
        let interval_duration = Duration::from_secs(self.config.interval * 60);
        let mut timer = interval(Duration::from_secs(1)); // Check every second

        info!("Daemon running, first bell in {} minutes", self.config.interval);

        loop {
            tokio::select! {
                // Handle IPC connections
                Ok(stream) = ipc_server.accept() => {
                    let cmd_tx = cmd_tx.clone();
                    tokio::spawn(async move {
                        IpcServer::handle_connection(stream, cmd_tx).await;
                    });
                }

                // Handle commands from IPC
                Some((command, resp_tx)) = cmd_rx.recv() => {
                    let is_stop = matches!(command, Command::Stop);
                    let response = self.handle_command(command);

                    let _ = resp_tx.send(response).await;

                    if is_stop {
                        info!("Stop command received, shutting down");
                        break;
                    }
                }

                // Handle lock events
                Some(event) = lock_rx.recv() => {
                    self.handle_lock_event(event);
                }

                // Timer tick
                _ = timer.tick() => {
                    if self.state == DaemonState::Running {
                        let elapsed = self.last_bell.elapsed();
                        if elapsed >= interval_duration {
                            self.ring_bell();
                        }
                    }
                }

                // Signal handlers
                _ = sigterm.recv() => {
                    info!("SIGTERM received, shutting down");
                    break;
                }
                _ = sigint.recv() => {
                    info!("SIGINT received, shutting down");
                    break;
                }
            }
        }

        info!("Daemon stopped");
        Ok(())
    }

    fn handle_command(&mut self, command: Command) -> Response {
        match command {
            Command::Pause => {
                if self.state == DaemonState::Running {
                    self.state = DaemonState::Paused;
                    info!("Bell paused");
                    Response::Ok
                } else {
                    Response::Error(format!("Cannot pause: currently {}", self.state))
                }
            }
            Command::Resume => {
                if self.state == DaemonState::Paused {
                    self.state = DaemonState::Running;
                    info!("Bell resumed");
                    Response::Ok
                } else {
                    Response::Error(format!("Cannot resume: currently {}", self.state))
                }
            }
            Command::Stop => {
                info!("Stop requested");
                Response::Ok
            }
            Command::Status => {
                let next_bell_secs = if self.state == DaemonState::Running {
                    let interval_secs = self.config.interval * 60;
                    let elapsed = self.last_bell.elapsed().as_secs();
                    Some(interval_secs.saturating_sub(elapsed))
                } else {
                    None
                };

                Response::Status(StatusInfo {
                    state: self.state.to_string(),
                    next_bell_secs,
                    interval_mins: self.config.interval,
                    total_bells_session: self.bells_this_session,
                })
            }
            Command::Ring => {
                self.ring_bell();
                Response::Ok
            }
            Command::Reload => {
                match Config::load() {
                    Ok(config) => {
                        self.config = config;
                        info!("Configuration reloaded");
                        Response::Ok
                    }
                    Err(e) => Response::Error(format!("Failed to reload config: {}", e)),
                }
            }
        }
    }

    fn handle_lock_event(&mut self, event: LockEvent) {
        match event {
            LockEvent::Locked => {
                self.was_paused_before_lock = self.state == DaemonState::Paused;
                if self.state == DaemonState::Running {
                    self.state = DaemonState::Locked;
                    info!("Screen locked, pausing bell");
                }
            }
            LockEvent::Unlocked => {
                if self.state == DaemonState::Locked {
                    if self.was_paused_before_lock {
                        self.state = DaemonState::Paused;
                        info!("Screen unlocked, bell remains paused (was paused before lock)");
                    } else {
                        self.state = DaemonState::Running;
                        // Reset the timer so we don't immediately ring after unlock
                        self.last_bell = Instant::now();
                        info!("Screen unlocked, resuming bell");
                    }
                }
            }
        }
    }

    fn ring_bell(&mut self) {
        debug!("Ringing bell");
        audio::ring_async(self.config.volume);
        self.bells_this_session += 1;
        self.stats.record_bell();
        self.last_bell = Instant::now();
        info!("Bell #{} this session", self.bells_this_session);
    }
}
