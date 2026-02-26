#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod config;
mod gui;
mod relay;

use tokio::sync::watch;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let headless = args.iter().any(|a| a == "--headless");
    let minimized = args.iter().any(|a| a == "--minimized");
    let foreground = args.iter().any(|a| a == "--foreground");
    let help = args.iter().any(|a| a == "--help" || a == "-h");

    if help {
        println!(
            "\
Tropa Relay — Local SOCKS5 relay

Runs all enabled proxies from the config file.
Config: {}

Usage: tropa-relay [OPTIONS]

Options:
    --minimized    Start minimized to system tray
    --headless     Run without GUI (CLI mode, runs all enabled proxies)
    --foreground   Keep running in the foreground terminal
    --help         Show this help message",
            config::config_path().display()
        );
        return;
    }

    // GUI mode: fork to background unless --foreground or --headless
    #[cfg(target_os = "linux")]
    if !headless && !foreground {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        let exe = std::env::current_exe().expect("failed to get current exe path");
        let mut child_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
        child_args.push("--foreground");

        let null = std::fs::File::open("/dev/null").expect("failed to open /dev/null");
        let null_out = null.try_clone().expect("failed to clone /dev/null");
        let null_err = null.try_clone().expect("failed to clone /dev/null");

        unsafe {
            Command::new(exe)
                .args(&child_args)
                .stdin(null)
                .stdout(null_out)
                .stderr(null_err)
                .pre_exec(|| {
                    libc::setsid();
                    Ok(())
                })
                .spawn()
                .expect("failed to fork to background");
        }
        println!("Tropa Relay running in background");
        return;
    }

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
                    autostart: false,
                    auto_update: true,
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
        gui::run_gui(minimized).expect("failed to launch GUI");
    }
}
