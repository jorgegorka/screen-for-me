use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::capture::{self, CaptureError, CaptureMode, CaptureOutcome};
use crate::history::{CaptureEntry, History};
use crate::settings::{EditorPrefs, EditorPrefsStore, OverlayPosition, Settings, SettingsStore};

pub struct AppState {
    pub history: History,
    pub settings: SettingsStore,
    pub editor_prefs: EditorPrefsStore,
    pub editor_target: Mutex<Option<String>>,
    pub timer_seconds: Mutex<u32>,
    pub last_capture_mode: Mutex<CaptureMode>,
    pub scroll_stop: Arc<AtomicBool>,
    pub scroll_running: Arc<AtomicBool>,
    pub overlay_follow_epoch: Arc<AtomicU64>,
    pub overlay_drag_active: AtomicBool,
    pub overlay_panels: AtomicUsize,
}

impl AppState {
    pub fn new(history: History, settings: SettingsStore, editor_prefs: EditorPrefsStore) -> Self {
        Self {
            history,
            settings,
            editor_prefs,
            editor_target: Mutex::new(None),
            timer_seconds: Mutex::new(5),
            last_capture_mode: Mutex::new(CaptureMode::Fullscreen),
            scroll_stop: Arc::new(AtomicBool::new(false)),
            scroll_running: Arc::new(AtomicBool::new(false)),
            overlay_follow_epoch: Arc::new(AtomicU64::new(0)),
            overlay_drag_active: AtomicBool::new(false),
            overlay_panels: AtomicUsize::new(1),
        }
    }
}

fn err_string(err: impl ToString) -> String {
    err.to_string()
}

pub fn trigger_capture(app: &AppHandle, mode: CaptureMode) {
    *app.state::<AppState>().last_capture_mode.lock().unwrap() = mode;
    spawn_capture(app.clone(), mode, None);
}

fn spawn_capture(app: AppHandle, mode: CaptureMode, delay: Option<Duration>) {
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(delay) = delay {
            std::thread::sleep(delay);
        }
        if let Err(err) = capture_and_publish(&app, mode) {
            eprintln!("capture failed: {err}");
        }
    });
}

fn capture_and_publish(app: &AppHandle, mode: CaptureMode) -> Result<(), CaptureError> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
    let state = app.state::<AppState>();
    let dest = state.history.new_capture_path();
    match capture::capture(mode, &dest)? {
        CaptureOutcome::Cancelled => Ok(()),
        CaptureOutcome::Captured(path) => {
            publish_capture(app, &path);
            Ok(())
        }
    }
}

fn publish_capture(app: &AppHandle, path: &Path) {
    let state = app.state::<AppState>();
    state.history.prune();
    let id = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    if let Some(entry) = state.history.resolve(&id) {
        if state.settings.get().copy_to_clipboard {
            if let Err(err) = copy_capture_to_clipboard(app, &entry) {
                eprintln!("failed to copy capture to clipboard: {err}");
            }
        }
        let _ = app.emit("capture:new", &entry);
        show_overlay(app);
    }
}

const OVERLAY_BASE_WIDTH: f64 = 300.0;
const OVERLAY_BASE_HEIGHT: f64 = 264.0;

#[cfg(target_os = "macos")]
fn cursor_point(_app: &AppHandle) -> Option<(f64, f64)> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let point = CGEvent::new(source).ok()?.location();
    Some((point.x, point.y))
}

#[cfg(not(target_os = "macos"))]
fn cursor_point(app: &AppHandle) -> Option<(f64, f64)> {
    let cursor = app.cursor_position().ok()?;
    Some((cursor.x, cursor.y))
}

fn monitor_under_cursor(app: &AppHandle) -> Option<tauri::Monitor> {
    cursor_point(app).and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten())
}

pub(crate) fn active_monitor(app: &AppHandle) -> Option<tauri::Monitor> {
    monitor_under_cursor(app).or_else(|| app.primary_monitor().ok().flatten())
}

pub(crate) fn monitor_logical_bounds(
    monitor: &tauri::Monitor,
) -> (tauri::LogicalPosition<f64>, tauri::LogicalSize<f64>) {
    let scale = monitor.scale_factor();
    (
        monitor.position().to_logical::<f64>(scale),
        monitor.size().to_logical::<f64>(scale),
    )
}

fn overlay_origin(
    position: OverlayPosition,
    (mon_x, mon_y): (f64, f64),
    (mon_w, mon_h): (f64, f64),
    (width, height): (f64, f64),
) -> (f64, f64) {
    const MARGIN: f64 = 16.0;
    let x = match position {
        OverlayPosition::Left => mon_x + MARGIN,
        OverlayPosition::Center => mon_x + (mon_w - width) / 2.0,
        OverlayPosition::Right => mon_x + mon_w - width - MARGIN,
    };
    (x, mon_y + mon_h - height - MARGIN)
}

fn clamp_panels(requested: usize, panel_height: f64, monitor_height: f64) -> usize {
    const MARGIN: f64 = 16.0;
    let fit = ((monitor_height - 2.0 * MARGIN) / panel_height).floor() as usize;
    requested.clamp(1, fit.max(1))
}

fn panel_height(settings: &Settings) -> f64 {
    OVERLAY_BASE_HEIGHT * settings.overlay_size
}

fn overlay_dimensions(settings: &Settings, panels: usize) -> (f64, f64) {
    (
        OVERLAY_BASE_WIDTH * settings.overlay_size,
        panel_height(settings) * panels.max(1) as f64,
    )
}

fn place_overlay(
    overlay: &tauri::WebviewWindow,
    monitor: &tauri::Monitor,
    settings: &Settings,
    panels: usize,
) {
    let (mon_pos, mon_size) = monitor_logical_bounds(monitor);
    let panels = clamp_panels(panels, panel_height(settings), mon_size.height);
    let (width, height) = overlay_dimensions(settings, panels);
    let _ = overlay.set_size(tauri::LogicalSize::new(width, height));
    let (x, y) = overlay_origin(
        settings.position,
        (mon_pos.x, mon_pos.y),
        (mon_size.width, mon_size.height),
        (width, height),
    );
    let _ = overlay.set_position(tauri::LogicalPosition::new(x, y));
}

fn show_overlay(app: &AppHandle) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    let state = app.state::<AppState>();
    state.overlay_drag_active.store(false, Ordering::SeqCst);
    let settings = state.settings.get();

    let monitor = settings
        .move_to_active_screen
        .then(|| monitor_under_cursor(app))
        .flatten()
        .or_else(|| overlay.primary_monitor().ok().flatten());

    let panels = state.overlay_panels.load(Ordering::SeqCst);
    match monitor {
        Some(monitor) => place_overlay(&overlay, &monitor, &settings, panels),
        None => {
            let (width, height) = overlay_dimensions(&settings, panels);
            let _ = overlay.set_size(tauri::LogicalSize::new(width, height));
        }
    }
    let _ = overlay.show();
    follow_active_monitor(app);
}

#[cfg(target_os = "macos")]
fn left_mouse_button_down() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceButtonState(state_id: i32, button: u32) -> bool;
    }
    unsafe { CGEventSourceButtonState(0, 0) }
}

#[cfg(not(target_os = "macos"))]
fn left_mouse_button_down() -> bool {
    false
}

fn follow_active_monitor(app: &AppHandle) {
    let epochs = app.state::<AppState>().overlay_follow_epoch.clone();
    let epoch = epochs.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        let mut pending: Option<(i32, i32)> = None;
        loop {
            std::thread::sleep(Duration::from_millis(400));
            if epochs.load(Ordering::SeqCst) != epoch {
                return;
            }
            let Some(overlay) = app.get_webview_window("overlay") else {
                return;
            };
            if !overlay.is_visible().unwrap_or(false) {
                return;
            }
            let state = app.state::<AppState>();
            let settings = state.settings.get();
            let dragging = state.overlay_drag_active.load(Ordering::SeqCst);
            if !settings.move_to_active_screen || left_mouse_button_down() || dragging {
                pending = None;
                continue;
            }
            let Some(target) = monitor_under_cursor(&app) else {
                pending = None;
                continue;
            };
            let on_target = overlay
                .current_monitor()
                .ok()
                .flatten()
                .is_some_and(|m| m.position() == target.position());
            if on_target {
                pending = None;
                continue;
            }
            let key = (target.position().x, target.position().y);
            if pending == Some(key) {
                place_overlay(
                    &overlay,
                    &target,
                    &settings,
                    state.overlay_panels.load(Ordering::SeqCst),
                );
                pending = None;
            } else {
                pending = Some(key);
            }
        }
    });
}

#[tauri::command]
pub fn set_overlay_drag_active(state: State<AppState>, active: bool) {
    state.overlay_drag_active.store(active, Ordering::SeqCst);
}

#[tauri::command]
pub fn set_overlay_panels(app: AppHandle, state: State<AppState>, count: usize) -> usize {
    let settings = state.settings.get();
    let Some(overlay) = app.get_webview_window("overlay") else {
        let count = count.max(1);
        state.overlay_panels.store(count, Ordering::SeqCst);
        return count;
    };
    let monitor = overlay
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| overlay.primary_monitor().ok().flatten());
    let clamped = match &monitor {
        Some(monitor) => {
            let (_, mon_size) = monitor_logical_bounds(monitor);
            clamp_panels(count, panel_height(&settings), mon_size.height)
        }
        None => count.max(1),
    };
    state.overlay_panels.store(clamped, Ordering::SeqCst);
    if let Some(monitor) = monitor {
        place_overlay(&overlay, &monitor, &settings, clamped);
    }
    clamped
}

#[tauri::command]
pub fn get_editor_prefs(state: State<AppState>) -> EditorPrefs {
    state.editor_prefs.get()
}

#[tauri::command]
pub fn set_editor_prefs(state: State<AppState>, prefs: EditorPrefs) -> Result<EditorPrefs, String> {
    state.editor_prefs.set(prefs).map_err(err_string)
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.get()
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: Settings,
) -> Result<Settings, String> {
    let old_language = state.settings.get().language;
    let saved = state.settings.set(settings).map_err(err_string)?;
    let language_changed = saved.language != old_language;
    if language_changed {
        crate::i18n::set_language(crate::i18n::resolve(&saved.language));
    }
    broadcast_settings(&app, &saved, language_changed);
    Ok(saved)
}

pub(crate) fn broadcast_settings(app: &AppHandle, saved: &Settings, refresh_native_ui: bool) {
    if refresh_native_ui {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Err(err) = crate::tray::refresh(&handle) {
                eprintln!("failed to rebuild tray menu: {err}");
            }
            for (label, key) in [
                ("main", "window.settings"),
                ("history", "window.history"),
                ("editor", "window.editor"),
                ("welcome", "window.welcome"),
            ] {
                if let Some(window) = handle.get_webview_window(label) {
                    let _ = window.set_title(&crate::i18n::t(key));
                }
            }
        });
    }
    let _ = app.emit("settings:changed", saved);
}

#[tauri::command]
pub fn set_shortcut(
    app: AppHandle,
    state: State<AppState>,
    action: crate::shortcuts::ShortcutAction,
    accelerator: String,
) -> Result<Settings, String> {
    let accelerator = accelerator.trim().to_string();
    let parsed = crate::shortcuts::validate(&accelerator)?;
    let mut settings = state.settings.get();
    for other in crate::shortcuts::ACTIONS {
        if other != action && crate::shortcuts::validate(settings.shortcut(other)) == Ok(parsed) {
            return Err(crate::i18n::t("settings.shortcut_error_duplicate"));
        }
    }
    let old = settings.shortcut(action).to_string();
    crate::shortcuts::rebind(&app, action, &old, &accelerator)?;
    *settings.shortcut_mut(action) = accelerator;
    let saved = state.settings.set(settings).map_err(err_string)?;
    broadcast_settings(&app, &saved, true);
    Ok(saved)
}

#[tauri::command]
pub fn resolved_language() -> String {
    crate::i18n::current().tag().to_string()
}

#[tauri::command]
pub fn save_capture_to_desktop(
    app: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<String, String> {
    use tauri::path::BaseDirectory;
    let entry = resolve(&state.history, &id)?;
    let dest = app
        .path()
        .resolve(&entry.id, BaseDirectory::Desktop)
        .map_err(err_string)?;
    copy_capture_file(&entry, &dest)?;
    Ok(dest.to_string_lossy().into_owned())
}

fn copy_capture_file(entry: &CaptureEntry, dest: &Path) -> Result<(), String> {
    std::fs::copy(&entry.path, dest)
        .map(|_| ())
        .map_err(err_string)
}

#[tauri::command]
pub fn open_editor(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    *state.editor_target.lock().unwrap() = Some(entry.id.clone());
    crate::windows::show_or_create(
        &app,
        "editor",
        "editor.html",
        "window.editor",
        |b| b.inner_size(1200.0, 800.0).min_inner_size(700.0, 500.0),
        |editor| {
            editor.emit("editor:load", &entry).map_err(err_string)?;
            let _ = editor.unminimize();
            Ok(())
        },
    )
}

#[tauri::command]
pub fn editor_target(state: State<AppState>) -> Result<CaptureEntry, String> {
    let id = state
        .editor_target
        .lock()
        .unwrap()
        .clone()
        .ok_or("no capture selected for the editor")?;
    resolve(&state.history, &id)
}

#[tauri::command]
pub fn read_capture_bytes(
    state: State<AppState>,
    id: String,
) -> Result<tauri::ipc::Response, String> {
    let entry = resolve(&state.history, &id)?;
    let bytes = std::fs::read(&entry.path).map_err(err_string)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportAction {
    Copy,
    SaveTo { dest: String },
    Overwrite { id: String },
}

#[tauri::command]
pub fn export_png(
    app: AppHandle,
    state: State<AppState>,
    data: String,
    action: ExportAction,
) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(err_string)?;
    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
    if !bytes.starts_with(PNG_MAGIC) {
        return Err(format!(
            "export produced invalid image data ({} bytes) — annotation not saved",
            bytes.len()
        ));
    }
    match action {
        ExportAction::Copy => copy_png_to_clipboard(&app, &bytes),
        ExportAction::SaveTo { dest } => std::fs::write(&dest, bytes).map_err(err_string),
        ExportAction::Overwrite { id } => {
            let entry = resolve(&state.history, &id)?;
            std::fs::write(&entry.path, bytes).map_err(err_string)?;
            let _ = app.emit("capture:new", &entry);
            Ok(())
        }
    }
}

#[tauri::command]
pub fn list_captures(state: State<AppState>) -> Vec<CaptureEntry> {
    state.history.list()
}

fn copy_png_to_clipboard(app: &AppHandle, bytes: &[u8]) -> Result<(), String> {
    let image = tauri::image::Image::from_bytes(bytes).map_err(err_string)?;
    app.clipboard().write_image(&image).map_err(err_string)
}

fn copy_capture_to_clipboard(app: &AppHandle, entry: &CaptureEntry) -> Result<(), String> {
    let bytes = std::fs::read(&entry.path).map_err(err_string)?;
    copy_png_to_clipboard(app, &bytes)
}

#[tauri::command]
pub fn copy_capture(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    copy_capture_to_clipboard(&app, &resolve(&state.history, &id)?)
}

#[tauri::command]
pub fn restore_capture(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    let _ = app.emit("capture:restore", &entry);
    show_overlay(&app);
    Ok(())
}

#[tauri::command]
pub fn save_capture_to(state: State<AppState>, id: String, dest: String) -> Result<(), String> {
    copy_capture_file(&resolve(&state.history, &id)?, Path::new(&dest))
}

#[tauri::command]
pub fn reveal_capture(state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    tauri_plugin_opener::reveal_item_in_dir(entry.path).map_err(err_string)
}

pub fn start_timed_capture(app: &AppHandle, seconds: u32) {
    if let Some(existing) = app.get_webview_window("timer") {
        let _ = existing.destroy();
    }
    *app.state::<AppState>().timer_seconds.lock().unwrap() = seconds;
    if let Err(err) = crate::windows::open_timer(app) {
        eprintln!("failed to open timer window: {err}");
    }
}

#[tauri::command]
pub fn timer_duration(state: State<AppState>) -> u32 {
    *state.timer_seconds.lock().unwrap()
}

#[tauri::command]
pub fn timed_capture_fire(app: AppHandle) {
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.destroy();
    }
    let mode = *app.state::<AppState>().last_capture_mode.lock().unwrap();
    spawn_capture(app, mode, Some(Duration::from_millis(150)));
}

#[derive(serde::Deserialize, Clone, Copy)]
pub struct SelectionRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[tauri::command]
pub fn run_scrolling_capture(
    app: AppHandle,
    rect: SelectionRect,
    direction: crate::capture::ScrollDirection,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return run_scrolling_capture_macos(app, rect, direction);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, rect, direction);
        Err("scrolling capture is only available on macOS".into())
    }
}

#[tauri::command]
pub fn stop_scrolling_capture(state: State<AppState>) {
    state.scroll_stop.store(true, Ordering::Relaxed);
}

#[cfg(target_os = "macos")]
fn run_scrolling_capture_macos(
    app: AppHandle,
    rect: SelectionRect,
    direction: crate::capture::ScrollDirection,
) -> Result<(), String> {
    use crate::capture::scrolling::{self, ScrollRegion};
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

    let window = app
        .get_webview_window("scrollcap")
        .ok_or("scrollcap window is not open")?;

    let scale = window.scale_factor().map_err(err_string)?;
    let origin = window
        .outer_position()
        .map_err(err_string)?
        .to_logical::<f64>(scale);
    let region = ScrollRegion {
        x: origin.x + rect.x,
        y: origin.y + rect.y,
        width: rect.width,
        height: rect.height,
    };

    let (running, stop) = {
        let state = app.state::<AppState>();
        (state.scroll_running.clone(), state.scroll_stop.clone())
    };
    if running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("a scrolling capture is already running".into());
    }

    if !crate::capture::scroll_input::ensure_accessibility() {
        running.store(false, Ordering::SeqCst);
        let _ = window.destroy();
        app.dialog()
            .message(crate::i18n::t("perm.accessibility_body"))
            .title(crate::i18n::t("perm.accessibility_title"))
            .kind(MessageDialogKind::Warning)
            .show(|_| {});
        return Ok(());
    }

    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }

    const PILL_W: f64 = 220.0;
    const PILL_H: f64 = 56.0;
    const GAP: f64 = 12.0;
    let (mon_left, mon_right, monitor_bottom) = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let (pos, size) = monitor_logical_bounds(&m);
            (pos.x, pos.x + size.width, pos.y + size.height)
        })
        .unwrap_or((f64::MIN, f64::MAX, f64::MAX));
    let below = region.y + region.height + GAP;
    let pill_y = if below + PILL_H <= monitor_bottom {
        below
    } else {
        (region.y - PILL_H - GAP).max(0.0)
    };
    let pill_x = region.x.min(mon_right - PILL_W).max(mon_left);
    let _ = window.set_size(tauri::LogicalSize::new(PILL_W, PILL_H));
    let _ = window.set_position(tauri::LogicalPosition::new(pill_x, pill_y));
    let _ = app.emit_to("scrollcap", "scroll:running", ());
    stop.store(false, Ordering::Relaxed);

    tauri::async_runtime::spawn_blocking(move || {
        let _running_guard = RunningGuard(running.clone());
        let result = (|| -> Result<(), CaptureError> {
            let work_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| CaptureError::Tool(e.to_string()))?
                .join("scroll-tmp");
            let image = scrolling::run(&region, direction, &stop, &work_dir, |frames| {
                let _ = app.emit_to("scrollcap", "scroll:progress", frames);
            })?;
            let state = app.state::<AppState>();
            let dest = state.history.new_capture_path();
            image
                .save(&dest)
                .map_err(|e| CaptureError::Tool(format!("could not save composite: {e}")))?;
            publish_capture(&app, &dest);
            Ok(())
        })();
        if let Some(window) = app.get_webview_window("scrollcap") {
            let _ = window.destroy();
        }
        if let Err(err) = result {
            eprintln!("scrolling capture failed: {err}");
        }
    });
    Ok(())
}

struct RunningGuard(Arc<AtomicBool>);
impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn resolve(history: &History, id: &str) -> Result<CaptureEntry, String> {
    history
        .resolve(id)
        .ok_or_else(|| format!("unknown capture: {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_origin_respects_monitor_offset() {
        let mon = ((2560.0, 100.0), (1920.0, 1080.0));
        let size = (300.0, 264.0);
        assert_eq!(
            overlay_origin(OverlayPosition::Left, mon.0, mon.1, size),
            (2576.0, 900.0)
        );
        assert_eq!(
            overlay_origin(OverlayPosition::Right, mon.0, mon.1, size),
            (2560.0 + 1920.0 - 300.0 - 16.0, 900.0)
        );
        assert_eq!(
            overlay_origin(OverlayPosition::Center, mon.0, mon.1, size),
            (2560.0 + (1920.0 - 300.0) / 2.0, 900.0)
        );
    }

    #[test]
    fn clamp_panels_fits_monitor_height() {
        assert_eq!(clamp_panels(1, 264.0, 1080.0), 1);
        assert_eq!(clamp_panels(3, 264.0, 1080.0), 3);
        assert_eq!(clamp_panels(9, 264.0, 1080.0), 3);
    }

    #[test]
    fn clamp_panels_never_returns_zero() {
        assert_eq!(clamp_panels(0, 264.0, 1080.0), 1);
        assert_eq!(clamp_panels(5, 264.0, 100.0), 1);
    }
}
