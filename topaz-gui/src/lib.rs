use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

use cfg::CfgSnapshot;

pub mod cfg_view;
pub mod ipc_stats;
pub mod persist;
pub mod server;

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "android")]
pub mod service_entry;

use cfg_view::CfgViewState;
use server::{ServerConfig, ServerHandle, ServerState};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

enum DecompileMsg {
    Ok {
        source: String,
        cfgs: Vec<CfgSnapshot>,
    },
    Err(String),
}

const ENCODE_KEY_TOOLTIP: &str = "\
Luau shuffles opcodes by multiplying each byte by an 8-bit key. The decompiler \
multiplies them back, so this number MUST match the key used when the bytecode \
was produced.\n\
\n\
Common values:\n\
  • 1   — vanilla Luau and most custom bytecode (no shuffle)\n\
  • 203 — Roblox-style bytecode (the CLI default)\n\
\n\
If decompilation produces garbage or panics, try the other one.";

const PORT_TOOLTIP: &str = "\
TCP port the local server listens on.\n\
\n\
Recommended:\n\
  • 3000, 8000, 8080, 8888 — common dev ports\n\
\n\
Avoid:\n\
  • Below 1024 — these require admin / root.\n\
  • 22, 80, 443, 5432, 6379, 3306 — already used by SSH / HTTP(S) / databases.\n\
\n\
If the chosen port is busy, starting the server will fail with \"address in use\".";

fn detect_format(bytes: &[u8]) -> Option<BytecodeFormat> {
    if bytes.starts_with(b"\x1BLua") {
        return Some(BytecodeFormat::Lua51);
    }
    match bytes.first() {
        Some(&0) => Some(BytecodeFormat::Luau),
        Some(b) if (4..=9).contains(b) => Some(BytecodeFormat::Luau),
        _ => None,
    }
}

fn dump_luau_cfgs_for_gui(bytecode: &[u8], encode_key: u8) -> Vec<CfgSnapshot> {
    luau_lifter::dump_cfgs_default(bytecode, encode_key)
}

fn port_hint(port: u16) -> &'static str {
    match port {
        0 => "0 = OS picks any free port.",
        1..=1023 => "Privileged port — needs admin/root.",
        3000 | 8000 | 8080 | 8888 => "Common dev port — good choice.",
        _ => "",
    }
}

/// Entry point for desktop binary
pub fn run_desktop() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 680.0])
            .with_min_inner_size([640.0, 440.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Topaz",
        options,
        Box::new(|cc| Ok(Box::new(TopazApp::new(cc)))),
    )
}

/// Android entry point. eframe/winit on Android requires android_app passed via NativeOptions.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    // android_main can be invoked more than once in the same process: if our
    // foreground service (KeepAliveService, now in its own :service process —
    // see ipc_stats.rs / service_entry.rs) kept this process alive after the
    // Activity was destroyed (e.g. swiped from Recents), reopening the app
    // creates a new Activity and the system calls android_main again here.
    // winit can only ever create one EventLoop per process. In practice a
    // second attempt doesn't cleanly return an Err we can catch (see the
    // run_native error branch below) — it hangs indefinitely before logging
    // anything, and the framework eventually gives up waiting on the new
    // Activity's window ("top resumed state loss timeout" in logcat), with
    // nothing ever reaching logcat from our side. So: bail out immediately,
    // before calling into winit at all, rather than hoping run_native errors.
    static ALREADY_RAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if ALREADY_RAN.swap(true, std::sync::atomic::Ordering::SeqCst) {
        std::process::exit(1);
    }

    // Setup Android logger so `log` macros and `println!` go to logcat
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "full");
    }
    log::info!("Topaz starting on Android");

    // Store globally for permission/clipboard helpers. Ask for permissions only
    // after a user action (Open File / Grant), not while the app is starting.
    crate::android::init_android_app(app.clone());

    let mut options = eframe::NativeOptions::default();
    // Critical: pass AndroidApp to winit
    options.android_app = Some(app);
    // Prefer Glow for Android (more mature), but allow wgpu fallback
    options.renderer = eframe::Renderer::Glow;
    // Make UI more usable on small screens
    options.viewport = egui::ViewportBuilder::default()
        .with_inner_size([1080.0, 1920.0])
        .with_min_inner_size([320.0, 480.0]);

    // eframe::run_native never returns on Android, it runs the event loop
    if let Err(e) = eframe::run_native(
        "Topaz",
        options,
        Box::new(|cc| {
            // ndk_context should already be initialized by winit, but ensure
            Ok(Box::new(TopazApp::new(cc)))
        }),
    ) {
        // winit can only ever create one EventLoop per process (see
        // https://github.com/rust-windowing/winit/issues/3325). If our
        // foreground service kept this process alive after the Activity was
        // destroyed (e.g. swiped from Recents) and the user reopens the app,
        // android_main runs again and this second run_native() call fails
        // with "RecreationAttempt" — nothing gets drawn and the OS splash
        // hangs forever. Self-terminate so the next launch gets a clean
        // process instead of requiring a manual force-stop.
        log::error!("eframe run_native error: {e:?} — restarting process");
        std::process::exit(1);
    }
}

#[derive(PartialEq, Clone, Copy)]
enum BytecodeFormat {
    Luau,
    Lua51,
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Decompile,
    Server,
    Settings,
}

#[derive(PartialEq, Clone, Copy)]
enum SubTab {
    Source,
    Cfg,
    Hex,
}

pub struct TopazApp {
    tab: Tab,

    sub_tab: SubTab,
    format: BytecodeFormat,
    encode_key: u8,
    encode_key_text: String,
    input_path: Option<PathBuf>,
    bytecode: Option<Vec<u8>>,
    raw_output: String,
    output: String,
    status: String,
    status_is_error: bool,
    decompile_rx: Option<mpsc::Receiver<DecompileMsg>>,
    decompile_started: Option<Instant>,

    add_watermark: bool,
    show_upvalue_comments: bool,
    theme: egui::ThemePreference,
    last_applied_theme: Option<egui::ThemePreference>,

    cfgs: Vec<CfgSnapshot>,
    selected_function: usize,
    cfg_view: CfgViewState,

    search_query: String,
    search_case_sensitive: bool,
    search_context: u32,

    hex_jump_text: String,
    hex_jump_to_row: Option<usize>,

    #[cfg(not(target_os = "android"))]
    server: ServerHandle,
    // On Android the server actually runs in KeepAliveService's separate
    // `:service` process (see service_entry.rs) so the UI process can freely
    // die/restart without taking it down. We control it via Intents and
    // read its state by polling the small JSON file it writes instead of
    // holding a live Arc to it.
    #[cfg(target_os = "android")]
    android_server_status: ipc_stats::SharedServerStatus,
    #[cfg(target_os = "android")]
    android_stats_path: Option<PathBuf>,
    #[cfg(target_os = "android")]
    android_last_poll: Option<Instant>,
    server_port_text: String,
    server_port: u16,
    server_luau: bool,
    server_lua51: bool,
    server_encode_key: u8,
    server_encode_key_text: String,

    // Android-specific: manual path input because rfd dialogs don't exist on Android
    android_manual_path: String,

    // --- Android enhancements ---
    keep_alive: bool,
    #[cfg(target_os = "android")]
    file_browser_open: bool,
    #[cfg(target_os = "android")]
    file_browser: crate::android::FileBrowser,
    #[cfg(target_os = "android")]
    permission_banner_dismissed: bool,
}

impl TopazApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Load persisted state
        let saved = persist::load();
        let saved_theme = match saved.theme.as_str() {
            "Dark" => egui::ThemePreference::Dark,
            "Light" => egui::ThemePreference::Light,
            _ => egui::ThemePreference::System,
        };

        // If keep_alive was on, restore it now (before the app runs)
        #[cfg(target_os = "android")]
        if saved.keep_alive {
            crate::android::enable_keepalive();
        }

        Self {
            tab: Tab::Decompile,
            sub_tab: SubTab::Source,
            format: BytecodeFormat::Luau,
            encode_key: saved.encode_key,
            encode_key_text: format!("{}", saved.encode_key),
            input_path: None,
            bytecode: None,
            raw_output: String::new(),
            output: String::new(),
            status: if cfg!(target_os = "android") {
                "Ready on Android — tap Open File → Browse, grant Files permission if asked.".to_string()
            } else {
                "Ready — open a bytecode file to begin.".to_string()
            },
            status_is_error: false,
            decompile_rx: None,
            decompile_started: None,
            add_watermark: saved.add_watermark,
            show_upvalue_comments: saved.show_upvalue_comments,
            theme: saved_theme,
            last_applied_theme: None,
            cfgs: Vec::new(),
            selected_function: 0,
            cfg_view: CfgViewState::default(),
            search_query: String::new(),
            search_case_sensitive: false,
            search_context: 0,
            hex_jump_text: String::new(),
            hex_jump_to_row: None,
            #[cfg(not(target_os = "android"))]
            server: ServerHandle::new(),
            #[cfg(target_os = "android")]
            android_server_status: ipc_stats::SharedServerStatus::default(),
            #[cfg(target_os = "android")]
            android_stats_path: None,
            #[cfg(target_os = "android")]
            android_last_poll: None,
            server_port_text: format!("{}", saved.server_port),
            server_port: saved.server_port,
            server_luau: saved.server_luau,
            server_lua51: saved.server_lua51,
            server_encode_key: saved.server_encode_key,
            server_encode_key_text: format!("{}", saved.server_encode_key),
            // Default Android paths that users often have
            android_manual_path: if cfg!(target_os = "android") {
                "/sdcard/Download/sample.luac".to_string()
            } else {
                String::new()
            },
            keep_alive: saved.keep_alive,
            #[cfg(target_os = "android")]
            file_browser_open: false,
            #[cfg(target_os = "android")]
            file_browser: crate::android::FileBrowser::new("/sdcard/Download"),
            #[cfg(target_os = "android")]
            permission_banner_dismissed: false,
        }
    }

    #[cfg(not(target_os = "android"))]
    fn pick_file(&mut self) {
        let dialog = rfd::FileDialog::new().set_title("Open bytecode file");
        if let Some(path) = dialog.pick_file() {
            self.load_file(path);
        }
    }

    #[cfg(target_os = "android")]
    fn pick_file(&mut self) {
        if !crate::android::direct_file_access_granted() {
            self.status = "Enable All files access, then press Open File again.".into();
            self.status_is_error = true;
            crate::android::request_storage_permissions_async();
            return;
        }

        self.file_browser.refresh();
        self.file_browser_open = true;
        self.status = "Opening Android file browser…".to_string();
        self.status_is_error = false;
    }

    fn load_lua51_sample(&mut self) {
        const SAMPLE: &[u8] = include_bytes!("../../sample.luac");
        self.write_and_load_sample(SAMPLE, "topaz_sample.luac");
    }

    fn load_luau_sample(&mut self) {
        const SAMPLE: &[u8] = include_bytes!("../../sample.luau.bin");
        self.write_and_load_sample(SAMPLE, "topaz_sample.luau.bin");
    }

    fn write_and_load_sample(&mut self, bytes: &[u8], name: &str) {
        let mut path = std::env::temp_dir();
        path.push(name);
        match std::fs::write(&path, bytes) {
            Ok(()) => self.load_file(path),
            Err(e) => {
                self.status = format!("Failed to write sample file: {e}");
                self.status_is_error = true;
            }
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        match std::fs::read(&path) {
            Ok(bytes) => {
                let detected = detect_format(&bytes);
                let mut status = format!("Loaded: {} ({} bytes)", path.display(), bytes.len());
                if let Some(f) = detected {
                    if self.format != f {
                        self.format = f;
                        let name = match f {
                            BytecodeFormat::Lua51 => "Lua 5.1",
                            BytecodeFormat::Luau => "Luau",
                        };
                        status.push_str(&format!(" · auto-detected {name}"));
                    }
                } else {
                    status.push_str(" · format unknown — pick Luau or Lua 5.1 manually");
                }
                self.status = status;
                self.status_is_error = false;
                self.bytecode = Some(bytes);
                self.input_path = Some(path.clone());
                self.output.clear();
                self.cfgs.clear();
                self.cfg_view.reset();
                #[cfg(target_os = "android")]
                {
                    self.android_manual_path = path.display().to_string();
                }
            }
            Err(e) => {
                self.status = format!("Failed to read file: {e}");
                self.status_is_error = true;
                #[cfg(target_os = "android")]
                crate::android::toast(&format!("Read failed: {e}"));
            }
        }
    }

    fn load_from_manual_path(&mut self) {
        let p = self.android_manual_path.trim();
        if p.is_empty() {
            self.status = "Enter a file path first.".to_string();
            self.status_is_error = true;
            return;
        }
        self.load_file(PathBuf::from(p));
    }

    fn decompile(&mut self, ctx: &egui::Context) {
        if self.decompile_rx.is_some() {
            return;
        }
        let Some(bytecode) = self.bytecode.clone() else {
            self.status = "No file loaded.".to_string();
            self.status_is_error = true;
            return;
        };

        let format = self.format;
        let encode_key = self.encode_key;
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();

        std::thread::Builder::new()
            .name("topaz-decompile".to_string())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let source = match format {
                        BytecodeFormat::Lua51 => lua51_lifter::decompile_bytecode(&bytecode),
                        BytecodeFormat::Luau => {
                            luau_lifter::decompile_bytecode_default(&bytecode, encode_key)
                        }
                    };

                    let cfgs = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match format {
                        BytecodeFormat::Lua51 => lua51_lifter::dump_cfgs(&bytecode),
                        BytecodeFormat::Luau => dump_luau_cfgs_for_gui(&bytecode, encode_key),
                    }))
                    .unwrap_or_default();
                    (source, cfgs)
                }));
                let msg = match result {
                    Ok((source, cfgs)) => DecompileMsg::Ok { source, cfgs },
                    Err(e) => DecompileMsg::Err(panic_message(&e)),
                };
                let _ = tx.send(msg);
                ctx.request_repaint();
            })
            .expect("failed to spawn decompile thread");

        self.decompile_rx = Some(rx);
        self.decompile_started = Some(Instant::now());
        self.output.clear();
        self.status = "Decompiling…".to_string();
        self.status_is_error = false;
    }

    fn poll_decompile(&mut self) {
        let Some(rx) = &self.decompile_rx else { return };
        match rx.try_recv() {
            Ok(DecompileMsg::Ok { source, cfgs }) => {
                let elapsed = self.decompile_started.map(|t| t.elapsed()).unwrap_or_default();
                self.raw_output = source;
                self.apply_output_settings();
                self.cfgs = cfgs;
                self.selected_function = 0;
                self.cfg_view.reset();
                self.status = format!(
                    "Decompilation successful ({:.2}s, {} function{}).",
                    elapsed.as_secs_f64(),
                    self.cfgs.len(),
                    if self.cfgs.len() == 1 { "" } else { "s" }
                );
                self.status_is_error = false;
                self.decompile_rx = None;
                self.decompile_started = None;
            }
            Ok(DecompileMsg::Err(msg)) => {
                self.raw_output.clear();
                self.output.clear();
                self.cfgs.clear();
                self.cfg_view.reset();
                self.status = format!("Decompilation failed: {msg}");
                self.status_is_error = true;
                self.decompile_rx = None;
                self.decompile_started = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status = "Decompile worker disconnected unexpectedly.".to_string();
                self.status_is_error = true;
                self.decompile_rx = None;
                self.decompile_started = None;
            }
        }
    }

    fn apply_output_settings(&mut self) {
        if self.raw_output.is_empty() {
            self.output.clear();
            return;
        }
        let mut out = String::with_capacity(self.raw_output.len() + 64);
        if self.add_watermark {
            out.push_str("-- Decompiled with Topaz\n\n");
        }
        if self.show_upvalue_comments {
            out.push_str(&self.raw_output);
        } else {
            for line in self.raw_output.lines() {
                if line.trim_start().starts_with("-- upvalues:") {
                    continue;
                }
                out.push_str(line);
                out.push('\n');
            }
            if out.ends_with('\n') {
                out.pop();
            }
        }
        self.output = out;
    }

    fn is_decompiling(&self) -> bool {
        self.decompile_rx.is_some()
    }

    #[cfg(not(target_os = "android"))]
    fn save_output(&mut self) {
        if self.output.is_empty() {
            self.status = "Nothing to save.".to_string();
            self.status_is_error = true;
            return;
        }

        let dialog = rfd::FileDialog::new()
            .set_title("Save decompiled output")
            .add_filter("Lua source", &["lua"])
            .set_file_name("decompiled.lua");

        if let Some(path) = dialog.save_file() {
            match std::fs::write(&path, &self.output) {
                Ok(()) => {
                    self.status = format!("Saved to: {}", path.display());
                    self.status_is_error = false;
                }
                Err(e) => {
                    self.status = format!("Failed to save: {e}");
                    self.status_is_error = true;
                }
            }
        }
    }

    #[cfg(target_os = "android")]
    fn save_output(&mut self) {
        if self.output.is_empty() {
            self.status = "Nothing to save.".to_string();
            self.status_is_error = true;
            return;
        }
        if !crate::android::direct_file_access_granted() {
            self.status = "Enable All files access, then press Save Output again.".into();
            self.status_is_error = true;
            crate::android::open_app_settings();
            return;
        }

        // Prefer a user-visible location. The old code tried /data/data first,
        // so it always succeeded there and never attempted Downloads.
        let public = [
            "/sdcard/Download/decompiled.lua",
            "/storage/emulated/0/Download/decompiled.lua",
            "/storage/emulated/0/Documents/decompiled.lua",
        ];
        let app_private = [
            "/data/data/com.exec.topaz/files/decompiled.lua",
        ];

        for cand in public.into_iter().chain(app_private.into_iter()) {
            if let Some(parent) = std::path::Path::new(cand).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(cand, &self.output) {
                Ok(()) => {
                    self.status = format!("Saved to: {cand}");
                    self.status_is_error = false;
                    crate::android::toast("Saved ✓");
                    return;
                }
                Err(e) => {
                    log::warn!("save failed {cand}: {e}");
                    continue;
                }
            }
        }
        // Fallback: temp dir
        let mut p = std::env::temp_dir();
        p.push("decompiled.lua");
        match std::fs::write(&p, &self.output) {
            Ok(()) => {
                self.status = format!("Saved to temp: {}", p.display());
                self.status_is_error = false;
                crate::android::toast("Saved to cache");
            }
            Err(e) => {
                self.status = format!("Failed to save on Android: {e} — grant Files permission in Settings");
                self.status_is_error = true;
                crate::android::toast("Save failed — check permissions");
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    fn current_server_state(&self) -> ServerState {
        self.server
            .state
            .lock()
            .map(|s| s.clone())
            .unwrap_or(ServerState::Stopped)
    }

    #[cfg(target_os = "android")]
    fn current_server_state(&self) -> ServerState {
        match &self.android_server_status.state {
            ipc_stats::SharedState::Stopped => ServerState::Stopped,
            ipc_stats::SharedState::Starting { port } => ServerState::Starting { port: *port },
            ipc_stats::SharedState::Running { addr, started_at_ms } => ServerState::Running {
                addr: addr.clone(),
                started_at: std::time::UNIX_EPOCH
                    + std::time::Duration::from_millis(*started_at_ms),
            },
            ipc_stats::SharedState::Stopping => ServerState::Stopping,
            ipc_stats::SharedState::Failed { message } => {
                ServerState::Failed { message: message.clone() }
            }
        }
    }

    /// Android only: re-read the status file KeepAliveService's `:service`
    /// process writes roughly once a second. Cheap enough to call every
    /// frame; throttled anyway so it's at most a few reads/sec while the
    /// Server tab is open and repainting.
    #[cfg(target_os = "android")]
    fn poll_android_server_status(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.android_last_poll {
            if now.duration_since(last) < std::time::Duration::from_millis(400) {
                return;
            }
        }
        self.android_last_poll = Some(now);

        let path = match &self.android_stats_path {
            Some(p) => p.clone(),
            None => match crate::android::files_dir() {
                Some(dir) => {
                    let p = ipc_stats::stats_path(&dir);
                    self.android_stats_path = Some(p.clone());
                    p
                }
                None => return,
            },
        };

        if let Some(status) = ipc_stats::read_status(&path) {
            self.android_server_status = status;
        }
    }

    // Unified clipboard copy that works on desktop + Android
    fn copy_text_system(&self, text: &str, ctx: &egui::Context) -> String {
        #[cfg(target_os = "android")]
        {
            if crate::android::copy_to_clipboard(text) {
                return "Copied to system clipboard.".into();
            } else {
                // fallback to egui internal
                ctx.copy_text(text.to_owned());
                return "Copied to in-app clipboard (system failed — see logcat).".into();
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            ctx.copy_text(text.to_owned());
            "Copied to clipboard.".into()
        }
    }

    /// Persist current settings to disk so they survive close/reopen.
    fn save_state(&self) {
        let theme_str = match self.theme {
            egui::ThemePreference::Dark => "Dark",
            egui::ThemePreference::Light => "Light",
            _ => "System",
        };
        persist::save(&persist::SavedState {
            theme: theme_str.into(),
            add_watermark: self.add_watermark,
            show_upvalue_comments: self.show_upvalue_comments,
            server_port: self.server_port,
            server_luau: self.server_luau,
            server_lua51: self.server_lua51,
            server_encode_key: self.server_encode_key,
            encode_key: self.encode_key,
            keep_alive: self.keep_alive,
        });
    }
}

impl eframe::App for TopazApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_applied_theme != Some(self.theme) {
            ctx.set_theme(self.theme);
            self.last_applied_theme = Some(self.theme);
            self.save_state();
        }

        self.poll_decompile();

        // Android: pump system clipboard into egui if user pasted externally?
        // eframe doesn't do this automatically – we do manual paste buttons instead.

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(6.0);
            let state = self.current_server_state();
            ui.horizontal(|ui| {
                ui.heading("Topaz");
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Luau / Lua 5.1 decompiler").weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    server_status_badge(ui, &state);
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Decompile, "  Decompile  ");
                ui.add_space(2.0);
                ui.selectable_value(&mut self.tab, Tab::Server, "  Server  ");
                ui.add_space(2.0);
                ui.selectable_value(&mut self.tab, Tab::Settings, "  Settings  ");
            });
            ui.add_space(4.0);
        });

        // Android permission banner
        #[cfg(target_os = "android")]
        if !self.permission_banner_dismissed {
            let perm = crate::android::permission_status();
            let needs_perm = !(perm.manage_external || perm.storage_granted || perm.media_images || perm.media_video || perm.media_audio);
            let first_check = perm.checked_at.is_none();
            if needs_perm || first_check {
                egui::TopBottomPanel::top("perm_banner").show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 200, 90), "⚠ Files permission needed");
                        ui.label(perm.last_message.clone());
                        if ui.button("Grant").clicked() {
                            crate::android::request_storage_permissions_async();
                        }
                        if ui.button("Open Settings").clicked() {
                            crate::android::open_app_settings();
                        }
                        if ui.small_button("✕").clicked() {
                            self.permission_banner_dismissed = true;
                        }
                    });
                    ui.add_space(4.0);
                });
            }
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add_space(2.0);
            let (msg, is_err) = match self.tab {
                Tab::Decompile => (self.status.clone(), self.status_is_error),
                Tab::Server => match self.current_server_state() {
                    ServerState::Stopped => ("Server stopped.".into(), false),
                    ServerState::Starting { port } => (format!("Starting on port {port}…"), false),
                    ServerState::Running { addr, .. } => (format!("Server running at http://{addr}"), false),
                    ServerState::Stopping => ("Stopping server…".into(), false),
                    ServerState::Failed { message } => (format!("Server error: {message}"), true),
                },
                Tab::Settings => ("Settings apply to the next render of the source view.".into(), false),
            };
            let color = if is_err {
                egui::Color32::from_rgb(220, 80, 80)
            } else {
                ui.visuals().text_color()
            };
            ui.colored_label(color, msg);
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Decompile => self.show_decompile_tab(ui),
            Tab::Server => self.show_server_tab(ui, ctx),
            Tab::Settings => self.show_settings_tab(ui),
        });

        // Android file browser modal
        #[cfg(target_os = "android")]
        if self.file_browser_open {
            let mut open = self.file_browser_open;
            egui::Window::new("📂 Open bytecode file")
                .open(&mut open)
                .resizable(true)
                .default_width(520.0)
                .default_height(520.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    if let Some(picked) = self.file_browser.ui(ui) {
                        self.load_file(picked);
                        self.file_browser_open = false;
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Use this folder's path").clicked() {
                            self.android_manual_path = self.file_browser.current_path().display().to_string();
                        }
                        ui.add_enabled(
                            false,
                            egui::Button::new("System picker requires Activity callback"),
                        )
                        .on_disabled_hover_text(
                            "NativeActivity cannot receive the selected content URI yet; use this browser with All files access.",
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.file_browser_open = false;
                            }
                        });
                    });
                });
            self.file_browser_open = open;
        }

        if self.current_server_state().is_transitional() || self.is_decompiling() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

impl TopazApp {
    fn show_decompile_tab(&mut self, ui: &mut egui::Ui) {
        // Android-specific manual path bar on top — now with Paste + Browse
        #[cfg(target_os = "android")]
        {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Path:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.android_manual_path)
                            .desired_width(ui.available_width() - 260.0)
                            .hint_text("/sdcard/Download/file.luac"),
                    );
                    if ui.button("📋 Paste").clicked() {
                        if let Some(t) = crate::android::paste_from_clipboard() {
                            self.android_manual_path = t;
                            self.status = "Pasted from clipboard".into();
                            self.status_is_error = false;
                        } else {
                            self.status = "Clipboard empty or permission denied".into();
                            self.status_is_error = true;
                        }
                    }
                    if ui.button("📂 Browse…").clicked() {
                        self.pick_file();
                    }
                    if ui.button("Load File").clicked() {
                        self.load_from_manual_path();
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.weak("Tip: put .luac in /sdcard/Download, then Browse → tap file.");
                    if ui.small_button("Fix permissions").clicked() {
                        crate::android::request_storage_permissions_async();
                    }
                    let perm = crate::android::permission_status();
                    let ok = perm.manage_external || perm.storage_granted || perm.media_images;
                    ui.colored_label(
                        if ok { egui::Color32::from_rgb(90, 200, 120) } else { egui::Color32::from_rgb(220, 160, 60) },
                        if ok { "✓ storage OK" } else { "⚠ no storage perm" }
                    );
                });
            });
            ui.add_space(4.0);
        }

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Open File…").clicked() {
                    self.pick_file();
                }
                let mut load_lua51 = false;
                let mut load_luau = false;
                ui.menu_button("Load sample ▾", |ui| {
                    if ui.button("Lua 5.1 (sample.luac)").clicked() {
                        load_lua51 = true;
                        ui.close_menu();
                    }
                    if ui.button("Luau (sample.luau)").clicked() {
                        load_luau = true;
                        ui.close_menu();
                    }
                });
                if load_lua51 {
                    self.load_lua51_sample();
                }
                if load_luau {
                    self.load_luau_sample();
                }
                ui.separator();
                ui.label("Format:");
                ui.radio_value(&mut self.format, BytecodeFormat::Luau, "Luau");
                ui.radio_value(&mut self.format, BytecodeFormat::Lua51, "Lua 5.1");

                if self.format == BytecodeFormat::Luau {
                    ui.separator();
                    ui.label("Encode key:")
                        .on_hover_text(ENCODE_KEY_TOOLTIP);
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.encode_key_text)
                                .desired_width(48.0),
                        )
                        .on_hover_text(ENCODE_KEY_TOOLTIP)
                        .changed()
                    {
                        if let Ok(v) = self.encode_key_text.parse::<u8>() {
                            self.encode_key = v;
                        }
                    }
                    ui.weak("(try 1, or 203 for Roblox)")
                        .on_hover_text(ENCODE_KEY_TOOLTIP);
                }

                ui.separator();
                let busy = self.is_decompiling();
                let can_decompile = self.bytecode.is_some() && !busy;
                let label = if busy { "Decompiling…" } else { "▶  Decompile" };
                if ui
                    .add_enabled(can_decompile, egui::Button::new(label))
                    .clicked()
                {
                    let ctx_clone = ui.ctx().clone();
                    self.decompile(&ctx_clone);
                }
                if busy {
                    ui.spinner();
                }
                if ui
                    .add_enabled(!self.output.is_empty(), egui::Button::new("Save Output…"))
                    .clicked()
                {
                    self.save_output();
                }
                if ui
                    .add_enabled(!self.output.is_empty(), egui::Button::new("📋 Copy"))
                    .clicked()
                {
                    let msg = self.copy_text_system(&self.output, ui.ctx());
                    self.status = msg;
                    self.status_is_error = false;
                }
                #[cfg(target_os = "android")]
                if ui
                    .add_enabled(!self.output.is_empty(), egui::Button::new("📤 Share…"))
                    .clicked()
                {
                    if crate::android::share_text(&self.output) {
                        self.status = "Opened Android share sheet.".into();
                        self.status_is_error = false;
                    } else {
                        self.status = "Could not open Android share sheet.".into();
                        self.status_is_error = true;
                    }
                }
            });
        });

        ui.add_space(4.0);

        if let Some(path) = &self.input_path {
            let path_str = path.display().to_string();
            let byte_count = self.bytecode.as_ref().map(|b| b.len());

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("File:").strong());
                ui.monospace(&path_str);
                if let Some(len) = byte_count {
                    ui.weak(format!("· {} bytes", len));
                }
                #[cfg(target_os = "android")]
                if ui.small_button("📋 copy path").clicked() {
                    self.status = self.copy_text_system(&path_str, ui.ctx());
                    self.status_is_error = false;
                }
            });
            ui.add_space(4.0);
        }

        if self.output.is_empty() && self.cfgs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Open a bytecode file and click Decompile to see source, CFG, and hex view.",
                    )
                    .weak(),
                );
            });
            return;
        }

        ui.separator();

        egui::SidePanel::left("functions_sidebar")
            .resizable(true)
            .default_width(200.0)
            .width_range(140.0..=320.0)
            .show_inside(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Functions").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak(format!("{}", self.cfgs.len()));
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    if self.cfgs.is_empty() {
                        ui.add_space(8.0);
                        ui.weak("(no CFGs available)");
                    } else {
                        for (i, c) in self.cfgs.iter().enumerate() {
                            let selected = i == self.selected_function;
                            let label = format!("{}  · {} blocks", c.name, c.nodes.len());
                            if ui.selectable_label(selected, label).clicked() {
                                self.selected_function = i;
                                self.cfg_view.reset();
                            }
                        }
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.selectable_value(&mut self.sub_tab, SubTab::Source, "  Source  ");
                    ui.add_space(2.0);
                    ui.selectable_value(&mut self.sub_tab, SubTab::Cfg, "  CFG  ");
                    ui.add_space(2.0);
                    ui.selectable_value(&mut self.sub_tab, SubTab::Hex, "  Hex  ");
                });
                ui.separator();
                ui.add_space(2.0);

                match self.sub_tab {
                    SubTab::Source => self.show_source_subtab(ui),
                    SubTab::Cfg => self.show_cfg_subtab(ui),
                    SubTab::Hex => self.show_hex_subtab(ui),
                }
            });
    }

    fn show_source_subtab(&mut self, ui: &mut egui::Ui) {
        if self.output.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.weak("No source yet — click Decompile.");
            });
            return;
        }

        let total_lines = self.output.lines().count();
        let match_lines = if self.search_query.is_empty() {
            None
        } else {
            Some(count_matches(
                &self.output,
                &self.search_query,
                self.search_case_sensitive,
            ))
        };

        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label("Find:");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .desired_width(220.0)
                    .hint_text("filter lines…"),
            );
            #[cfg(target_os = "android")]
            if ui.small_button("📋 Paste").clicked() {
                if let Some(t) = crate::android::paste_from_clipboard() {
                    self.search_query = t;
                }
            }
            ui.add_space(6.0);
            ui.checkbox(&mut self.search_case_sensitive, "Aa")
                .on_hover_text("Case-sensitive search");
            ui.add_space(6.0);
            ui.label("Context:");
            ui.add(
                egui::DragValue::new(&mut self.search_context)
                    .range(0..=20)
                    .speed(0.1),
            )
            .on_hover_text("Lines of context shown above/below each match");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if !self.search_query.is_empty() {
                    if ui.small_button("Clear").clicked() {
                        self.search_query.clear();
                    }
                    if let Some(n) = match_lines {
                        ui.weak(format!(
                            "{n} matching line{}",
                            if n == 1 { "" } else { "s" }
                        ));
                    }
                } else {
                    ui.weak(format!("{} lines", total_lines));
                }
            });
        });
        ui.add_space(6.0);

        let display = render_source_view(
            &self.output,
            &self.search_query,
            self.search_case_sensitive,
            self.search_context,
        );

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut display.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(30),
                );
            });
    }

    fn show_cfg_subtab(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.cfgs.get(self.selected_function).cloned() else {
            ui.centered_and_justified(|ui| {
                ui.weak("No CFG to display.");
            });
            return;
        };

        egui::SidePanel::right("cfg_inspector")
            .resizable(true)
            .default_width(320.0)
            .width_range(220.0..=520.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                cfg_view::show_selected_panel(ui, &self.cfg_view, &snapshot);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                cfg_view::show(ui, &mut self.cfg_view, &snapshot);
            });
    }

    fn show_hex_subtab(&mut self, ui: &mut egui::Ui) {
        let Some(bytes) = self.bytecode.clone() else {
            ui.centered_and_justified(|ui| {
                ui.weak("Open a file to view its bytes.");
            });
            return;
        };
        const ROW_BYTES: usize = 16;
        let total_rows = bytes.len().div_ceil(ROW_BYTES);

        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label("Jump to offset:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.hex_jump_text)
                    .desired_width(110.0)
                    .hint_text("0x… or decimal"),
            );
            #[cfg(target_os = "android")]
            if ui.small_button("📋").clicked() {
                if let Some(t) = crate::android::paste_from_clipboard() {
                    self.hex_jump_text = t;
                }
            }
            let go = (resp.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.small_button("Go").clicked();
            if go {
                if let Some(off) = parse_offset(&self.hex_jump_text) {
                    if off < bytes.len() {
                        self.hex_jump_to_row = Some(off / ROW_BYTES);
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                ui.weak(format!("{} bytes · {} rows", bytes.len(), total_rows));
            });
        });
        ui.add_space(4.0);
        ui.separator();

        let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
        let bytes_for_show = bytes.clone();
        let jump_target = self.hex_jump_to_row.take();

        let mut area = egui::ScrollArea::vertical().auto_shrink(false);
        if let Some(row) = jump_target {
            let y = row as f32 * row_h;
            area = area.vertical_scroll_offset(y.max(0.0));
        }
        area.show_rows(ui, row_h, total_rows, |ui, row_range| {
            let mut buf = String::with_capacity(row_range.len() * 80);
            for row in row_range.clone() {
                let start = row * ROW_BYTES;
                let end = (start + ROW_BYTES).min(bytes_for_show.len());
                let chunk = &bytes_for_show[start..end];
                append_hex_row(&mut buf, start, chunk, ROW_BYTES);
                buf.push('\n');
            }

            if buf.ends_with('\n') {
                buf.pop();
            }
            ui.add(
                egui::TextEdit::multiline(&mut buf.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(row_range.len()),
            );
        });
    }

    fn show_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.heading("Settings");
        });
        ui.add_space(4.0);

        let mut output_changed = false;

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new("Appearance").strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Theme:");
                ui.add_space(4.0);
                ui.selectable_value(&mut self.theme, egui::ThemePreference::System, "System");
                ui.add_space(2.0);
                ui.selectable_value(&mut self.theme, egui::ThemePreference::Light, "Light");
                ui.add_space(2.0);
                ui.selectable_value(&mut self.theme, egui::ThemePreference::Dark, "Dark");
            });
            #[cfg(target_os = "android")]
            {
                ui.add_space(4.0);
                ui.weak("Android: touch-optimized, system clipboard enabled, in-app file browser.");
            }
        });

        ui.add_space(8.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new("Output").strong());
            ui.add_space(4.0);

            let r1 = ui.checkbox(
                &mut self.add_watermark,
                "Add \"-- Decompiled with Topaz\" header to output",
            );
            ui.weak("    Prepends a single comment line at the top of the source view.");
            if r1.changed() {
                output_changed = true;
            }

            ui.add_space(6.0);

            let r2 = ui.checkbox(
                &mut self.show_upvalue_comments,
                "Show \"-- upvalues:\" comments on closures",
            );
            ui.weak("    Lines like `-- upvalues: (copy) x, (ref) y` are emitted by the lifter; turning this off strips them post-hoc from the source view.");
            if r2.changed() {
                output_changed = true;
            }
        });

        ui.add_space(8.0);

        #[cfg(target_os = "android")]
        {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(egui::RichText::new("Android permissions").strong());
                ui.add_space(4.0);
                let p = crate::android::permission_status();
                ui.monospace(format!(
                    "READ_EXTERNAL_STORAGE: {}\nREAD_MEDIA_IMAGES: {}\nREAD_MEDIA_VIDEO: {}\nREAD_MEDIA_AUDIO: {}\nMANAGE_EXTERNAL: {}",
                    p.storage_granted, p.media_images, p.media_video, p.media_audio, p.manage_external
                ));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Request again").clicked() {
                        crate::android::request_storage_permissions_async();
                    }
                    if ui.button("Open All-files Settings").clicked() {
                        crate::android::open_app_settings();
                    }
                });
                ui.weak(&p.last_message);
                ui.add_space(4.0);
                ui.weak("Android 11+: direct-path mode needs Special app access → All files access. Photo/video/audio permissions do not cover Lua or bytecode files.");
            });

            ui.add_space(8.0);

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(egui::RichText::new("Android keep-alive (like Termux)").strong());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let mut ka = self.keep_alive;
                    if ui.checkbox(&mut ka, "Keep alive in background").clicked() {
                        self.keep_alive = ka;
                        if ka {
                            crate::android::enable_keepalive();
                        } else {
                            crate::android::disable_keepalive();
                        }
                        self.save_state();
                        ui.ctx().request_repaint();
                    }
                });
                ui.weak("Acquires a wakelock and shows a non-swipeable notification so the app stays alive after closing (like Termux). Settings are saved and restored on next launch.");
                ui.add_space(4.0);
                let wl_held = crate::android::is_wakelock_held();
                ui.horizontal(|ui| {
                    ui.monospace("Wakelock:");
                    if wl_held {
                        ui.colored_label(egui::Color32::from_rgb(90, 200, 120), "● held");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(180, 180, 180), "○ released");
                    }
                    ui.monospace("  Notification:");
                    if self.keep_alive {
                        ui.colored_label(egui::Color32::from_rgb(90, 200, 120), "● ongoing (non-swipeable)");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(180, 180, 180), "○ none");
                    }
                });
            });

            ui.add_space(8.0);

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(egui::RichText::new("Android test controls").strong());
                ui.add_space(4.0);

                let wl_held = crate::android::is_wakelock_held();
                ui.horizontal(|ui| {
                    ui.label("Wakelock:");
                    if wl_held {
                        ui.colored_label(egui::Color32::from_rgb(90, 200, 120), "● held");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(180, 180, 180), "○ released");
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("🔒 Acquire test wakelock").clicked() {
                        crate::android::acquire_partial_wakelock("TopazTest");
                        ui.ctx().request_repaint();
                    }
                    if ui.button("🔓 Release wakelock").clicked() {
                        crate::android::release_wakelock();
                        ui.ctx().request_repaint();
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("🔔 Test notification").clicked() {
                        crate::android::create_notification_channel(
                            "topaz_test",
                            "Test Channel",
                            crate::android::notification_importance::DEFAULT,
                        );
                        crate::android::show_notification(
                            "topaz_test",
                            "Topaz Test",
                            "This is a test notification from Topaz.",
                            42,
                        );
                    }
                    if ui.button("🔕 Cancel test notification").clicked() {
                        crate::android::cancel_notification(42);
                    }
                    if ui.button("Cancel all").clicked() {
                        crate::android::cancel_all_notifications();
                    }
                });

                ui.add_space(4.0);
                ui.weak("The server tab automatically acquires a wakelock and shows a notification when the server is running.");
            });
        }

        ui.add_space(8.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new("About").strong());
            ui.add_space(4.0);
            ui.label("Topaz · Luau and Lua 5.1 decompiler");
            #[cfg(target_os = "android")]
            ui.label("Android build · pure Rust · eframe + NativeActivity · system clipboard + SAF browser");
            #[cfg(not(target_os = "android"))]
            ui.label("Desktop build");
        });

        if output_changed {
            self.apply_output_settings();
            self.save_state();
        }
    }

    fn show_server_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        #[cfg(target_os = "android")]
        self.poll_android_server_status();

        let state = self.current_server_state();
        let running = state.is_running();
        let transitional = state.is_transitional();
        let inputs_enabled = !running && !transitional;

        // Status changes on Android arrive via polling, not a repaint callback
        // (there's no live callback across the process boundary), so keep the
        // UI refreshing while this tab is open.
        #[cfg(target_os = "android")]
        ctx.request_repaint_after(std::time::Duration::from_millis(400));

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Run a local HTTP server that accepts bytecode and returns Lua source.",
            )
            .weak(),
        );
        #[cfg(target_os = "android")]
        ui.label(
            egui::RichText::new(
                "On Android: server binds 0.0.0.0, use adb or local device. Needs INTERNET permission.",
            )
            .weak(),
        );
        ui.add_space(4.0);
        egui::CollapsingHeader::new("What do these settings mean?")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Port").strong());
                ui.label(PORT_TOOLTIP);
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Default encode key").strong());
                ui.label(ENCODE_KEY_TOOLTIP);
            });
        ui.add_space(8.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.add_enabled_ui(inputs_enabled, |ui| {
            egui::Grid::new("server_cfg")
                .num_columns(3)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Port").on_hover_text(PORT_TOOLTIP);
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.server_port_text)
                                .desired_width(80.0),
                        )
                        .on_hover_text(PORT_TOOLTIP)
                        .changed()
                    {
                        if let Ok(v) = self.server_port_text.parse::<u16>() {
                            self.server_port = v;
                            self.save_state();
                        }
                    }
                    ui.weak(port_hint(self.server_port));
                    ui.end_row();

                    ui.label("Routes");
                    ui.horizontal(|ui| {
                        let old_luau = self.server_luau;
                        let old_lua51 = self.server_lua51;
                        ui.checkbox(&mut self.server_luau, "Luau (/luau/decompile)");
                        ui.checkbox(&mut self.server_lua51, "Lua 5.1 (/lua51/decompile)");
                        if self.server_luau != old_luau || self.server_lua51 != old_lua51 {
                            self.save_state();
                        }
                    });
                    ui.label("");
                    ui.end_row();

                    ui.label("Default encode key")
                        .on_hover_text(ENCODE_KEY_TOOLTIP);
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.server_encode_key_text)
                                .desired_width(60.0),
                        )
                        .on_hover_text(ENCODE_KEY_TOOLTIP)
                        .changed()
                    {
                        if let Ok(v) = self.server_encode_key_text.parse::<u8>() {
                            self.server_encode_key = v;
                            self.save_state();
                        }
                    }
                    ui.weak("Used when ?encode_key=… is not in the URL.")
                        .on_hover_text(ENCODE_KEY_TOOLTIP);
                    ui.end_row();
                });
            });
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            let start_label = if transitional { "Working…" } else { "▶  Start server" };
            let can_start = !running && !transitional && (self.server_luau || self.server_lua51);
            if ui
                .add_enabled(can_start, egui::Button::new(start_label).min_size(egui::vec2(140.0, 28.0)))
                .clicked()
            {
                #[cfg(not(target_os = "android"))]
                {
                    let ctx = ctx.clone();
                    let cfg = ServerConfig {
                        port: self.server_port,
                        luau: self.server_luau,
                        lua51: self.server_lua51,
                        encode_key: self.server_encode_key,
                    };
                    self.server.start(cfg, move || ctx.request_repaint());
                }
                #[cfg(target_os = "android")]
                {
                    crate::android::start_remote_server(crate::android::RemoteServerConfig {
                        port: self.server_port,
                        luau: self.server_luau,
                        lua51: self.server_lua51,
                        encode_key: self.server_encode_key,
                    });
                }
            }

            if ui
                .add_enabled(running, egui::Button::new("■  Stop server").min_size(egui::vec2(140.0, 28.0)))
                .clicked()
            {
                #[cfg(not(target_os = "android"))]
                self.server.stop();
                #[cfg(target_os = "android")]
                crate::android::stop_remote_server();
            }

            if !self.server_luau && !self.server_lua51 && !running {
                ui.weak("Enable at least one route to start.");
            }
        });

        ui.add_space(10.0);

        if let ServerState::Running { addr, started_at } = &state {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(egui::RichText::new("Endpoints").strong());
                ui.add_space(4.0);

                let base = format!("http://{addr}");
                endpoint_row(ui, "GET", &format!("{base}/"), "Health check");
                if self.server_luau {
                    endpoint_row(
                        ui,
                        "POST",
                        &format!("{base}/luau/decompile?encode_key={}", self.server_encode_key),
                        "Body: raw or base64 Luau bytecode",
                    );
                }
                if self.server_lua51 {
                    endpoint_row(
                        ui,
                        "POST",
                        &format!("{base}/lua51/decompile"),
                        "Body: raw or base64 Lua 5.1 bytecode",
                    );
                }

                ui.add_space(6.0);
                if let Ok(elapsed) = started_at.elapsed() {
                    ui.weak(format!("Uptime: {}", format_duration(elapsed)));
                    ctx.request_repaint_after(std::time::Duration::from_secs(1));
                }
            });

            ui.add_space(8.0);

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Activity").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Reset").clicked() {
                            #[cfg(not(target_os = "android"))]
                            self.server.reset_stats();
                            // Android: stats live in the :service process; a
                            // "reset while running" remote command isn't
                            // wired up yet, so this is a no-op there for now.
                        }
                    });
                });
                ui.add_space(4.0);

                #[cfg(not(target_os = "android"))]
                let (luau, lua51, bytes_in, bytes_out, last_str) = {
                    use std::sync::atomic::Ordering;
                    let s = &self.server.stats;
                    let last_str = {
                        let last = s.last_request.lock().ok().and_then(|g| *g);
                        match last.and_then(|t| t.elapsed().ok()) {
                            None => "—".to_string(),
                            Some(elapsed) => format!("{} ago", format_duration(elapsed)),
                        }
                    };
                    (
                        s.luau_requests.load(Ordering::Relaxed),
                        s.lua51_requests.load(Ordering::Relaxed),
                        s.bytes_in.load(Ordering::Relaxed),
                        s.bytes_out.load(Ordering::Relaxed),
                        last_str,
                    )
                };
                #[cfg(target_os = "android")]
                let (luau, lua51, bytes_in, bytes_out, last_str) = {
                    let s = &self.android_server_status;
                    let last_str = match s.last_request_ms {
                        None => "—".to_string(),
                        Some(ms) => {
                            let now = ipc_stats::now_ms();
                            let elapsed = std::time::Duration::from_millis(now.saturating_sub(ms));
                            format!("{} ago", format_duration(elapsed))
                        }
                    };
                    (s.luau_requests, s.lua51_requests, s.bytes_in, s.bytes_out, last_str)
                };
                let total = luau + lua51;

                egui::Grid::new("server_stats")
                    .num_columns(2)
                    .spacing([18.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Requests served");
                        ui.monospace(format!(
                            "{total}  ({} Luau, {} Lua 5.1)",
                            luau, lua51
                        ));
                        ui.end_row();

                        ui.label("Bytes in / out");
                        ui.monospace(format!(
                            "{}  /  {}",
                            format_bytes(bytes_in),
                            format_bytes(bytes_out)
                        ));
                        ui.end_row();

                        ui.label("Last request");
                        ui.monospace(last_str);
                        ui.end_row();
                    });

                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            });

            ui.add_space(8.0);
            egui::CollapsingHeader::new("Example: curl").show(ui, |ui| {
                let example = if self.server_luau {
                    format!(
                        "curl -X POST --data-binary @bytecode.bin \\\n  '{}/luau/decompile?encode_key={}'",
                        format!("http://{addr}"),
                        self.server_encode_key,
                    )
                } else {
                    format!(
                        "curl -X POST --data-binary @bytecode.bin '{}/lua51/decompile'",
                        format!("http://{addr}"),
                    )
                };
                ui.add(
                    egui::TextEdit::multiline(&mut example.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(2)
                        .desired_width(f32::INFINITY),
                );
            });
        }
    }
}

fn server_status_badge(ui: &mut egui::Ui, state: &ServerState) {
    let (dot, label, color) = match state {
        ServerState::Stopped => ("●", "stopped", egui::Color32::from_rgb(120, 120, 120)),
        ServerState::Starting { .. } => ("●", "starting", egui::Color32::from_rgb(230, 180, 60)),
        ServerState::Running { .. } => ("●", "running", egui::Color32::from_rgb(80, 200, 110)),
        ServerState::Stopping => ("●", "stopping", egui::Color32::from_rgb(230, 180, 60)),
        ServerState::Failed { .. } => ("●", "error", egui::Color32::from_rgb(220, 80, 80)),
    };
    ui.colored_label(color, dot);
    ui.weak(format!("server: {label}"));
}

fn endpoint_row(ui: &mut egui::Ui, method: &str, url: &str, hint: &str) {
    ui.horizontal(|ui| {
        let method_color = match method {
            "GET" => egui::Color32::from_rgb(80, 160, 220),
            "POST" => egui::Color32::from_rgb(220, 140, 80),
            _ => ui.visuals().text_color(),
        };
        ui.colored_label(method_color, egui::RichText::new(method).strong().monospace());
        ui.monospace(url);
        if ui.small_button("Copy").clicked() {
            #[cfg(target_os = "android")]
            {
                crate::android::copy_to_clipboard(url);
                crate::android::toast("Copied URL");
            }
            #[cfg(not(target_os = "android"))]
            ui.ctx().copy_text(url.to_string());
        }
        ui.weak(hint);
    });
}

fn panic_message(e: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = e.downcast_ref::<&'static str>() {
        s.to_string()
    } else {
        "Unknown error".to_string()
    }
}

fn render_source_view(source: &str, query: &str, case_sensitive: bool, context: u32) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let gutter_w = (lines.len().max(1) as f32).log10() as usize + 1;
    let gutter_w = gutter_w.max(3);

    if query.is_empty() {
        let mut out = String::with_capacity(source.len() + lines.len() * (gutter_w + 4));
        for (i, line) in lines.iter().enumerate() {
            use std::fmt::Write;
            let _ = write!(out, "{:>width$} │ {}\n", i + 1, line, width = gutter_w);
        }
        if out.ends_with('\n') {
            out.pop();
        }
        return out;
    }

    let needle = if case_sensitive { query.to_string() } else { query.to_lowercase() };
    let context = context as usize;
    let mut keep = vec![false; lines.len()];
    for (i, line) in lines.iter().enumerate() {
        let hay = if case_sensitive { (*line).to_string() } else { line.to_lowercase() };
        if hay.contains(&needle) {
            let lo = i.saturating_sub(context);
            let hi = (i + context + 1).min(lines.len());
            for k in lo..hi {
                keep[k] = true;
            }
        }
    }

    let mut out = String::new();
    let mut last_kept: Option<usize> = None;
    for i in 0..lines.len() {
        if !keep[i] {
            continue;
        }
        if let Some(prev) = last_kept {
            if i > prev + 1 {
                out.push_str("    …\n");
            }
        }
        use std::fmt::Write;
        let marker = {
            let hay = if case_sensitive { lines[i].to_string() } else { lines[i].to_lowercase() };
            if hay.contains(&needle) { '▸' } else { ' ' }
        };
        let _ = write!(
            out,
            "{:>width$} {} {}\n",
            i + 1,
            marker,
            lines[i],
            width = gutter_w
        );
        last_kept = Some(i);
    }
    if last_kept.is_none() {
        out.push_str("(no matches)");
    } else if out.ends_with('\n') {
        out.pop();
    }
    out
}

fn count_matches(source: &str, query: &str, case_sensitive: bool) -> usize {
    let needle = if case_sensitive { query.to_string() } else { query.to_lowercase() };
    source
        .lines()
        .filter(|l| {
            let hay = if case_sensitive { (*l).to_string() } else { l.to_lowercase() };
            hay.contains(&needle)
        })
        .count()
}

fn append_hex_row(out: &mut String, offset: usize, chunk: &[u8], row_width: usize) {
    use std::fmt::Write;
    let _ = write!(out, "{:08x}  ", offset);
    for i in 0..row_width {
        if let Some(b) = chunk.get(i) {
            let _ = write!(out, "{:02x} ", b);
        } else {
            out.push_str("   ");
        }
        if i == row_width / 2 - 1 {
            out.push(' ');
        }
    }
    out.push(' ');
    out.push('|');
    for b in chunk {
        out.push(if (0x20..=0x7e).contains(b) { *b as char } else { '.' });
    }
    out.push('|');
}

fn parse_offset(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(rest, 16).ok()
    } else {
        s.parse::<usize>()
            .ok()
            .or_else(|| usize::from_str_radix(s, 16).ok())
    }
}

fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if n >= GB {
        format!("{:.2} GiB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.2} MiB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KiB", n as f64 / KB as f64)
    } else {
        format!("{n} B")
    }
}

fn format_duration(d: std::time::Duration) -> String {
    let s = d.as_secs();
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}h {m}m {sec}s")
    } else if m > 0 {
        format!("{m}m {sec}s")
    } else {
        format!("{sec}s")
    }
}
