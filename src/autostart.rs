#[cfg(target_os = "linux")]
fn desktop_file_path() -> std::path::PathBuf {
    dirs::config_dir()
        .expect("could not determine config directory")
        .join("autostart")
        .join("tropa-relay.desktop")
}

#[cfg(target_os = "linux")]
pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("failed to get exe path: {e}"))?;
    let path = desktop_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create autostart dir: {e}"))?;
    }
    let contents = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Tropa Relay\n\
         Exec={} --headless\n\
         X-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    std::fs::write(&path, contents).map_err(|e| format!("failed to write desktop file: {e}"))
}

#[cfg(target_os = "linux")]
pub fn disable() -> Result<(), String> {
    let path = desktop_file_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("failed to remove desktop file: {e}"))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub fn is_enabled() -> bool {
    desktop_file_path().exists()
}

#[cfg(target_os = "windows")]
pub fn enable() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let exe = std::env::current_exe().map_err(|e| format!("failed to get exe path: {e}"))?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .map_err(|e| format!("failed to open registry key: {e}"))?;
    let value = format!("\"{}\" --headless", exe.display());
    key.set_value("tropa-relay", &value)
        .map_err(|e| format!("failed to set registry value: {e}"))
}

#[cfg(target_os = "windows")]
pub fn disable() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
        .map_err(|e| format!("failed to open registry key: {e}"))?;
    match key.delete_value("tropa-relay") {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("failed to delete registry value: {e}")),
    }
}

#[cfg(target_os = "windows")]
pub fn is_enabled() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") else {
        return false;
    };
    key.get_value::<String, _>("tropa-relay").is_ok()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn enable() -> Result<(), String> {
    Err("autostart is not supported on this platform".into())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn disable() -> Result<(), String> {
    Err("autostart is not supported on this platform".into())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn is_enabled() -> bool {
    false
}
