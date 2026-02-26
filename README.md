<p align="center">
  <img src="assets/icon_128.png" alt="Tropa Relay" width="96" />
</p>

<h1 align="center">Tropa Relay</h1>

<p align="center">
  Local SOCKS5 relay for your proxies.<br>
  Takes authenticated remote SOCKS5 proxies and re-exposes them on localhost with no auth.
</p>

<p align="center">
  <a href="https://github.com/0443n/tropa-relay/releases/latest">Download</a>
</p>

## How it works

```
App/Browser  -->  localhost:PORT (no auth)  -->  remote-proxy:PORT (user/pass)  -->  Internet
```

Add your SOCKS5 proxies with credentials, toggle them on/off, and point your apps at `127.0.0.1:<local_port>` — no authentication needed on the local side.

## Install

Download the latest release for your platform:

- **Linux:** [tropa-relay-linux-amd64.zip](https://github.com/0443n/tropa-relay/releases/latest)
- **Windows:** [tropa-relay-windows-amd64.zip](https://github.com/0443n/tropa-relay/releases/latest)

Extract and run. No installation required.

## Usage

| Mode | Command | Description |
|------|---------|-------------|
| GUI | `tropa-relay` | Window + system tray |
| Minimized | `tropa-relay --minimized` | Tray only, click to open |
| Headless | `tropa-relay --headless` | CLI only, no GUI |

Run `tropa-relay --help` for all options.

## Config

Config is created automatically on first run at:

- **Linux:** `~/.config/tropa-relay/config.toml`
- **Windows:** `%APPDATA%\tropa-relay\config.toml`

```toml
[[proxies]]
name = "my-proxy"
remote_host = "proxy.example.com"
remote_port = 1080
username = "user"
password = "pass"
local_port = 11080
enabled = true
```

## Features

- Multiple proxy support with per-proxy on/off toggle
- System tray with minimize-to-tray
- Start on login (autostart)
- Auto-update from GitHub Releases
- Dark theme

## Building from source

Requires Rust and system dependencies (Linux: `libgtk-3-dev libxdo-dev pkg-config`).

```
cargo build --release
```
