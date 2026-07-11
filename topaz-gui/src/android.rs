// Android-specific utilities for Topaz
// - system clipboard (android_clipboard)
// - runtime storage permissions (android-permissions)
// - in-app file browser
// - SAF file picker fallback
#![cfg(target_os = "android")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use eframe::egui;
use log::{error, info, warn};
use winit::platform::android::activity::AndroidApp;

// Global AndroidApp handle so we can request permissions anywhere
static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

pub fn init_android_app(app: AndroidApp) {
    let _ = ANDROID_APP.set(app);
}

pub fn android_app() -> Option<AndroidApp> {
    ANDROID_APP.get().cloned()
}

// --------------------------------------------------------------------
// Clipboard
// --------------------------------------------------------------------

pub fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "android")]
    {
        match android_clipboard::set_text(text.to_string()) {
            Ok(()) => {
                info!("Copied {} bytes to Android clipboard", text.len());
                true
            }
            Err(e) => {
                error!("android_clipboard set_text failed: {e:?}");
                false
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = text;
        false
    }
}

pub fn paste_from_clipboard() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        match android_clipboard::get_text() {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("clipboard get_text failed: {e:?}");
                None
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    None
}

// Small helper to use in egui: copy and return a status string
pub fn egui_copy(text: &str) -> &'static str {
    if copy_to_clipboard(text) {
        "Copied to system clipboard ✓"
    } else {
        "Clipboard copy failed — check logcat"
    }
}

// --------------------------------------------------------------------
// Permissions
// --------------------------------------------------------------------
// Uses android-permissions 0.1.2 with android-activity feature

#[derive(Clone, Debug, Default)]
pub struct PermissionStatus {
    pub checked_at: Option<Instant>,
    pub storage_granted: bool,
    pub media_images: bool,
    pub media_video: bool,
    pub media_audio: bool,
    pub manage_external: bool,
    pub last_message: String,
}

static PERM_STATUS: OnceLock<std::sync::Mutex<PermissionStatus>> = OnceLock::new();

// android-permissions loads PermissionFragment through a DexClassLoader. Creating a
// manager for every request creates several distinct Java classes with the same
// name. Keep one manager/class loader for the process and never overlap requests.
static PERMISSION_MANAGER: OnceLock<android_permissions::PermissionManager> = OnceLock::new();
static PERMISSION_REQUEST_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

fn permission_manager(
    app: &AndroidApp,
) -> Result<&'static android_permissions::PermissionManager, android_permissions::Error> {
    if let Some(manager) = PERMISSION_MANAGER.get() {
        return Ok(manager);
    }

    // More than one caller may race here. Both managers can be constructed, but
    // only the winner is retained and returned, so all requests use one loader.
    let manager = android_permissions::PermissionManager::create_from_android_app(app)?;
    let _ = PERMISSION_MANAGER.set(manager);
    Ok(PERMISSION_MANAGER
        .get()
        .expect("permission manager was initialized"))
}

fn perm_status_lock() -> &'static std::sync::Mutex<PermissionStatus> {
    PERM_STATUS.get_or_init(|| std::sync::Mutex::new(PermissionStatus::default()))
}

pub fn permission_status() -> PermissionStatus {
    perm_status_lock().lock().unwrap().clone()
}

fn set_permission_status(s: PermissionStatus) {
    *perm_status_lock().lock().unwrap() = s;
}

// Call after a user action; repeated calls while a request is active are ignored.
pub fn request_storage_permissions_async() {
    let Some(app) = android_app() else {
        warn!("request_storage_permissions: no AndroidApp yet");
        return;
    };

    if PERMISSION_REQUEST_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        info!("A storage permission request is already in progress");
        return;
    }

    std::thread::spawn(move || {
        struct InFlightGuard;
        impl Drop for InFlightGuard {
            fn drop(&mut self) {
                PERMISSION_REQUEST_IN_FLIGHT.store(false, Ordering::Release);
            }
        }
        let _in_flight_guard = InFlightGuard;

        // android-permissions needs a JavaVM + Activity jobject. Reuse the
        // process-wide manager so PermissionFragment always has one class identity.
        match permission_manager(&app) {
            Ok(manager) => {
                use android_permissions as perm;
                // Build a version-aware list
                // API 33+ (Tiramisu): READ_MEDIA_*
                // API 30-32: READ_EXTERNAL_STORAGE
                // Always try MANAGE_EXTERNAL_STORAGE for “All files access” on API 30+
                let to_request = [
                    &perm::READ_EXTERNAL_STORAGE,
                    &perm::WRITE_EXTERNAL_STORAGE,
                    &perm::READ_MEDIA_IMAGES,
                    &perm::READ_MEDIA_VIDEO,
                    &perm::READ_MEDIA_AUDIO,
                ];

                let mut status = PermissionStatus {
                    checked_at: Some(Instant::now()),
                    last_message: "Requesting…".into(),
                    ..Default::default()
                };

                // check first
                for p in to_request.iter() {
                    if let Ok(granted) = manager.check(p) {
                        match p.as_str() {
                            "android.permission.READ_EXTERNAL_STORAGE" => status.storage_granted |= granted,
                            "android.permission.READ_MEDIA_IMAGES" => status.media_images = granted,
                            "android.permission.READ_MEDIA_VIDEO" => status.media_video = granted,
                            "android.permission.READ_MEDIA_AUDIO" => status.media_audio = granted,
                            _ => {}
                        }
                    }
                }

                // actually request (blocking, shows system dialog)
                match manager.request(&to_request) {
                    Ok(grants) => {
                        let mut msg = String::new();
                        for (k, v) in &grants {
                            msg.push_str(&format!("{}: {}  ", k.rsplit('.').next().unwrap_or(k), if *v {"✓"} else {"✗"}));
                            match k.as_str() {
                                "android.permission.READ_EXTERNAL_STORAGE" => status.storage_granted = *v,
                                "android.permission.READ_MEDIA_IMAGES" => status.media_images = *v,
                                "android.permission.READ_MEDIA_VIDEO" => status.media_video = *v,
                                "android.permission.READ_MEDIA_AUDIO" => status.media_audio = *v,
                                _ => {}
                            }
                        }
                        status.last_message = if status.storage_granted || status.media_images || status.media_video || status.media_audio {
                            format!("Permissions OK — {msg}")
                        } else {
                            format!("Permissions denied — {msg} — use Settings → Apps → Topaz → Permissions → Allow Files")
                        };
                        info!("{}", status.last_message);
                    }
                    Err(e) => {
                        error!("permission request failed: {e}");
                        status.last_message = format!("Permission request error: {e}");
                    }
                }

                // Try to detect MANAGE_EXTERNAL_STORAGE via AppOps (best-effort)
                status.manage_external = is_manage_external_storage_granted();

                status.checked_at = Some(Instant::now());
                set_permission_status(status);
            }
            Err(e) => {
                error!("PermissionManager initialization failed: {e:?}");
                set_permission_status(PermissionStatus{
                    checked_at: Some(Instant::now()),
                    last_message: format!("Permission manager init failed: {e}"),
                    ..Default::default()
                });
            }
        }
    });
}

// Best-effort check for MANAGE_EXTERNAL_STORAGE via Environment.isExternalStorageManager()
fn is_manage_external_storage_granted() -> bool {
    // Do a tiny JNI call – if anything fails, return false
    (|| -> anyhow::Result<bool> {
        use jni::objects::JObject;
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as *mut _) }?;
        let mut env = vm.attach_current_thread()?;
        let version_class = env.find_class("android/os/Build$VERSION")?;
        let sdk_int: i32 = env.get_static_field(version_class, "SDK_INT", "I")?.i()?;
        if sdk_int < 30 {
            return Ok(false);
        }
        let env_class = env.find_class("android/os/Environment")?;
        let is_mgr: bool = env.call_static_method(
            env_class,
            "isExternalStorageManager",
            "()Z",
            &[],
        )?.z()?;
        Ok(is_mgr)
    })().unwrap_or(false)
}

pub fn open_app_settings() {
    // Launch ACTION_APPLICATION_DETAILS_SETTINGS
    let Some(_app) = android_app() else { return };
    (|| -> anyhow::Result<()> {
        use jni::objects::{JObject, JString, JValue};
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as *mut _) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(ctx.context() as *mut _) };

        let intent_class = env.find_class("android/content/Intent")?;
        let action_settings = env.new_string("android.settings.APPLICATION_DETAILS_SETTINGS")?;
        let intent = env.new_object(
            intent_class,
            "(Ljava/lang/String;)V",
            &[(&action_settings).into()],
        )?;

        // Uri.parse("package:com.exec.topaz")
        let uri_class = env.find_class("android/net/Uri")?;
        let pkg_str = env.new_string("package:com.exec.topaz")?;
        let uri = env.call_static_method(
            uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[(&pkg_str).into()],
        )?.l()?;

        env.call_method(
            &intent,
            "setData",
            "(Landroid/net/Uri;)Landroid/content/Intent;",
            &[(&uri).into()],
        )?;

        // FLAG_ACTIVITY_NEW_TASK
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(0x10000000)],
        )?;

        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[(&intent).into()],
        )?;
        Ok(())
    })().map_err(|e| error!("open_app_settings failed: {e:?}")).ok();
}

// --------------------------------------------------------------------
// File browser
// --------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

pub struct FileBrowser {
    current: PathBuf,
    entries: Vec<Entry>,
    error: Option<String>,
    show_hidden: bool,
    filter_ext: Vec<String>,
    pub selected: Option<PathBuf>,
    last_refresh: Instant,
}

impl FileBrowser {
    pub fn new(start: impl AsRef<Path>) -> Self {
        let mut fb = Self {
            current: start.as_ref().to_path_buf(),
            entries: Vec::new(),
            error: None,
            show_hidden: false,
            filter_ext: vec!["luac".into(), "lua".into(), "bin".into(), "txt".into(), "luau".into(), "dat".into()],
            selected: None,
            last_refresh: Instant::now() - Duration::from_secs(10),
        };
        fb.refresh();
        fb
    }

    pub fn current_path(&self) -> &Path {
        &self.current
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.error = None;
        match std::fs::read_dir(&self.current) {
            Ok(rd) => {
                let mut dirs = Vec::new();
                let mut files = Vec::new();
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if !self.show_hidden && name.starts_with('.') {
                        continue;
                    }
                    let path = e.path();
                    let meta = e.metadata().ok();
                    let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                    let entry = Entry { name, path, is_dir, size };
                    if is_dir {
                        dirs.push(entry);
                    } else {
                        // filter
                        if self.filter_ext.is_empty() || entry.name.rsplit('.').next().map(|ext| self.filter_ext.iter().any(|f| f.eq_ignore_ascii_case(ext))).unwrap_or(false) {
                            files.push(entry);
                        } else {
                            // still include if no extension filter hit? include all to avoid confusion
                            files.push(entry);
                        }
                    }
                }
                dirs.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                files.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                self.entries.extend(dirs);
                self.entries.extend(files);
            }
            Err(e) => {
                self.error = Some(format!("Cannot read {}: {e}", self.current.display()));
            }
        }
        self.last_refresh = Instant::now();
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.current.parent().map(|p| p.to_path_buf()) {
            self.current = parent;
            self.refresh();
        }
    }

    fn go_to(&mut self, p: PathBuf) {
        self.current = p;
        self.refresh();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let mut picked = None;

        ui.horizontal_wrapped(|ui| {
            if ui.button("⬆ Up").clicked() { self.go_up(); }
            if ui.button("⟳ Refresh").clicked() { self.refresh(); }
            ui.checkbox(&mut self.show_hidden, "hidden");
            ui.separator();
            // Quick bookmarks
            for (label, path) in [
                ("sdcard", "/sdcard"),
                ("Download", "/sdcard/Download"),
                ("emulated", "/storage/emulated/0"),
                ("Docs", "/sdcard/Documents"),
                ("App files", "/data/data/com.exec.topaz/files"),
            ] {
                if ui.small_button(label).clicked() {
                    self.go_to(PathBuf::from(path));
                }
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("📁");
            ui.monospace(self.current.display().to_string());
            if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
            if self.entries.is_empty() {
                ui.weak("(empty — check permissions)");
                return;
            }
            egui::Grid::new("fb_grid").num_columns(3).spacing([8.0, 4.0]).striped(true).show(ui, |ui| {
                for entry in self.entries.clone() {
                    let icon = if entry.is_dir { "📁" } else { "📄" };
                    ui.label(icon);
                    if ui.link(&entry.name).clicked() {
                        if entry.is_dir {
                            self.go_to(entry.path);
                            break;
                        } else {
                            picked = Some(entry.path.clone());
                            self.selected = picked.clone();
                        }
                    }
                    if entry.is_dir {
                        ui.weak("dir");
                    } else {
                        ui.weak(format!("{} B", entry.size));
                    }
                    ui.end_row();
                }
            });
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Filter ext:");
            let mut filter_str = self.filter_ext.join(",");
            if ui.text_edit_singleline(&mut filter_str).changed() {
                self.filter_ext = filter_str.split(',').map(|s| s.trim().trim_start_matches('.').to_lowercase()).filter(|s| !s.is_empty()).collect();
                self.refresh();
            }
            if ui.small_button("clear").clicked() {
                self.filter_ext.clear();
                self.refresh();
            }
            ui.weak("comma-separated, e.g. luac,bin,lua");
        });

        picked
    }
}

// --------------------------------------------------------------------
// SAF picker (best-effort, fire-and-forget — copies result to /sdcard/Download/topaz_picked_* )
// For a full SAF implementation you’d need onActivityResult plumbing.
// Here we provide a simple “Open system picker” button that launches the intent.
// User must manually copy file path back — so we primarily rely on FileBrowser.
// --------------------------------------------------------------------
pub fn launch_saf_open_document(mime: &str) {
    let Some(_app) = android_app() else { return };
    (|| -> anyhow::Result<()> {
        use jni::objects::{JObject, JValue};
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as *mut _) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(ctx.context() as *mut _) };

        // Intent(Intent.ACTION_OPEN_DOCUMENT)
        let intent_class = env.find_class("android/content/Intent")?;
        let action = env.new_string("android.intent.action.OPEN_DOCUMENT")?;
        let intent = env.new_object(intent_class, "(Ljava/lang/String;)V", &[(&action).into()])?;

        // intent.setType(mime)
        let mime_s = env.new_string(mime)?;
        env.call_method(&intent, "setType", "(Ljava/lang/String;)Landroid/content/Intent;", &[(&mime_s).into()])?;
        // addCategory OPENABLE
        let cat = env.new_string("android.intent.category.OPENABLE")?;
        env.call_method(&intent, "addCategory", "(Ljava/lang/String;)Landroid/content/Intent;", &[(&cat).into()])?;

        // startActivityForResult is deprecated, use startActivity
        env.call_method(activity, "startActivity", "(Landroid/content/Intent;)V", &[(&intent).into()])?;
        Ok(())
    })().map_err(|e| warn!("launch_saf_open_document failed: {e:?}")).ok();
}

// Toast helper
pub fn toast(msg: &str) {
    (|| -> anyhow::Result<()> {
        use jni::objects::{JObject, JValue};
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as *mut _) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(ctx.context() as *mut _) };
        let text = env.new_string(msg)?;
        let toast_class = env.find_class("android/widget/Toast")?;
        let toast_obj = env.call_static_method(
            toast_class,
            "makeText",
            "(Landroid/content/Context;Ljava/lang/CharSequence;I)Landroid/widget/Toast;",
            &[(&activity).into(), (&text).into(), JValue::Int(0)],
        )?.l()?;
        env.call_method(toast_obj, "show", "()V", &[])?;
        Ok(())
    })().map_err(|e| warn!("toast failed: {e:?}")).ok();
}
