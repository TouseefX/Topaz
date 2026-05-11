

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::SystemTime;

use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Query},
    routing::{get, post},
};
use base64::prelude::*;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Default, Debug)]
pub struct ServerStats {
    pub luau_requests: AtomicU64,
    pub lua51_requests: AtomicU64,
    pub bytes_in: AtomicU64,
    pub bytes_out: AtomicU64,
    pub errors: AtomicU64,
    pub last_request: Mutex<Option<SystemTime>>,
}

impl ServerStats {
    fn record_request(&self, bytes_in: u64, bytes_out: u64, is_lua51: bool) {
        if is_lua51 {
            self.lua51_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.luau_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.bytes_in.fetch_add(bytes_in, Ordering::Relaxed);
        self.bytes_out.fetch_add(bytes_out, Ordering::Relaxed);
        if let Ok(mut last) = self.last_request.lock() {
            *last = Some(SystemTime::now());
        }
    }
}

#[derive(Clone, Debug)]
pub enum ServerState {
    Stopped,
    Starting { port: u16 },
    Running { addr: String, started_at: std::time::SystemTime },
    Stopping,
    Failed { message: String },
}

impl ServerState {
    pub fn is_running(&self) -> bool {
        matches!(self, ServerState::Running { .. })
    }
    pub fn is_transitional(&self) -> bool {
        matches!(self, ServerState::Starting { .. } | ServerState::Stopping)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub luau: bool,
    pub lua51: bool,
    pub encode_key: u8,
}

pub struct ServerHandle {
    pub state: Arc<Mutex<ServerState>>,
    pub stats: Arc<ServerStats>,
    thread: Option<JoinHandle<()>>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl ServerHandle {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ServerState::Stopped)),
            stats: Arc::new(ServerStats::default()),
            thread: None,
            shutdown: None,
        }
    }

    pub fn reset_stats(&self) {
        self.stats.luau_requests.store(0, Ordering::Relaxed);
        self.stats.lua51_requests.store(0, Ordering::Relaxed);
        self.stats.bytes_in.store(0, Ordering::Relaxed);
        self.stats.bytes_out.store(0, Ordering::Relaxed);
        self.stats.errors.store(0, Ordering::Relaxed);
        if let Ok(mut last) = self.stats.last_request.lock() {
            *last = None;
        }
    }

    pub fn start(&mut self, cfg: ServerConfig, repaint: impl Fn() + Send + Sync + 'static) {
        if self.thread.is_some() {
            return;
        }
        self.reset_stats();
        set_state(&self.state, ServerState::Starting { port: cfg.port });
        repaint();

        let (tx, rx) = oneshot::channel();
        self.shutdown = Some(tx);

        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);
        let repaint: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(repaint);
        let thread = std::thread::Builder::new()
            .name("topaz-server".to_string())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        set_state(&state, ServerState::Failed { message: e.to_string() });
                        repaint();
                        return;
                    }
                };

                rt.block_on(async move {
                    if let Err(e) = run_server(
                        cfg,
                        rx,
                        Arc::clone(&state),
                        Arc::clone(&stats),
                        Arc::clone(&repaint),
                    )
                    .await
                    {
                        set_state(&state, ServerState::Failed { message: e.to_string() });
                        repaint();
                    } else {
                        set_state(&state, ServerState::Stopped);
                        repaint();
                    }
                });
            })
            .expect("failed to spawn server thread");

        self.thread = Some(thread);
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            set_state(&self.state, ServerState::Stopping);
            let _ = tx.send(());
        }

        self.thread.take();
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {

        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        self.thread.take();
    }
}

fn set_state(state: &Arc<Mutex<ServerState>>, next: ServerState) {
    if let Ok(mut s) = state.lock() {
        *s = next;
    }
}

#[derive(Clone)]
struct AppCfg {
    encode_key: u8,
    stats: Arc<ServerStats>,
}

#[derive(Deserialize)]
struct EncodeKeyQuery {
    encode_key: Option<u8>,
}

async fn root() -> &'static str {
    "Topaz decompilation server is running."
}

async fn decompile_luau(
    axum::extract::State(cfg): axum::extract::State<AppCfg>,
    Query(q): Query<EncodeKeyQuery>,
    body: Bytes,
) -> String {
    let key = q.encode_key.unwrap_or(cfg.encode_key);
    let bytes_in = body.len() as u64;
    let bytes = maybe_decode_base64(body.to_vec());
    let out = luau_lifter::decompile_bytecode(&bytes, key);
    cfg.stats.record_request(bytes_in, out.len() as u64, false);
    out
}

async fn decompile_lua51(
    axum::extract::State(cfg): axum::extract::State<AppCfg>,
    body: Bytes,
) -> String {
    let bytes_in = body.len() as u64;
    let bytes = maybe_decode_base64(body.to_vec());
    let out = lua51_lifter::decompile_bytecode(&bytes);
    cfg.stats.record_request(bytes_in, out.len() as u64, true);
    out
}

fn maybe_decode_base64(bytes: Vec<u8>) -> Vec<u8> {
    BASE64_STANDARD.decode(&bytes).unwrap_or(bytes)
}

async fn run_server(
    cfg: ServerConfig,
    shutdown_rx: oneshot::Receiver<()>,
    state: Arc<Mutex<ServerState>>,
    stats: Arc<ServerStats>,
    repaint: Arc<dyn Fn() + Send + Sync + 'static>,
) -> std::io::Result<()> {
    let app_cfg = AppCfg {
        encode_key: cfg.encode_key,
        stats: Arc::clone(&stats),
    };

    let mut app = Router::new().route("/", get(root));
    if cfg.luau {
        app = app.route("/luau/decompile", post(decompile_luau));
    }
    if cfg.lua51 {
        app = app.route("/lua51/decompile", post(decompile_lua51));
    }
    let app = app
        .with_state(app_cfg)
        .layer(DefaultBodyLimit::disable());

    let listener = TcpListener::bind(format!("0.0.0.0:{}", cfg.port)).await?;
    let addr = listener.local_addr()?.to_string();

    set_state(&state, ServerState::Running {
        addr: addr.clone(),
        started_at: std::time::SystemTime::now(),
    });
    repaint();

    axum::serve(listener, app)
        .with_graceful_shutdown(wait_for_shutdown(shutdown_rx))
        .await?;
    Ok(())
}

async fn wait_for_shutdown(shutdown_rx: oneshot::Receiver<()>) {
    let _ = shutdown_rx.await;
}
