#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod gui;
mod relay;

use tokio::sync::watch;

fn main() {
    let headless = std::env::args().any(|a| a == "--headless");

    if headless {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(async {
            let cfg = config::AppConfig::load();

            if cfg.proxies.is_empty() {
                eprintln!(
                    "No proxies configured. Edit: {}",
                    config::config_path().display()
                );
                eprintln!("Example config:\n");
                let example = config::AppConfig {
                    proxies: vec![config::ProxyEntry {
                        name: "my-proxy".into(),
                        remote_host: "proxy.example.com".into(),
                        remote_port: 1080,
                        username: "user".into(),
                        password: "pass".into(),
                        local_port: 11080,
                        enabled: true,
                    }],
                };
                eprintln!("{}", toml::to_string_pretty(&example).unwrap());
                return;
            }

            let (shutdown_tx, shutdown_rx) = watch::channel(false);
            let relay_handle = tokio::spawn(relay::run_all(cfg, shutdown_rx));

            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl+c");
            eprintln!("\nShutting down...");
            let _ = shutdown_tx.send(true);
            let _ = relay_handle.await;
        });
    } else {
        gui::run_gui().expect("failed to launch GUI");
    }
}
