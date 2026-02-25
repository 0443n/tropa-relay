# TODO — tropa-relay

## Phase 1: Project Skeleton

- [x] `cargo init` with binary target
- [x] Add dependencies to `Cargo.toml`: `tokio`, `fast-socks5`, `serde`, `toml`, `dirs`, `eframe`
- [ ] Set up module structure: `main.rs`, `config.rs`, `relay.rs`, `gui.rs`, `autostart.rs` (missing `autostart.rs`)

## Phase 2: Config

- [x] Define `ProxyEntry` struct (name, remote_host, remote_port, username, password, local_port, enabled)
- [ ] Define `AppConfig` struct (vec of entries, autostart bool) — struct exists but missing autostart field
- [x] Implement load/save to platform config path (`dirs` crate)
- [x] Create default config if file doesn't exist

## Phase 3: Core Relay

- [x] Single-entry relay: listen on `127.0.0.1:<local_port>` with no-auth (`fast-socks5` server)
- [x] On CONNECT: extract target addr, open `Socks5Stream::connect_with_password()` to upstream
- [x] Bidirectional copy with `tokio::io::copy_bidirectional()`
- [x] Multi-entry: spawn a relay task per enabled proxy entry
- [x] Graceful shutdown — stop individual relays, stop all on exit
- [x] `--headless` mode: run relay only, block on ctrl+c

## Phase 4: GUI

- [x] Basic eframe window with proxy table (name, remote host, local port, status)
- [x] Add proxy dialog (fields: name, remote_host, remote_port, username, password, local_port)
- [x] Edit proxy — populate dialog with existing entry
- [x] Remove proxy — with confirmation
- [x] Per-proxy on/off toggle — starts/stops individual relay at runtime
- [x] Save config on every change
- [ ] System tray icon — minimize to tray, restore on click
- [ ] Autostart toggle in UI

## Phase 5: Autostart

- [ ] Linux: write/remove `.desktop` file in `~/.config/autostart/`
- [ ] Windows: write/remove registry key in `HKCU\...\Run`
- [ ] Wire to GUI toggle

## GUI Improvements

- [ ] Add/edit proxy should open a real separate window, not a modal inside the main window
- [ ] Fix inconsistent button heights across the UI
- [ ] Fix text alignment and spacing — things feel misplaced
- [ ] Replace the pill toggle switch with something less out-of-place
- [ ] Make the UI feel less flat and more responsive (hover states, feedback, depth)

## Phase 6: Polish + Ship

- [ ] Error handling — surface relay errors in GUI (connection refused, auth failed, port in use)
- [ ] Validate config on save (no duplicate local ports, ports in valid range)
- [ ] Test on Linux amd64
- [ ] Test on Windows amd64
- [ ] `cargo build --release` for both targets
- [ ] Write minimal README with usage
