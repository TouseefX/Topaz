// Persistent app state — save/restore to JSON so settings survive close/reopen.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(not(target_os = "android"))]
mod log {
    pub use std::eprintln as error;
    pub use std::eprintln as warn;
    pub use std::println as info;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedState {
    pub theme: String,
    pub add_watermark: bool,
    pub show_upvalue_comments: bool,
    pub server_port: u16,
    pub server_luau: bool,
    pub server_lua51: bool,
    pub server_encode_key: u8,
    pub encode_key: u8,
    pub keep_alive: bool,
}

impl Default for SavedState {
    fn default() -> Self {
        Self {
            theme: "System".into(),
            add_watermark: true,
            show_upvalue_comments: true,
            server_port: 3000,
            server_luau: true,
            server_lua51: false,
            server_encode_key: 203,
            encode_key: 1,
            keep_alive: false,
        }
    }
}

fn state_path() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        // Android: app-private files dir
        PathBuf::from("/data/data/com.exec.topaz/files/topaz_config.json")
    }
    #[cfg(not(target_os = "android"))]
    {
        // Desktop: standard config dir
        let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("topaz");
        let _ = std::fs::create_dir_all(&p);
        p.push("config.json");
        p
    }
}

pub fn load() -> SavedState {
    let path = state_path();
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            match serde_json::from_str::<SavedState>(&json) {
                Ok(s) => {
                    log::info!("Loaded saved state from {}", path.display());
                    return s;
                }
                Err(e) => log::warn!("Failed to parse saved state: {e}"),
            }
        }
        Err(e) => log::info!("No saved state found at {}: {e}", path.display()),
    }
    SavedState::default()
}

pub fn save(state: &SavedState) {
    let path = state_path();
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&path, &json) {
                Ok(()) => log::info!("Saved state to {}", path.display()),
                Err(e) => log::error!("Failed to save state to {}: {e}", path.display()),
            }
        }
        Err(e) => log::error!("Failed to serialize state: {e}"),
    }
}
