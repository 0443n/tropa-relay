# tropa-relay

Local SOCKS5 relay that forwards traffic through authenticated upstream SOCKS5 proxies. Configure multiple proxies, toggle them on/off, and use them as local no-auth SOCKS5 endpoints.

## Usage

### GUI

```
cargo run
```

### Headless

```
cargo run -- --headless
```

Runs all enabled proxies without a window. Stop with Ctrl+C.

## Config

Config lives at `~/.config/tropa-relay/config.toml` (Linux) or the platform equivalent. Created automatically on first run.

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

Each proxy listens on `127.0.0.1:<local_port>` and forwards CONNECT requests through the remote SOCKS5 proxy with the given credentials.

## Building

```
cargo build --release
```
