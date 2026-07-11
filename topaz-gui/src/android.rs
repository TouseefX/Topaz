// Android-specific utilities for Topaz
// - system clipboard (direct JNI)
// - runtime storage permissions (android-permissions)
// - in-app file browser
// - SAF file picker fallback
#![cfg(target_os = "android")]

use std::path::{Path, PathBuf};
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

fn clear_pending_java_exception(env: &mut jni::JNIEnv<'_>) {
    if matches!(env.exception_check(), Ok(true)) {
        // A pending Java exception must not escape back through NativeActivity;
        // doing so aborts the process with SIGABRT.
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

pub fn copy_to_clipboard(text: &str) -> bool {
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else {
        return false;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return false;
    };
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> anyhow::Result<()> {
        let service = env.new_string("clipboard")?;
        let manager = env
            .call_method(
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service)],
            )?
            .l()?;
        if manager.is_null() {
            anyhow::bail!("ClipboardManager is unavailable");
        }

        let label = env.new_string("Topaz")?;
        let value = env.new_string(text)?;
        let clip = env
            .call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&value)],
            )?
            .l()?;
        env.call_method(
            manager,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JValue::Object(&clip)],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            info!("Copied {} bytes to Android clipboard", text.len());
            true
        }
        Err(e) => {
            clear_pending_java_exception(&mut env);
            error!("Android clipboard copy failed: {e:?}");
            false
        }
    }
}

pub fn paste_from_clipboard() -> Option<String> {
    use jni::objects::{JObject, JString, JValue};

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> anyhow::Result<Option<String>> {
        let service = env.new_string("clipboard")?;
        let manager = env
            .call_method(
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service)],
            )?
            .l()?;
        if manager.is_null()
            || !env.call_method(&manager, "hasPrimaryClip", "()Z", &[])?.z()?
        {
            return Ok(None);
        }

        let clip = env
            .call_method(
                &manager,
                "getPrimaryClip",
                "()Landroid/content/ClipData;",
                &[],
            )?
            .l()?;
        if clip.is_null() || env.call_method(&clip, "getItemCount", "()I", &[])?.i()? == 0 {
            return Ok(None);
        }

        let item = env
            .call_method(
                &clip,
                "getItemAt",
                "(I)Landroid/content/ClipData$Item;",
                &[JValue::Int(0)],
            )?
            .l()?;

        // getText() may legally return null for URI/Intent/rich clipboard
        // entries. coerceToText() is the Android-supported conversion API.
        let chars = env
            .call_method(
                item,
                "coerceToText",
                "(Landroid/content/Context;)Ljava/lang/CharSequence;",
                &[JValue::Object(&activity)],
            )?
            .l()?;
        if chars.is_null() {
            return Ok(None);
        }

        let string_obj = env
            .call_method(chars, "toString", "()Ljava/lang/String;", &[])?
            .l()?;
        if string_obj.is_null() {
            return Ok(None);
        }
        let string = JString::from(string_obj);
        let value: String = env.get_string(&string)?.into();
        Ok(Some(value))
    })();

    match result {
        Ok(value) => value,
        Err(e) => {
            clear_pending_java_exception(&mut env);
            warn!("Android clipboard paste failed: {e:?}");
            None
        }
    }
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

fn perm_status_lock() -> &'static std::sync::Mutex<PermissionStatus> {
    PERM_STATUS.get_or_init(|| std::sync::Mutex::new(PermissionStatus::default()))
}

pub fn permission_status() -> PermissionStatus {
    let mut status = perm_status_lock().lock().unwrap().clone();
    if android_sdk_int() >= 30 {
        status.manage_external = is_manage_external_storage_granted();
    }
    status
}

fn set_permission_status(status: PermissionStatus) {
    *perm_status_lock().lock().unwrap() = status;
}

pub fn request_storage_permissions_async() {
    let sdk = android_sdk_int();

    // Android 11+ uses scoped storage. READ_EXTERNAL_STORAGE and READ_MEDIA_*
    // cannot grant access to arbitrary .lua/.luac/.bin files. Direct-path mode
    // needs the special MANAGE_EXTERNAL_STORAGE settings switch instead.
    if sdk >= 30 {
        let granted = is_manage_external_storage_granted();
        set_permission_status(PermissionStatus {
            checked_at: Some(Instant::now()),
            manage_external: granted,
            last_message: if granted {
                "All-files access is enabled.".into()
            } else {
                "Enable “Allow access to manage all files” for direct-path mode.".into()
            },
            ..Default::default()
        });
        if !granted {
            open_app_settings();
        }
        return;
    }

    // Android 6-10 legacy fallback. No callback is required here; Android will
    // show the dialog and the next file operation can check the result.
    use jni::objects::{JObject, JValue};
    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return;
    };
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> anyhow::Result<()> {
        let permissions = env.new_object_array(2, "java/lang/String", JObject::null())?;
        let read = env.new_string("android.permission.READ_EXTERNAL_STORAGE")?;
        let write = env.new_string("android.permission.WRITE_EXTERNAL_STORAGE")?;
        env.set_object_array_element(&permissions, 0, read)?;
        env.set_object_array_element(&permissions, 1, write)?;
        env.call_method(
            activity,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[JValue::Object(&permissions), JValue::Int(7310)],
        )?;
        Ok(())
    })();

    if let Err(e) = result {
        clear_pending_java_exception(&mut env);
        error!("Legacy storage permission request failed: {e:?}");
    }
}

pub fn direct_file_access_granted() -> bool {
    android_sdk_int() < 30 || is_manage_external_storage_granted()
}

fn android_sdk_int() -> i32 {
    (|| -> anyhow::Result<i32> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        Ok(env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
            .i()?)
    })()
    .unwrap_or(0)
}

// Best-effort check for MANAGE_EXTERNAL_STORAGE via Environment.isExternalStorageManager()
fn is_manage_external_storage_granted() -> bool {
    // Do a tiny JNI call – if anything fails, return false
    (|| -> anyhow::Result<bool> {
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
    // Android 11+: open the app-specific "Manage all files" special-access
    // screen. This is not part of the normal runtime permission dialog.
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return;
    };
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> anyhow::Result<()> {
        let action = env.new_string("android.settings.MANAGE_APP_ALL_FILES_ACCESS_PERMISSION")?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;

        let package = env.new_string("package:com.exec.topaz")?;
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&package)],
            )?
            .l()?;
        env.call_method(
            &intent,
            "setData",
            "(Landroid/net/Uri;)Landroid/content/Intent;",
            &[JValue::Object(&uri)],
        )?;
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })();

    if let Err(e) = result {
        clear_pending_java_exception(&mut env);
        error!("open_app_settings failed: {e:?}");
    }
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
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return;
    };
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> anyhow::Result<()> {
        let action = env.new_string("android.intent.action.OPEN_DOCUMENT")?;
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;
        let mime = env.new_string(mime)?;
        env.call_method(
            &intent,
            "setType",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&mime)],
        )?;
        let category = env.new_string("android.intent.category.OPENABLE")?;
        env.call_method(
            &intent,
            "addCategory",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&category)],
        )?;

        // This only opens the picker. Receiving its content:// result requires
        // an Activity callback and is intentionally not claimed as a loaded file.
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })();

    if let Err(e) = result {
        clear_pending_java_exception(&mut env);
        warn!("launch_saf_open_document failed: {e:?}");
    }
}

pub fn share_text(text: &str) -> bool {
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else {
        return false;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return false;
    };
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let result = (|| -> anyhow::Result<()> {
        let action = env.new_string("android.intent.action.SEND")?;
        let send = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;

        let mime = env.new_string("text/plain")?;
        env.call_method(
            &send,
            "setType",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&mime)],
        )?;

        let extra_text = env.new_string("android.intent.extra.TEXT")?;
        let value = env.new_string(text)?;
        env.call_method(
            &send,
            "putExtra",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&extra_text), JValue::Object(&value)],
        )?;

        let title = env.new_string("Share decompiled output")?;
        let chooser = env
            .call_static_method(
                "android/content/Intent",
                "createChooser",
                "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
                &[JValue::Object(&send), JValue::Object(&title)],
            )?
            .l()?;

        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&chooser)],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => true,
        Err(e) => {
            clear_pending_java_exception(&mut env);
            error!("Android share failed: {e:?}");
            false
        }
    }
}

// Toast.makeText() was previously called from android_main, which is not the
// Java UI/Looper thread. Some Android versions abort the native process in that
// situation. Keep UI feedback in the egui status bar until a Java UI-thread
// bridge is added.
pub fn toast(msg: &str) {
    info!("Topaz notification: {msg}");
}
