//! JNI entry points called from `KeepAliveService.java`, which — per the
//! `process = ":service"` line in Cargo.toml — runs in a separate Linux
//! process from the UI. `android_main` / eframe / winit never run here.
//!
//! That split is the whole point: winit can only create one `EventLoop` per
//! process, so when the Activity is destroyed and recreated the UI process
//! has to be free to restart from scratch (see `android_main` in lib.rs)
//! without taking the running HTTP server down with it. This module is what
//! keeps the server alive through that — it owns the actual `ServerHandle`,
//! independent of any Activity/UI lifecycle, and hands stats back to
//! whichever process wants to display them via a small JSON file (see
//! `ipc_stats.rs`), since two OS processes no longer share memory.
#![cfg(target_os = "android")]

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint};
use log::{error, info};

use crate::ipc_stats::{self, SharedServerStatus, SharedState};
use crate::server::{ServerConfig, ServerHandle, ServerState};

static SERVER: OnceLock<Mutex<ServerHandle>> = OnceLock::new();
static STATS_PATH: OnceLock<PathBuf> = OnceLock::new();
static WATCHER_STARTED: OnceLock<()> = OnceLock::new();

fn server() -> &'static Mutex<ServerHandle> {
    SERVER.get_or_init(|| Mutex::new(ServerHandle::new()))
}

/// Every second, snapshot the real `ServerHandle` state into the shared JSON
/// file so the UI process (if one happens to be running) can display it.
/// Started once, the first time the server is started; keeps running for
/// the lifetime of this process regardless of start/stop so a "Stopped"
/// snapshot still gets written out after a stop.
fn spawn_status_watcher() {
    if WATCHER_STARTED.set(()).is_err() {
        return; // already running
    }
    std::thread::Builder::new()
        .name("topaz-status-watcher".into())
        .spawn(|| {
            loop {
                if let Some(path) = STATS_PATH.get() {
                    if let Ok(handle) = server().lock() {
                        let snapshot = snapshot_status(&handle);
                        ipc_stats::write_status(path, &snapshot);
                    }
                }
                std::thread::sleep(Duration::from_millis(1000));
            }
        })
        .expect("failed to spawn status watcher thread");
}

fn snapshot_status(handle: &ServerHandle) -> SharedServerStatus {
    let state = handle
        .state
        .lock()
        .map(|s| match &*s {
            ServerState::Stopped => SharedState::Stopped,
            ServerState::Starting { port } => SharedState::Starting { port: *port },
            ServerState::Running { addr, started_at } => SharedState::Running {
                addr: addr.clone(),
                started_at_ms: started_at
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            },
            ServerState::Stopping => SharedState::Stopping,
            ServerState::Failed { message } => SharedState::Failed {
                message: message.clone(),
            },
        })
        .unwrap_or(SharedState::Stopped);

    let last_request_ms = handle
        .stats
        .last_request
        .lock()
        .ok()
        .and_then(|g| *g)
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);

    SharedServerStatus {
        state,
        luau_requests: handle.stats.luau_requests.load(Ordering::Relaxed),
        lua51_requests: handle.stats.lua51_requests.load(Ordering::Relaxed),
        bytes_in: handle.stats.bytes_in.load(Ordering::Relaxed),
        bytes_out: handle.stats.bytes_out.load(Ordering::Relaxed),
        errors: handle.stats.errors.load(Ordering::Relaxed),
        last_request_ms,
        updated_ms: ipc_stats::now_ms(),
    }
}

fn jstring_to_string(env: &mut JNIEnv, s: &JString) -> String {
    env.get_string(s).map(|s| s.into()).unwrap_or_default()
}

/// Called from `KeepAliveService.onStartCommand` when the Intent carries a
/// "start_server" command. `files_dir` is `Context.getFilesDir().getPath()`,
/// passed down from Java since this process's JNI Context is the Service,
/// not an Activity.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_exec_topaz_KeepAliveService_nativeStartServer(
    mut env: JNIEnv,
    _class: JClass,
    port: jint,
    luau: jboolean,
    lua51: jboolean,
    encode_key: jint,
    files_dir: JString,
) {
    let files_dir = jstring_to_string(&mut env, &files_dir);
    let _ = STATS_PATH.set(ipc_stats::stats_path(&files_dir));
    spawn_status_watcher();

    let cfg = ServerConfig {
        port: port as u16,
        luau: luau != 0,
        lua51: lua51 != 0,
        encode_key: encode_key as u8,
    };

    match server().lock() {
        // No egui context exists in this process, so there's nothing useful
        // to repaint — the status-watcher thread above is this process's
        // equivalent of a repaint: it's what tells the UI process something
        // changed.
        Ok(mut handle) => {
            info!("service process: starting server on port {}", cfg.port);
            handle.start(cfg, || {});
        }
        Err(e) => error!("service process: server mutex poisoned: {e}"),
    }
}

/// Called from `KeepAliveService.onStartCommand` when the Intent carries a
/// "stop_server" command, and from `onDestroy` as a safety net.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_exec_topaz_KeepAliveService_nativeStopServer(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(server) = SERVER.get() {
        if let Ok(mut handle) = server.lock() {
            info!("service process: stopping server");
            handle.stop();
        }
    }
    if let Some(path) = STATS_PATH.get() {
        let snapshot = SharedServerStatus {
            state: SharedState::Stopped,
            updated_ms: ipc_stats::now_ms(),
            ..Default::default()
        };
        ipc_stats::write_status(path, &snapshot);
    }
}
