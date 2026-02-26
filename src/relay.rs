use crate::config::{AppConfig, ProxyEntry};
use fast_socks5::client::Socks5Stream;
use std::net::Ipv6Addr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};

#[derive(Debug, Clone)]
pub enum RelayEvent {
    Listening,
    BindError(String),
}

/// Perform the server-side SOCKS5 handshake (no auth) and extract the target address.
async fn socks5_accept(stream: &mut TcpStream) -> Result<(String, u16), Box<dyn std::error::Error>> {
    // Read greeting: [VER, NMETHODS, METHODS...]
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;
    if buf[0] != 5 {
        return Err("not a SOCKS5 request".into());
    }
    let nmethods = buf[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).await?;

    // Reply: no authentication required
    stream.write_all(&[0x05, 0x00]).await?;

    // Read request: [VER, CMD, RSV, ATYP, DST.ADDR, DST.PORT]
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    if header[1] != 0x01 {
        // Only CONNECT is supported
        stream.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
        return Err("only CONNECT command is supported".into());
    }

    let host = match header[3] {
        0x01 => {
            // IPv4
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await?;
            format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
        }
        0x03 => {
            // Domain name
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize];
            stream.read_exact(&mut domain).await?;
            String::from_utf8(domain)?
        }
        0x04 => {
            // IPv6
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await?;
            let ipv6 = Ipv6Addr::new(
                u16::from_be_bytes([addr[0], addr[1]]),
                u16::from_be_bytes([addr[2], addr[3]]),
                u16::from_be_bytes([addr[4], addr[5]]),
                u16::from_be_bytes([addr[6], addr[7]]),
                u16::from_be_bytes([addr[8], addr[9]]),
                u16::from_be_bytes([addr[10], addr[11]]),
                u16::from_be_bytes([addr[12], addr[13]]),
                u16::from_be_bytes([addr[14], addr[15]]),
            );
            ipv6.to_string()
        }
        _ => {
            stream.write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await?;
            return Err("unsupported address type".into());
        }
    };

    let mut port_buf = [0u8; 2];
    stream.read_exact(&mut port_buf).await?;
    let port = u16::from_be_bytes(port_buf);

    Ok((host, port))
}

/// Send SOCKS5 reply back to the local client.
async fn socks5_reply(stream: &mut TcpStream, success: bool) -> std::io::Result<()> {
    let rep = if success { 0x00 } else { 0x01 };
    // Reply with bound address 0.0.0.0:0
    stream.write_all(&[0x05, rep, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await
}

/// Handle a single incoming connection: SOCKS5 handshake, upstream connect, relay.
async fn handle_connection(mut local: TcpStream, entry: &ProxyEntry) {
    let (host, port) = match socks5_accept(&mut local).await {
        Ok(target) => target,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("[{}] handshake failed: {_e}", entry.name);
            return;
        }
    };

    let upstream_addr = format!("{}:{}", entry.remote_host, entry.remote_port);
    let upstream = Socks5Stream::connect_with_password(
        &upstream_addr,
        host.clone(),
        port,
        entry.username.clone(),
        entry.password.clone(),
        fast_socks5::client::Config::default(),
    )
    .await;

    let mut upstream = match upstream {
        Ok(s) => {
            if let Err(_e) = socks5_reply(&mut local, true).await {
                #[cfg(debug_assertions)]
                eprintln!("[{}] failed to send reply: {_e}", entry.name);
                return;
            }
            s
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("[{}] upstream connect to {host}:{port} via {upstream_addr} failed: {_e}", entry.name);
            let _ = socks5_reply(&mut local, false).await;
            return;
        }
    };

    match tokio::io::copy_bidirectional(&mut local, &mut upstream).await {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("[{}] relay error for {host}:{port}: {_e}", entry.name);
        }
    }
}

/// Run a single relay: listen on local_port, forward through the upstream SOCKS5 proxy.
pub async fn run_relay(
    entry: ProxyEntry,
    mut shutdown: watch::Receiver<bool>,
    status_tx: Option<mpsc::UnboundedSender<(usize, RelayEvent)>>,
    index: usize,
) {
    let addr = format!("127.0.0.1:{}", entry.local_port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[{}] failed to bind {addr}: {e}", entry.name);
            if let Some(tx) = &status_tx {
                let _ = tx.send((index, RelayEvent::BindError(format!("Port {} already in use", entry.local_port))));
            }
            return;
        }
    };
    eprintln!(
        "[{}] listening on {addr} -> {}:{}",
        entry.name, entry.remote_host, entry.remote_port
    );
    if let Some(tx) = &status_tx {
        let _ = tx.send((index, RelayEvent::Listening));
    }

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _peer)) => {
                        let entry = entry.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, &entry).await;
                        });
                    }
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        eprintln!("[{}] accept error: {_e}", entry.name);
                    }
                }
            }
            _ = shutdown.changed() => {
                eprintln!("[{}] shutting down", entry.name);
                break;
            }
        }
    }
}

/// Spawn a relay task for each enabled proxy entry.
pub async fn run_all(config: AppConfig, shutdown: watch::Receiver<bool>) {
    let mut handles = Vec::new();
    for (i, entry) in config.proxies.into_iter().enumerate() {
        if !entry.enabled {
            continue;
        }
        let rx = shutdown.clone();
        handles.push(tokio::spawn(async move {
            run_relay(entry, rx, None, i).await;
        }));
    }
    for h in handles {
        let _ = h.await;
    }
}
