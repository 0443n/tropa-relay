# TODO — tropa-relay

## Completed

### Core (Phases 1–3)
- [x] Project skeleton, module structure
- [x] Config: `ProxyEntry`, `AppConfig`, load/save to platform config path
- [x] Core relay: SOCKS5 relay per entry, graceful shutdown, `--headless` mode

### GUI (Phase 4)
- [x] Proxy list with cards (name, host:port, local port)
- [x] Add/edit/remove proxy with form view
- [x] Per-proxy on/off toggle
- [x] Save config on every change
- [x] Dark theme, hover states, card depth

### Autostart (Phase 5)
- [x] Linux: XDG `.desktop` file in `~/.config/autostart/`
- [x] Windows: registry key in `HKCU\...\Run`
- [x] GUI toggle ("Start on login"), syncs with filesystem on launch

---

## Phase 6: Polish

- [x] Error handling — surface relay errors in GUI (connection refused, auth failed, port in use)
- [x] Config validation on save (no duplicate local ports, ports in valid range)
- [x] System tray icon — minimize to tray, restore on click
- [x] `--minimized` flag — starts with tray icon only, no window (window opens on tray click)
  - Three runtime modes: `tropa-relay` (window + tray), `--minimized` (tray only), `--headless` (no GUI)
  - Update autostart to use `--minimized` instead of `--headless`
- [x] Self-update via `self_update` crate (GitHub Releases backend)
  - Auto-check on launch, prompt if update available
  - Toggle in GUI to enable/disable auto-update
- [x] `--help` flag with usage text and config path
- [x] Fork to background by default (GUI mode), `--foreground` to stay attached
- [x] Tray menu polish — app name header with separator
- [ ] Test on Linux amd64
- [ ] Test on Windows amd64

## Phase 7: GitHub + CI

- [x] Push repo to GitHub (`0443n/tropa-relay`)
- [x] GitHub Actions workflow: build Linux amd64 + Windows amd64
- [x] Automated GitHub Releases on tag push (attach binaries)

## Phase 8: Distribution

### Windows
- [ ] NSIS installer (.exe), user-level install to `%LOCALAPPDATA%`
  - `RequestExecutionLevel user`, no UAC prompt
  - Install to `$LOCALAPPDATA\Programs\tropa-relay`
  - Registry keys in HKCU only
- [ ] Code signing — research free options:
  - Certum Open Source Certificate (free for OSS)
  - SignPath Foundation (free for OSI-approved projects)
  - Azure Trusted Signing (free tier)
  - Without signing, SmartScreen will warn on first run

### Linux
- [ ] Ship binary only, recommend `~/.local/bin/`
- [ ] No packaging (no .deb/.rpm) for now

### Both
- [ ] Self-update pulls latest release from GitHub Releases
- [ ] README with install instructions
