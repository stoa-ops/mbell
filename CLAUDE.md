# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

mbell is a lightweight Rust daemon that rings a Tibetan singing bowl at configurable intervals to encourage mindfulness. It runs as a background process on Linux with automatic pause during screen lock.

## Build Commands

```bash
cargo build --release          # Build optimized binary
cargo build --release --locked # Build with locked dependencies (for packaging)
cargo run -- <command>         # Run in development
```

No test suite is currently implemented.

## Architecture

### Event-Driven State Machine (daemon.rs)

The daemon uses `tokio::select!` to handle concurrent events:
- IPC commands via Unix socket
- Timer ticks for bell scheduling (per-second intervals)
- D-Bus signals for screen lock/unlock
- Unix signals (SIGTERM/SIGINT) for graceful shutdown

Three states: `Running`, `Paused` (manual), `Locked` (screen locked)

### IPC Protocol (ipc.rs)

Unix socket at `/run/user/$UID/mbell.sock` with JSON-encoded messages:
- Commands: `Pause`, `Resume`, `Stop`, `Status`, `Ring`, `Reload`
- Responses: `Ok`, `Status(StatusInfo)`, `Error(String)`

### Key Integration Points

- **Audio (audio.rs)**: Uses rodio with embedded OGG file. Auto-detects PipeWire → PulseAudio → ALSA.
- **Screen Lock (lock.rs)**: Monitors `org.freedesktop.login1.Session` via zbus for Lock/Unlock signals.
- **Config (config.rs)**: TOML at `~/.config/mbell/config.toml`. Validates interval > 0, volume 0-100.
- **Stats (stats.rs)**: JSON at `~/.local/share/mbell/stats.json`. Tracks bells, streaks, active days.

### CLI Structure (main.rs)

Clap-based subcommands: `start [-d]`, `stop`, `pause`, `resume`, `status`, `stats [--reset]`, `ring`, `config [--edit|--path]`

## Key Dependencies

- **tokio** (async runtime)
- **rodio** (audio playback)
- **zbus** (D-Bus for lock detection)
- **clap** (CLI parsing)
- **serde/toml** (config/IPC serialization)

## Build Optimizations

Release profile uses `opt-level = "z"`, LTO, and stripping for minimal binary size.
