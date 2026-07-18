//! Cross-process server status/stats sharing (Android only).
//!
//! On Android, `KeepAliveService` (and the HTTP server it now owns) runs in a
//! separate OS process (`:service`, see Cargo.toml) from the UI process. That
//! split is what lets the UI process freely die and restart when winit hits
//! its one-EventLoop-per-process limit (see `android_main` in lib.rs) without
//! taking the running server down with it — but it also means the two
//! processes no longer share memory, so `Arc<ServerStats>` can't be handed
//! across the boundary like it used to be.
//!
//! Instead, the server process snapshots its state into this small JSON
//! struct and writes it to a file in app-private storage every second (and
//! on every state transition). The UI process just reads that file whenever
//! it wants to display current stats — a poll, not true IPC, but plenty for
//! numbers that only need to be "current within ~1s" on a status screen.
//!
//! Not used on desktop, where the server already runs in the same process as
//! the UI and can just share an `Arc` directly.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const STATS_FILE_NAME: &str = "topaz_server_status.json";

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub enum SharedState {
    #[default]
    Stopped,
    Starting {
        port: u16,
    },
    Running {
        addr: String,
        started_at_ms: u64,
    },
    Stopping,
    Failed {
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SharedServerStatus {
    pub state: SharedState,
    pub luau_requests: u64,
    pub lua51_requests: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub errors: u64,
    /// Unix millis of the last request, if any.
    pub last_request_ms: Option<u64>,
    /// Unix millis this snapshot was written — lets a reader notice a stale
    /// file (e.g. the server process died without a clean shutdown).
    pub updated_ms: u64,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Write the file atomically (write to a temp path, then rename) so a
/// concurrent reader in the other process never sees a half-written file.
pub fn write_status(path: &Path, status: &SharedServerStatus) {
    let Ok(json) = serde_json::to_vec(status) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, &json).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

pub fn read_status(path: &Path) -> Option<SharedServerStatus> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// `<filesDir>/topaz_server_status.json` — same path formula used by both
/// processes (each derives it independently from its own JNI `Context`, but
/// `getFilesDir()` resolves to the same on-disk directory for both since
/// they belong to the same app/UID).
pub fn stats_path(files_dir: &str) -> PathBuf {
    Path::new(files_dir).join(STATS_FILE_NAME)
}
