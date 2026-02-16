# mbell - Mindfulness Bell for Linux

A lightweight Rust daemon that rings a gentle Tibetan singing bowl at configurable intervals to encourage mindfulness. Runs silently in the background and auto-pauses when your screen is locked.

## Features

- Configurable bell interval (default: 10 minutes)
- Adjustable volume
- Auto-detects audio backend (PipeWire, PulseAudio, or ALSA)
- Automatically pauses when screen is locked (via D-Bus/systemd-logind)
- Persistent statistics tracking (total bells, streaks, etc.)
- Unix socket IPC for control commands
- Systemd user service support

## Installation

### From source

```bash
git clone https://github.com/stoa-ops/mbell
cd mbell
cargo build --release
cp target/release/mbell ~/.cargo/bin/
```

### Ubuntu / Debian

Install build dependencies:

```bash
sudo apt install build-essential pkg-config libasound2-dev
```

Install Rust if you don't have it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

Build and install:

```bash
make
sudo make install
```

Enable the systemd user service:

```bash
systemctl --user daemon-reload
systemctl --user enable --now mbell
```

To uninstall:

```bash
sudo make uninstall
```

### Arch Linux (AUR)

```bash
yay -S mbell
```

## Usage

### Starting the daemon

```bash
# Run in foreground
mbell start

# Run in background (detached)
mbell start -d
```

### Controlling the daemon

```bash
mbell pause     # Pause the bell
mbell resume    # Resume the bell
mbell stop      # Stop the daemon
mbell status    # Show status and next bell time
mbell ring      # Ring the bell immediately
```

### Statistics

```bash
mbell stats           # Show statistics
mbell stats --reset   # Reset all statistics
```

### Configuration

```bash
mbell config          # Show current configuration
mbell config --edit   # Open config in $EDITOR
mbell config --path   # Print config file path
```

Configuration file: `~/.config/mbell/config.toml`

```toml
# Interval between bells in minutes
interval = 10

# Volume level (0-100)
volume = 70

# Log level: error, warn, info, debug, trace
log_level = "info"
```

## Systemd Integration

Install the user service:

```bash
mkdir -p ~/.config/systemd/user/
cp mbell.service ~/.config/systemd/user/

# Enable and start
systemctl --user enable mbell
systemctl --user start mbell

# Check status
systemctl --user status mbell
```

## File Locations

| File | Path |
|------|------|
| Config | `~/.config/mbell/config.toml` |
| Statistics | `~/.local/share/mbell/stats.json` |
| Socket | `/run/user/$UID/mbell.sock` |

## Building

Requirements:
- Rust 1.70+
- ALSA development libraries (`alsa-lib` on Arch, `libasound2-dev` on Debian/Ubuntu)

```bash
cargo build --release
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Bell sound: Tibetan singing bowl sample (CC0/Public Domain)
