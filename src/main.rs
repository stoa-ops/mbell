use clap::{Parser, Subcommand};
use mbell::config::Config;
use mbell::daemon::Daemon;
use mbell::ipc::{Command, IpcClient, Response};
use mbell::stats::Stats;
use std::process::Command as ProcessCommand;

#[derive(Parser)]
#[command(name = "mbell")]
#[command(author, version, about = "Mindfulness bell daemon for Linux")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon
    Start {
        /// Run in background (detached)
        #[arg(short, long)]
        detach: bool,
    },
    /// Stop the running daemon
    Stop,
    /// Pause the bell (daemon stays running)
    Pause,
    /// Resume the bell
    Resume,
    /// Show daemon status and next bell time
    Status,
    /// Show statistics
    Stats {
        /// Reset all statistics
        #[arg(long)]
        reset: bool,
    },
    /// Ring the bell immediately
    Ring,
    /// Configuration commands
    Config {
        /// Open config in $EDITOR
        #[arg(long)]
        edit: bool,
        /// Print config file path
        #[arg(long)]
        path: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { detach } => cmd_start(detach).await,
        Commands::Stop => cmd_stop().await,
        Commands::Pause => cmd_pause().await,
        Commands::Resume => cmd_resume().await,
        Commands::Status => cmd_status().await,
        Commands::Stats { reset } => cmd_stats(reset),
        Commands::Ring => cmd_ring().await,
        Commands::Config { edit, path } => cmd_config(edit, path),
    }
}

async fn cmd_start(detach: bool) {
    if IpcClient::is_daemon_running() {
        eprintln!("Daemon is already running");
        std::process::exit(1);
    }

    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    if detach {
        // Fork and run in background
        match daemonize::Daemonize::new()
            .working_directory(std::env::current_dir().unwrap_or_else(|_| "/".into()))
            .start()
        {
            Ok(_) => {
                // We're now in the child process
                mbell::logging::init(&config.log_level);
                let daemon = Daemon::new(config);
                if let Err(e) = daemon.run().await {
                    tracing::error!("Daemon error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to daemonize: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Run in foreground
        mbell::logging::init(&config.log_level);
        println!("Starting mbell daemon (Ctrl+C to stop)");
        let daemon = Daemon::new(config);
        if let Err(e) = daemon.run().await {
            eprintln!("Daemon error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn cmd_stop() {
    match IpcClient::send_command(Command::Stop).await {
        Ok(Response::Ok) => println!("Daemon stopped"),
        Ok(Response::Error(e)) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to stop daemon: {}", e);
            std::process::exit(1);
        }
    }
}

async fn cmd_pause() {
    match IpcClient::send_command(Command::Pause).await {
        Ok(Response::Ok) => println!("Bell paused"),
        Ok(Response::Error(e)) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to pause: {}", e);
            std::process::exit(1);
        }
    }
}

async fn cmd_resume() {
    match IpcClient::send_command(Command::Resume).await {
        Ok(Response::Ok) => println!("Bell resumed"),
        Ok(Response::Error(e)) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to resume: {}", e);
            std::process::exit(1);
        }
    }
}

async fn cmd_status() {
    match IpcClient::send_command(Command::Status).await {
        Ok(Response::Status(info)) => {
            println!("Status:     {}", info.state);
            println!("Interval:   {} minutes", info.interval_mins);
            if let Some(secs) = info.next_bell_secs {
                let mins = secs / 60;
                let remaining_secs = secs % 60;
                println!("Next bell:  {}:{:02}", mins, remaining_secs);
            } else {
                println!("Next bell:  (paused)");
            }
            println!("Session:    {} bells", info.total_bells_session);
        }
        Ok(Response::Error(e)) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("Daemon not running: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_stats(reset: bool) {
    if reset {
        let mut stats = Stats::load().unwrap_or_default();
        if let Err(e) = stats.reset() {
            eprintln!("Failed to reset stats: {}", e);
            std::process::exit(1);
        }
        println!("Statistics reset");
    } else {
        let stats = match Stats::load() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to load stats: {}", e);
                std::process::exit(1);
            }
        };
        println!("{}", stats.display());
    }
}

async fn cmd_ring() {
    // First try to send to daemon if running
    if IpcClient::is_daemon_running() {
        match IpcClient::send_command(Command::Ring).await {
            Ok(Response::Ok) => {
                println!("Bell rung");
                return;
            }
            Ok(Response::Error(e)) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            Ok(_) => return,
            Err(_) => {
                // Fall through to ring directly
            }
        }
    }

    // Ring directly if daemon not running
    let config = Config::load().unwrap_or_default();
    if let Err(e) = mbell::audio::ring(config.volume) {
        eprintln!("Failed to play bell: {}", e);
        std::process::exit(1);
    }
    println!("Bell rung");
}

fn cmd_config(edit: bool, path: bool) {
    let config_path = match Config::config_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get config path: {}", e);
            std::process::exit(1);
        }
    };

    if path {
        println!("{}", config_path.display());
        return;
    }

    if edit {
        // Ensure config exists
        if !config_path.exists() {
            if let Err(e) = Config::default().save() {
                eprintln!("Failed to create config: {}", e);
                std::process::exit(1);
            }
        }

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        let status = ProcessCommand::new(&editor)
            .arg(&config_path)
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                eprintln!("Editor exited with status: {}", s);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Failed to open editor: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Show current config
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    println!("interval  = {}", config.interval);
    println!("volume    = {}", config.volume);
    println!("log_level = {}", config.log_level);
    println!();
    println!("Config file: {}", config_path.display());
}
