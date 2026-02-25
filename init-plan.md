# tropa-relay — Init Plan

## What It Does

A lightweight SOCKS5 relay. Takes authenticated remote SOCKS5 proxies and re-exposes them on localhost with no auth.

```
[App/Browser] --> localhost:PORT (no auth) --> remote-proxy:PORT (user/pass) --> Internet
```

Multiple entries supported — each remote proxy gets its own local port.

## Target

- Platforms: Linux, Windows
- Arch: amd64 only
- Language: Rust

## Components

### 1. Core Relay (`tropa-relay`)

- Listens on `127.0.0.1:<local_port>` per entry, no SOCKS5 auth
- Connects upstream to `<remote_host>:<remote_port>` with username/password SOCKS5 auth
- Proxies traffic bidirectionally
- Config stored as a single JSON/TOML file

Config example (`config.toml`):

```toml
[[proxy]]
name = "US East"
remote_host = "proxy.example.com"
remote_port = 1080
username = "user1"
password = "pass1"
local_port = 10801

[[proxy]]
name = "EU West"
remote_host = "eu.example.com"
remote_port = 1080
username = "user2"
password = "pass2"
local_port = 10802
```

### 2. GUI (Settings + Tray)

- System tray icon — runs in background
- Settings window: add/edit/remove proxy entries, toggle individual proxies on/off
- Start on boot toggle (writes to OS autostart mechanism)
- Minimal UI — just a table of proxies + add/edit/remove buttons

GUI framework: **egui** via `eframe` — pure Rust, single binary, no system deps, works on both Linux and Windows.

### 3. Autostart

- **Linux:** `.desktop` file in `~/.config/autostart/`
- **Windows:** Registry key in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`

## Crate Dependencies (Minimal)

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime for proxy relay |
| `fast-socks5` | SOCKS5 server + client (see below) |
| `eframe`/`egui` | GUI + tray |
| `serde` + `toml` | Config serialization |
| `dirs` | Platform config path resolution |

## SOCKS5 Implementation — `fast-socks5`

Using **[`fast-socks5`](https://github.com/dizda/fast-socks5)** v1.0.0 (MIT, tokio-based, ~1M downloads).

It provides both server and client in one crate — exactly what we need for a relay:

- **Server side:** `Socks5Server` — accepts incoming SOCKS5 connections, configurable auth (we use no-auth)
- **Client side:** `Socks5Stream::connect_with_password()` — connects to upstream proxy with user/pass

### Relay flow

```
1. fast-socks5 server accepts connection on 127.0.0.1:<local_port> (no auth)
2. Client sends CONNECT request with target address
3. We extract target_addr + target_port from the request
4. Open Socks5Stream::connect_with_password(upstream, target, user, pass)
5. tokio::io::copy_bidirectional() between client stream and upstream stream
```

### Why this crate

| Considered | Verdict |
|------------|---------|
| `fast-socks5` | **Winner.** Server + client, MIT, active (v1.0.0 Jan 2026), 591 GH stars |
| `tokio-socks` | Client-only. Would still need a server impl. |
| `socks5-server` | GPL-3.0 — no go |
| `socks5-impl` | GPL-3.0 — no go |
| Hand-roll | Protocol is simple (~200 LOC) but `fast-socks5` handles edge cases already |

## Binary Output

Single binary. Modes:

- `tropa-relay` — launches GUI (default)
- `tropa-relay --headless` — runs relay only, no GUI (for service/background use)

## Config Location

- **Linux:** `~/.config/tropa-relay/config.toml`
- **Windows:** `%APPDATA%\tropa-relay\config.toml`

## Build

```bash
# native
cargo build --release

# cross-compile windows from linux
cargo build --release --target x86_64-pc-windows-gnu
```

## Non-Goals

- No HTTP/HTTPS proxy support
- No proxy chaining
- No DNS-over-SOCKS
- No auto-discovery
- No remote management
- No logging UI
