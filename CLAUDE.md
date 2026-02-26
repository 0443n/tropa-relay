# tropa-relay

Local SOCKS5 relay. Takes authenticated remote SOCKS5 proxies and re-exposes them on localhost with no auth.

```
[App/Browser] --> localhost:PORT (no auth) --> remote-proxy:PORT (user/pass) --> Internet
```

## Stack

- **Language:** Rust (edition 2024)
- **Async runtime:** tokio
- **SOCKS5:** fast-socks5 (server + client)
- **GUI:** iced 0.14 (retained-mode)
- **Config:** serde + toml, stored at platform config dir
- **Autostart:** XDG .desktop (Linux), winreg (Windows)

## Architecture

- `src/main.rs` — entry point, `--headless` flag for CLI-only mode
- `src/config.rs` — `AppConfig` / `ProxyEntry`, load/save
- `src/relay.rs` — async SOCKS5 relay (one task per proxy entry)
- `src/gui.rs` — iced app: list view, edit form, all styling
- `src/autostart.rs` — platform-specific enable/disable/is_enabled

## Targets

- Linux amd64, Windows amd64
- Single binary, two modes: GUI (default) or `--headless`

## Config location

- Linux: `~/.config/tropa-relay/config.toml`
- Windows: `%APPDATA%\tropa-relay\config.toml`

## GitHub

- Repo: `0443n/tropa-relay`

## Current status

See `TODO.md` for roadmap. Phases 1–5 complete. Next: Phase 6 (polish).
