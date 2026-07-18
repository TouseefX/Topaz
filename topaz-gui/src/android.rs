// Android-specific utilities for Topaz
// - system clipboard (direct JNI)
// - runtime storage permissions (android-permissions)
// - in-app file browser
// - SAF file picker fallback
// - wakelock (PowerManager via JNI)
// - notification (NotificationManager via JNI)
#![cfg(target_os = "android")]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use eframe::egui;
use jni::objects::JValue;
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
// JNI helper — clear any pending Java exception so we don't SIGABRT
// --------------------------------------------------------------------

fn clear_pending_java_exception(env: &mut jni::JNIEnv<'_>) {
    if matches!(env.exception_check(), Ok(true)) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

/// Attach to the JVM from the ndk_context, call the closure,
/// and clear any Java exception on failure.
fn with_jni<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&mut jni::JNIEnv<'_>, &jni::objects::JObject<'_>) -> Result<T, String>,
{
    let ctx = ndk_context::android_context();
    let vm = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) })
        .map_err(|e| format!("JavaVM::from_raw: {e:?}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach_current_thread: {e:?}"))?;
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    let result = f(&mut env, &activity);
    if result.is_err() {
        clear_pending_java_exception(&mut env);
    }
    result
}

fn app_package_name(
    env: &mut jni::JNIEnv<'_>,
    activity: &jni::objects::JObject<'_>,
) -> Result<String, String> {
    let name_obj = env
        .call_method(activity, "getPackageName", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("getPackageName(): {e:?}"))?
        .l()
        .map_err(|e| format!("getPackageName().l(): {e:?}"))?;
    let jstr = jni::objects::JString::from(name_obj);
    let s: String = env
        .get_string(&jstr)
        .map_err(|e| format!("get_string(package_name): {e:?}"))?
        .into();
    Ok(s)
}

// --------------------------------------------------------------------
// Clipboard
// --------------------------------------------------------------------

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
            || !env
                .call_method(&manager, "hasPrimaryClip", "()Z", &[])?
                .z()?
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

    if sdk >= 30 {
        let granted = is_manage_external_storage_granted();
        set_permission_status(PermissionStatus {
            checked_at: Some(Instant::now()),
            manage_external: granted,
            last_message: if granted {
                "All-files access is enabled.".into()
            } else {
                "Enable 'Allow access to manage all files' for direct-path mode.".into()
            },
            ..Default::default()
        });
        if !granted {
            open_app_settings();
        }
        return;
    }

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

fn is_manage_external_storage_granted() -> bool {
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
        let is_mgr: bool = env
            .call_static_method(env_class, "isExternalStorageManager", "()Z", &[])?
            .z()?;
        Ok(is_mgr)
    })()
    .unwrap_or(false)
}

pub fn open_app_settings() {
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
            filter_ext: vec![
                "luac".into(),
                "lua".into(),
                "bin".into(),
                "txt".into(),
                "luau".into(),
                "dat".into(),
            ],
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
                    let entry = Entry {
                        name,
                        path,
                        is_dir,
                        size,
                    };
                    if is_dir {
                        dirs.push(entry);
                    } else {
                        if self.filter_ext.is_empty()
                            || entry
                                .name
                                .rsplit('.')
                                .next()
                                .map(|ext| {
                                    self.filter_ext.iter().any(|f| f.eq_ignore_ascii_case(ext))
                                })
                                .unwrap_or(false)
                        {
                            files.push(entry);
                        } else {
                            files.push(entry);
                        }
                    }
                }
                dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
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
            if ui.button("⬆ Up").clicked() {
                self.go_up();
            }
            if ui.button("⟳ Refresh").clicked() {
                self.refresh();
            }
            ui.checkbox(&mut self.show_hidden, "hidden");
            ui.separator();
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

        egui::ScrollArea::vertical()
            .max_height(380.0)
            .show(ui, |ui| {
                if self.entries.is_empty() {
                    ui.weak("(empty — check permissions)");
                    return;
                }
                egui::Grid::new("fb_grid")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
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
                self.filter_ext = filter_str
                    .split(',')
                    .map(|s| s.trim().trim_start_matches('.').to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
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
// SAF picker
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

// Legacy toast — kept as log-only since NativeActivity doesn't have a UI thread bridge
pub fn toast(msg: &str) {
    info!("Topaz notification: {msg}");
}

// ====================================================================
// Wakelock — PowerManager JNI
// ====================================================================

// Wakelock level constants matching android.os.PowerManager
pub mod wake_lock_level {
    /// Partial wakelock: CPU stays on, screen may be off.
    pub const PARTIAL: i32 = 0x00000001;
    /// Full wakelock: CPU + screen bright.
    pub const FULL: i32 = 0x0000001a;
    /// Screen dim wakelock: CPU + screen dim.
    pub const SCREEN_DIM: i32 = 0x00000006;
    /// Screen bright wakelock: CPU + screen bright.
    pub const SCREEN_BRIGHT: i32 = 0x0000000a;
    /// Proximity screen off (API 21+).
    pub const PROXIMITY_SCREEN_OFF: i32 = 0x00000020;
}

/// Wakelock handle that automatically releases on drop.
pub struct Wakelock {
    // We store a GlobalRef so the Java object survives across JNI calls.
    inner: Option<jni::objects::GlobalRef>,
    #[allow(dead_code)]
    tag: String,
}

// Static lock for the global wakelock reference
static WAKELOCK: OnceLock<std::sync::Mutex<Option<Wakelock>>> = OnceLock::new();

fn wakelock_mutex() -> &'static std::sync::Mutex<Option<Wakelock>> {
    WAKELOCK.get_or_init(|| std::sync::Mutex::new(None))
}

/// Acquire a wakelock with the given level.
///
/// `level` should be one of the `wake_lock_level::*` constants.
/// `tag` is a debug label for the wakelock (shown in dumpsys).
///
/// If a wakelock is already held, it is released first.
pub fn acquire_wakelock(level: i32, tag: &str) -> bool {
    // First release any existing wakelock
    release_wakelock();

    let result = with_jni(|env, activity| {
        // Get PowerManager system service
        let service_str = env
            .new_string("power")
            .map_err(|e| format!("new_string: {e:?}"))?;
        let power_manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_str)],
            )
            .map_err(|e| format!("getSystemService: {e:?}"))?
            .l()
            .map_err(|e| format!("getSystemService.l(): {e:?}"))?;

        if power_manager.is_null() {
            return Err("PowerManager is null".into());
        }

        let tag_obj = env
            .new_string(tag)
            .map_err(|e| format!("new_string(tag): {e:?}"))?;

        // PowerManager.newWakeLock(int level, String tag)
        let wakelock_obj = env
            .call_method(
                &power_manager,
                "newWakeLock",
                "(ILjava/lang/String;)Landroid/os/PowerManager$WakeLock;",
                &[JValue::Int(level), JValue::Object(&tag_obj)],
            )
            .map_err(|e| format!("newWakeLock: {e:?}"))?
            .l()
            .map_err(|e| format!("newWakeLock.l(): {e:?}"))?;

        if wakelock_obj.is_null() {
            return Err("newWakeLock returned null".into());
        }

        // Acquire the wakelock
        env.call_method(&wakelock_obj, "acquire", "()V", &[])
            .map_err(|e| format!("wakelock.acquire(): {e:?}"))?;

        // Store as a global reference
        let global_ref = env
            .new_global_ref(wakelock_obj)
            .map_err(|e| format!("new_global_ref: {e:?}"))?;

        // Store in our static
        let mut guard = wakelock_mutex().lock().unwrap();
        *guard = Some(Wakelock {
            inner: Some(global_ref),
            tag: tag.to_string(),
        });

        info!("Wakelock acquired: level=0x{level:08x}, tag=\"{tag}\"");
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("acquire_wakelock failed: {e}");
            false
        }
    }
}

/// Acquire a partial wakelock (CPU on, screen may be off).
/// This is the most common type for background server work.
pub fn acquire_partial_wakelock(tag: &str) -> bool {
    acquire_wakelock(wake_lock_level::PARTIAL, tag)
}

/// Release any currently held wakelock.
pub fn release_wakelock() -> bool {
    let mut guard = wakelock_mutex().lock().unwrap();
    if let Some(wl) = guard.take() {
        // Drop the Wakelock — its Drop impl calls PowerManager.WakeLock.release()
        drop(wl);
        info!("Wakelock released");
        true
    } else {
        false // no wakelock was held
    }
}

impl Wakelock {
    fn release_inner(&mut self) {
        let Some(ref global_ref) = self.inner else {
            return;
        };
        let result = with_jni(|env, _activity| {
            let wakelock_obj = global_ref.as_obj();
            env.call_method(wakelock_obj, "release", "()V", &[])
                .map_err(|e| format!("wakelock.release(): {e:?}"))?;
            Ok(())
        });
        if let Err(e) = result {
            error!("release_wakelock (inner) failed: {e}");
        }
        self.inner = None;
    }
}

impl Drop for Wakelock {
    fn drop(&mut self) {
        self.release_inner();
    }
}

/// Check whether a wakelock is currently held.
pub fn is_wakelock_held() -> bool {
    wakelock_mutex().lock().unwrap().is_some()
}

// ====================================================================
// Notification — NotificationManager JNI
// ====================================================================

/// Notification importance constants matching android.app.NotificationManager
pub mod notification_importance {
    /// Default importance — makes a sound and appears in the shade.
    pub const DEFAULT: i32 = 3;
    /// High importance — heads-up display.
    pub const HIGH: i32 = 4;
    /// Low importance — no sound.
    pub const LOW: i32 = 2;
    /// Min importance — no sound, no status bar ticker.
    pub const MIN: i32 = 1;
    /// None — no UX interruption.
    pub const NONE: i32 = 0;
}

/// Create a notification channel (required on Android 8.0+ / API 26+).
///
/// API 25 and below will silently ignore this call since NotificationChannel
/// does not exist — notifications will still be delivered via the legacy path.
///
/// `id` — the channel ID (e.g. "server_status").
/// `name` — human-readable channel name shown in system Settings.
/// `importance` — one of `notification_importance::*`.
pub fn create_notification_channel(id: &str, name: &str, importance: i32) -> bool {
    let sdk = android_sdk_int();
    if sdk < 26 {
        // NotificationChannel is API 26+
        warn!("NotificationChannel requires API 26+ (current SDK: {sdk}), skipping");
        return false;
    }

    let result = with_jni(|env, activity| {
        let service_str = env
            .new_string("notification")
            .map_err(|e| format!("new_string(notification): {e:?}"))?;
        let notification_manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_str)],
            )
            .map_err(|e| format!("getSystemService(notification): {e:?}"))?
            .l()
            .map_err(|e| format!("getSystemService.l(): {e:?}"))?;

        if notification_manager.is_null() {
            return Err("NotificationManager is null".into());
        }

        let channel_id = env
            .new_string(id)
            .map_err(|e| format!("new_string(channel_id): {e:?}"))?;
        let channel_name = env
            .new_string(name)
            .map_err(|e| format!("new_string(channel_name): {e:?}"))?;

        // NotificationChannel(String id, CharSequence name, int importance)
        let channel = env
            .new_object(
                "android/app/NotificationChannel",
                "(Ljava/lang/String;Ljava/lang/CharSequence;I)V",
                &[
                    JValue::Object(&channel_id),
                    JValue::Object(&channel_name),
                    JValue::Int(importance),
                ],
            )
            .map_err(|e| format!("new NotificationChannel: {e:?}"))?;

        // NotificationManager.createNotificationChannel(NotificationChannel)
        env.call_method(
            &notification_manager,
            "createNotificationChannel",
            "(Landroid/app/NotificationChannel;)V",
            &[JValue::Object(&channel)],
        )
        .map_err(|e| format!("createNotificationChannel: {e:?}"))?;

        info!(
            "Notification channel created: id=\"{id}\", name=\"{name}\", importance={importance}"
        );
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("create_notification_channel failed: {e}");
            false
        }
    }
}

/// Show a persistent notification.
///
/// On API 26+ the channel must have been created first via
/// `create_notification_channel()`.
///
/// `channel_id` — must match a channel created via `create_notification_channel`.
/// `title` — notification title.
/// `text` — notification body text.
/// `notif_id` — unique int identifier for this notification (used with `cancel_notification`).
pub fn show_notification(channel_id: &str, title: &str, text: &str, notif_id: i32) -> bool {
    let result = with_jni(|env, activity| {
        let service_str = env
            .new_string("notification")
            .map_err(|e| format!("new_string(notification): {e:?}"))?;
        let notification_manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_str)],
            )
            .map_err(|e| format!("getSystemService(notification): {e:?}"))?
            .l()
            .map_err(|e| format!("getSystemService.l(): {e:?}"))?;

        if notification_manager.is_null() {
            return Err("NotificationManager is null".into());
        }

        let sdk = android_sdk_int();
        let sdk_26 = sdk >= 26;

        if sdk_26 {
            // --- API 26+ path: Notification.Builder ---
            let channel_id_obj = env
                .new_string(channel_id)
                .map_err(|e| format!("new_string(channel_id): {e:?}"))?;

            // new Notification.Builder(Context, String channelId)
            let builder = env
                .new_object(
                    "android/app/Notification$Builder",
                    "(Landroid/content/Context;Ljava/lang/String;)V",
                    &[JValue::Object(activity), JValue::Object(&channel_id_obj)],
                )
                .map_err(|e| format!("new Notification.Builder: {e:?}"))?;

            let title_obj = env
                .new_string(title)
                .map_err(|e| format!("new_string(title): {e:?}"))?;
            let text_obj = env
                .new_string(text)
                .map_err(|e| format!("new_string(text): {e:?}"))?;

            // .setContentTitle(CharSequence)
            env.call_method(
                &builder,
                "setContentTitle",
                "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;",
                &[JValue::Object(&title_obj)],
            )
            .map_err(|e| format!("setContentTitle: {e:?}"))?;

            // .setContentText(CharSequence)
            env.call_method(
                &builder,
                "setContentText",
                "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;",
                &[JValue::Object(&text_obj)],
            )
            .map_err(|e| format!("setContentText: {e:?}"))?;

            // .setSmallIcon(...) — use android.R.drawable.ic_dialog_info (17301655)
            // We use a built-in system icon to avoid needing custom resources.
            env.call_method(
                &builder,
                "setSmallIcon",
                "(I)Landroid/app/Notification$Builder;",
                &[JValue::Int(17301655)], // android.R.drawable.ic_dialog_info
            )
            .map_err(|e| format!("setSmallIcon: {e:?}"))?;

            // .setAutoCancel(true) — dismiss when tapped
            env.call_method(
                &builder,
                "setAutoCancel",
                "(Z)Landroid/app/Notification$Builder;",
                &[JValue::Bool(1)],
            )
            .map_err(|e| format!("setAutoCancel: {e:?}"))?;

            // Build the notification
            let notification = env
                .call_method(&builder, "build", "()Landroid/app/Notification;", &[])
                .map_err(|e| format!("build(): {e:?}"))?
                .l()
                .map_err(|e| format!("build().l(): {e:?}"))?;

            // NotificationManager.notify(int id, Notification notification)
            env.call_method(
                &notification_manager,
                "notify",
                "(ILandroid/app/Notification;)V",
                &[JValue::Int(notif_id), JValue::Object(&notification)],
            )
            .map_err(|e| format!("notify(): {e:?}"))?;
        } else {
            // --- Pre-API 26 legacy path: Notification.Builder (without channel) ---
            let builder = env
                .new_object(
                    "android/app/Notification$Builder",
                    "(Landroid/content/Context;)V",
                    &[JValue::Object(activity)],
                )
                .map_err(|e| format!("new Notification.Builder (legacy): {e:?}"))?;

            let title_obj = env
                .new_string(title)
                .map_err(|e| format!("new_string(title): {e:?}"))?;
            let text_obj = env
                .new_string(text)
                .map_err(|e| format!("new_string(text): {e:?}"))?;

            env.call_method(
                &builder,
                "setContentTitle",
                "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;",
                &[JValue::Object(&title_obj)],
            )
            .map_err(|e| format!("setContentTitle (legacy): {e:?}"))?;

            env.call_method(
                &builder,
                "setContentText",
                "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;",
                &[JValue::Object(&text_obj)],
            )
            .map_err(|e| format!("setContentText (legacy): {e:?}"))?;

            env.call_method(
                &builder,
                "setSmallIcon",
                "(I)Landroid/app/Notification$Builder;",
                &[JValue::Int(17301655)],
            )
            .map_err(|e| format!("setSmallIcon (legacy): {e:?}"))?;

            env.call_method(
                &builder,
                "setAutoCancel",
                "(Z)Landroid/app/Notification$Builder;",
                &[JValue::Bool(1)],
            )
            .map_err(|e| format!("setAutoCancel (legacy): {e:?}"))?;

            let notification = env
                .call_method(&builder, "build", "()Landroid/app/Notification;", &[])
                .map_err(|e| format!("build() (legacy): {e:?}"))?
                .l()
                .map_err(|e| format!("build().l() (legacy): {e:?}"))?;

            env.call_method(
                &notification_manager,
                "notify",
                "(ILandroid/app/Notification;)V",
                &[JValue::Int(notif_id), JValue::Object(&notification)],
            )
            .map_err(|e| format!("notify() (legacy): {e:?}"))?;
        }

        info!("Notification shown: id={notif_id}, channel=\"{channel_id}\", title=\"{title}\"");
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("show_notification failed: {e}");
            false
        }
    }
}

/// Cancel a notification previously shown with `show_notification`.
pub fn cancel_notification(notif_id: i32) -> bool {
    let result = with_jni(|env, activity| {
        let service_str = env
            .new_string("notification")
            .map_err(|e| format!("new_string(notification): {e:?}"))?;
        let notification_manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_str)],
            )
            .map_err(|e| format!("getSystemService(notification): {e:?}"))?
            .l()
            .map_err(|e| format!("getSystemService.l(): {e:?}"))?;

        if notification_manager.is_null() {
            return Err("NotificationManager is null".into());
        }

        env.call_method(
            &notification_manager,
            "cancel",
            "(I)V",
            &[JValue::Int(notif_id)],
        )
        .map_err(|e| format!("cancel(): {e:?}"))?;

        info!("Notification cancelled: id={notif_id}");
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("cancel_notification failed: {e}");
            false
        }
    }
}

/// Show a persistent notification that **cannot be swiped away** (ongoing=true).
pub fn show_ongoing_notification(channel_id: &str, title: &str, text: &str, notif_id: i32) -> bool {
    let result = with_jni(|env, activity| {
        let service_str = env
            .new_string("notification")
            .map_err(|e| format!("new_string(notification): {e:?}"))?;
        let notification_manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_str)],
            )
            .map_err(|e| format!("getSystemService(notification): {e:?}"))?
            .l()
            .map_err(|e| format!("getSystemService.l(): {e:?}"))?;

        if notification_manager.is_null() {
            return Err("NotificationManager is null".into());
        }

        let sdk = android_sdk_int();
        let sdk_26 = sdk >= 26;

        if sdk_26 {
            let channel_id_obj = env
                .new_string(channel_id)
                .map_err(|e| format!("new_string(channel_id): {e:?}"))?;

            let builder = env
                .new_object(
                    "android/app/Notification$Builder",
                    "(Landroid/content/Context;Ljava/lang/String;)V",
                    &[JValue::Object(activity), JValue::Object(&channel_id_obj)],
                )
                .map_err(|e| format!("new Notification.Builder: {e:?}"))?;

            let title_obj = env
                .new_string(title)
                .map_err(|e| format!("new_string(title): {e:?}"))?;
            let text_obj = env
                .new_string(text)
                .map_err(|e| format!("new_string(text): {e:?}"))?;

            env.call_method(&builder, "setContentTitle", "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;", &[JValue::Object(&title_obj)])
                .map_err(|e| format!("setContentTitle: {e:?}"))?;
            env.call_method(&builder, "setContentText", "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;", &[JValue::Object(&text_obj)])
                .map_err(|e| format!("setContentText: {e:?}"))?;
            env.call_method(&builder, "setSmallIcon", "(I)Landroid/app/Notification$Builder;", &[JValue::Int(17301655)])
                .map_err(|e| format!("setSmallIcon: {e:?}"))?;
            // Ongoing = true → user cannot swipe it away (like Termux foreground service)
            env.call_method(&builder, "setOngoing", "(Z)Landroid/app/Notification$Builder;", &[JValue::Bool(1)])
                .map_err(|e| format!("setOngoing: {e:?}"))?;
            // AutoCancel = false → tapping does NOT dismiss it
            env.call_method(&builder, "setAutoCancel", "(Z)Landroid/app/Notification$Builder;", &[JValue::Bool(0)])
                .map_err(|e| format!("setAutoCancel: {e:?}"))?;

            let notification = env
                .call_method(&builder, "build", "()Landroid/app/Notification;", &[])
                .map_err(|e| format!("build(): {e:?}"))?
                .l()
                .map_err(|e| format!("build().l(): {e:?}"))?;

            env.call_method(&notification_manager, "notify", "(ILandroid/app/Notification;)V", &[JValue::Int(notif_id), JValue::Object(&notification)])
                .map_err(|e| format!("notify(): {e:?}"))?;
        } else {
            let builder = env
                .new_object("android/app/Notification$Builder", "(Landroid/content/Context;)V", &[JValue::Object(activity)])
                .map_err(|e| format!("new Notification.Builder (legacy): {e:?}"))?;

            let title_obj = env.new_string(title).map_err(|e| format!("new_string(title): {e:?}"))?;
            let text_obj = env.new_string(text).map_err(|e| format!("new_string(text): {e:?}"))?;

            env.call_method(&builder, "setContentTitle", "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;", &[JValue::Object(&title_obj)])
                .map_err(|e| format!("setContentTitle (legacy): {e:?}"))?;
            env.call_method(&builder, "setContentText", "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;", &[JValue::Object(&text_obj)])
                .map_err(|e| format!("setContentText (legacy): {e:?}"))?;
            env.call_method(&builder, "setSmallIcon", "(I)Landroid/app/Notification$Builder;", &[JValue::Int(17301655)])
                .map_err(|e| format!("setSmallIcon (legacy): {e:?}"))?;
            env.call_method(&builder, "setOngoing", "(Z)Landroid/app/Notification$Builder;", &[JValue::Bool(1)])
                .map_err(|e| format!("setOngoing (legacy): {e:?}"))?;
            env.call_method(&builder, "setAutoCancel", "(Z)Landroid/app/Notification$Builder;", &[JValue::Bool(0)])
                .map_err(|e| format!("setAutoCancel (legacy): {e:?}"))?;

            let notification = env
                .call_method(&builder, "build", "()Landroid/app/Notification;", &[])
                .map_err(|e| format!("build() (legacy): {e:?}"))?
                .l()
                .map_err(|e| format!("build().l() (legacy): {e:?}"))?;

            env.call_method(&notification_manager, "notify", "(ILandroid/app/Notification;)V", &[JValue::Int(notif_id), JValue::Object(&notification)])
                .map_err(|e| format!("notify() (legacy): {e:?}"))?;
        }

        info!("Ongoing notification shown: id={notif_id}, channel=\"{channel_id}\", title=\"{title}\"");
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("show_ongoing_notification failed: {e}");
            false
        }
    }
}

// ── Keep-alive: wakelock + real foreground-service notification ──

/// Build an explicit Intent targeting `<pkg>.KeepAliveService` and run `f` with it.
fn with_keepalive_intent<F>(f: F) -> Result<(), String>
where
    F: FnOnce(&mut jni::JNIEnv<'_>, &jni::objects::JObject<'_>, &jni::objects::JObject<'_>) -> Result<(), String>,
{
    with_jni(|env, activity| {
        let pkg = app_package_name(env, activity)?;
        let service_class = format!("{pkg}.KeepAliveService");

        let intent = env
            .new_object("android/content/Intent", "()V", &[])
            .map_err(|e| format!("new Intent: {e:?}"))?;
        let pkg_str = env
            .new_string(&pkg)
            .map_err(|e| format!("new_string(pkg): {e:?}"))?;
        let cls_str = env
            .new_string(&service_class)
            .map_err(|e| format!("new_string(cls): {e:?}"))?;
        env.call_method(
            &intent,
            "setClassName",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&pkg_str), JValue::Object(&cls_str)],
        )
        .map_err(|e| format!("setClassName: {e:?}"))?;

        f(env, activity, &intent)
    })
}

// Both the "Persistent" toggle and the running HTTP server want the service alive at
// the same time in some cases. Ref-count so whichever one calls stop() first doesn't
// pull the notification out from under the other.
static KEEPALIVE_REFS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Start (or update the text of) the KeepAliveService — a genuine Android foreground
/// service. This is what actually prevents the notification from being swiped away;
/// see `KeepAliveService.java`. Safe to call repeatedly (e.g. to refresh the notification
/// text); each call registers one "owner" that must later call `stop_keepalive_service`.
pub fn start_keepalive_service(text: &str) -> bool {
    KEEPALIVE_REFS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    jni_start_keepalive_service(text)
}

fn jni_start_keepalive_service(text: &str) -> bool {
    let result = with_keepalive_intent(|env, activity, intent| {
        let key = env
            .new_string("text")
            .map_err(|e| format!("new_string(key): {e:?}"))?;
        let value = env
            .new_string(text)
            .map_err(|e| format!("new_string(value): {e:?}"))?;
        env.call_method(
            intent,
            "putExtra",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&key), JValue::Object(&value)],
        )
        .map_err(|e| format!("putExtra: {e:?}"))?;

        let sdk = android_sdk_int();
        let method = if sdk >= 26 {
            "startForegroundService"
        } else {
            "startService"
        };
        env.call_method(
            activity,
            method,
            "(Landroid/content/Intent;)Landroid/content/ComponentName;",
            &[JValue::Object(intent)],
        )
        .map_err(|e| format!("{method}: {e:?}"))?;

        info!("KeepAliveService started via {method} (\"{text}\")");
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("start_keepalive_service failed: {e}");
            false
        }
    }
}

/// Release one "owner" of the KeepAliveService. Only actually stops the service (and its
/// notification) once every caller that started it has also called this.
pub fn stop_keepalive_service() -> bool {
    use std::sync::atomic::Ordering;
    let prev = KEEPALIVE_REFS
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| Some(v.saturating_sub(1)))
        .unwrap_or(0);
    if prev > 1 {
        info!(
            "stop_keepalive_service: {} other owner(s) still active, leaving service running",
            prev - 1
        );
        return true;
    }
    jni_stop_keepalive_service()
}

fn jni_stop_keepalive_service() -> bool {
    let result = with_keepalive_intent(|env, activity, intent| {
        env.call_method(
            activity,
            "stopService",
            "(Landroid/content/Intent;)Z",
            &[JValue::Object(intent)],
        )
        .map_err(|e| format!("stopService: {e:?}"))?;
        info!("KeepAliveService stopped");
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("stop_keepalive_service failed: {e}");
            false
        }
    }
}

/// Enable keepalive mode: acquire a partial wakelock + start the real foreground service.
/// When the user opens the app again, this persists (state is saved), so it re-activates on boot.
pub fn enable_keepalive() -> bool {
    let wl = acquire_partial_wakelock("TopazKeepAlive");
    let svc = start_keepalive_service("Tap to open — Topaz is running in the background.");
    if wl && svc {
        info!("Keepalive enabled: wakelock + foreground service");
    } else if wl {
        warn!("Keepalive: wakelock OK but foreground service failed to start");
    } else if svc {
        warn!("Keepalive: foreground service OK but wakelock failed");
    } else {
        error!("Keepalive: both wakelock and foreground service failed");
    }
    wl || svc
}

/// Disable keepalive mode: release wakelock + stop the foreground service.
pub fn disable_keepalive() -> bool {
    let wl = release_wakelock();
    let svc = stop_keepalive_service();
    if wl || svc {
        info!("Keepalive disabled");
        true
    } else {
        false
    }
}

// ── Remote server control (the actual HTTP server lives in KeepAliveService's
// separate `:service` process — see Cargo.toml `process = ":service"` and
// service_entry.rs). These just send it commands over an Intent; reading its
// status back happens out-of-band via the shared JSON file (ipc_stats.rs). ──

fn put_string_extra(
    env: &mut jni::JNIEnv<'_>,
    intent: &jni::objects::JObject<'_>,
    key: &str,
    value: &str,
) -> Result<(), String> {
    let key = env.new_string(key).map_err(|e| format!("new_string(key): {e:?}"))?;
    let value = env.new_string(value).map_err(|e| format!("new_string(value): {e:?}"))?;
    env.call_method(
        intent,
        "putExtra",
        "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&key), JValue::Object(&value)],
    )
    .map_err(|e| format!("putExtra(String): {e:?}"))?;
    Ok(())
}

fn put_int_extra(
    env: &mut jni::JNIEnv<'_>,
    intent: &jni::objects::JObject<'_>,
    key: &str,
    value: i32,
) -> Result<(), String> {
    let key = env.new_string(key).map_err(|e| format!("new_string(key): {e:?}"))?;
    env.call_method(
        intent,
        "putExtra",
        "(Ljava/lang/String;I)Landroid/content/Intent;",
        &[JValue::Object(&key), JValue::Int(value)],
    )
    .map_err(|e| format!("putExtra(int): {e:?}"))?;
    Ok(())
}

fn put_bool_extra(
    env: &mut jni::JNIEnv<'_>,
    intent: &jni::objects::JObject<'_>,
    key: &str,
    value: bool,
) -> Result<(), String> {
    let key = env.new_string(key).map_err(|e| format!("new_string(key): {e:?}"))?;
    env.call_method(
        intent,
        "putExtra",
        "(Ljava/lang/String;Z)Landroid/content/Intent;",
        &[JValue::Object(&key), JValue::Bool(value as u8)],
    )
    .map_err(|e| format!("putExtra(bool): {e:?}"))?;
    Ok(())
}

/// `Context.getFilesDir().getPath()` — app-private storage, same path from
/// either process since both belong to the same app/UID. Used as the
/// directory for the cross-process stats file.
pub fn files_dir() -> Option<String> {
    with_jni(files_dir_inner).ok()
}

pub struct RemoteServerConfig {
    pub port: u16,
    pub luau: bool,
    pub lua51: bool,
    pub encode_key: u8,
}

/// Tell KeepAliveService (in its own process) to start the HTTP server.
/// Also registers this as an "owner" of the service's foreground lifetime,
/// same as the Persistent toggle — see `KEEPALIVE_REFS`.
pub fn start_remote_server(cfg: RemoteServerConfig) -> bool {
    KEEPALIVE_REFS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let result = with_keepalive_intent(|env, activity, intent| {
        put_string_extra(env, intent, "cmd", "start_server")?;
        put_int_extra(env, intent, "port", cfg.port as i32)?;
        put_bool_extra(env, intent, "luau", cfg.luau)?;
        put_bool_extra(env, intent, "lua51", cfg.lua51)?;
        put_int_extra(env, intent, "encode_key", cfg.encode_key as i32)?;

        let dir = files_dir_inner(env, activity)?;
        put_string_extra(env, intent, "files_dir", &dir)?;

        let sdk = android_sdk_int();
        let method = if sdk >= 26 { "startForegroundService" } else { "startService" };
        env.call_method(
            activity,
            method,
            "(Landroid/content/Intent;)Landroid/content/ComponentName;",
            &[JValue::Object(intent)],
        )
        .map_err(|e| format!("{method}: {e:?}"))?;

        info!("start_remote_server: sent start_server via {method} (port {})", cfg.port);
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("start_remote_server failed: {e}");
            false
        }
    }
}

/// Tell KeepAliveService to stop the HTTP server, and release this call's
/// "owner" stake in the service's foreground lifetime (the service itself
/// stays up if the Persistent toggle is still separately holding it alive).
pub fn stop_remote_server() -> bool {
    let cmd_ok = with_keepalive_intent(|env, activity, intent| {
        put_string_extra(env, intent, "cmd", "stop_server")?;
        let sdk = android_sdk_int();
        let method = if sdk >= 26 { "startForegroundService" } else { "startService" };
        env.call_method(
            activity,
            method,
            "(Landroid/content/Intent;)Landroid/content/ComponentName;",
            &[JValue::Object(intent)],
        )
        .map_err(|e| format!("{method}: {e:?}"))?;
        info!("stop_remote_server: sent stop_server cmd");
        Ok(())
    })
    .is_ok();

    use std::sync::atomic::Ordering;
    let prev = KEEPALIVE_REFS
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| Some(v.saturating_sub(1)))
        .unwrap_or(0);
    let stop_ok = if prev <= 1 { jni_stop_keepalive_service() } else { true };

    cmd_ok && stop_ok
}

fn files_dir_inner(
    env: &mut jni::JNIEnv<'_>,
    activity: &jni::objects::JObject<'_>,
) -> Result<String, String> {
    let file_obj = env
        .call_method(activity, "getFilesDir", "()Ljava/io/File;", &[])
        .map_err(|e| format!("getFilesDir: {e:?}"))?
        .l()
        .map_err(|e| format!("getFilesDir().l(): {e:?}"))?;
    let path_obj = env
        .call_method(&file_obj, "getPath", "()Ljava/lang/String;", &[])
        .map_err(|e| format!("getPath: {e:?}"))?
        .l()
        .map_err(|e| format!("getPath().l(): {e:?}"))?;
    let jstr = jni::objects::JString::from(path_obj);
    env.get_string(&jstr)
        .map_err(|e| format!("get_string(files_dir): {e:?}"))
        .map(|s| s.into())
}


/// Cancel all notifications from this app.
pub fn cancel_all_notifications() -> bool {
    let result = with_jni(|env, activity| {
        let service_str = env
            .new_string("notification")
            .map_err(|e| format!("new_string(notification): {e:?}"))?;
        let notification_manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_str)],
            )
            .map_err(|e| format!("getSystemService(notification): {e:?}"))?
            .l()
            .map_err(|e| format!("getSystemService.l(): {e:?}"))?;

        if notification_manager.is_null() {
            return Err("NotificationManager is null".into());
        }

        env.call_method(&notification_manager, "cancelAll", "()V", &[])
            .map_err(|e| format!("cancelAll(): {e:?}"))?;

        info!("All notifications cancelled");
        Ok(())
    });

    match result {
        Ok(()) => true,
        Err(e) => {
            error!("cancel_all_notifications failed: {e}");
            false
        }
    }
}
