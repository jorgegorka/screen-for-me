use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::capture::{self, CaptureError, CaptureMode, CaptureOutcome};
use crate::history::{CaptureEntry, History};
use crate::settings::{EditorPrefs, EditorPrefsStore, OverlayPosition, Settings, SettingsStore};

pub struct AppState {
    pub history: History,
    pub settings: SettingsStore,
    pub editor_prefs: EditorPrefsStore,
    /// Capture the editor window should be showing. The editor pulls this on
    /// load, so opening never depends on an event landing before the page's
    /// listener is ready.
    pub editor_target: std::sync::Mutex<Option<String>>,
    /// Seconds for the pending self-timer; the timer window pulls this on load.
    pub timer_seconds: std::sync::Mutex<u32>,
    /// Mode of the most recent user-triggered capture; the self-timer fires
    /// this mode. Defaults to Fullscreen until a capture is taken.
    pub last_capture_mode: std::sync::Mutex<CaptureMode>,
    /// Set by `stop_scrolling_capture` to end the scroll loop early.
    pub scroll_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// True while a scrolling capture run is in flight; guards against a
    /// second run racing the pill window, stop flag, and tmp frame file.
    pub scroll_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Generation counter for the overlay's follow-the-cursor loop: each
    /// `show_overlay` bumps it and spawns a fresh loop, and any older loop
    /// exits on its next tick when it sees a newer epoch.
    pub overlay_follow_epoch: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// Entry point shared by tray items and global shortcuts.
pub fn trigger_capture(app: &AppHandle, mode: CaptureMode) {
    *app.state::<AppState>().last_capture_mode.lock().unwrap() = mode;
    spawn_capture(app.clone(), mode, None);
}

/// Run the (blocking, possibly interactive) capture off the main thread,
/// optionally after a delay.
fn spawn_capture(app: AppHandle, mode: CaptureMode, delay: Option<std::time::Duration>) {
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
    // The overlay is content-protected (never in the shot), but hide it anyway
    // so it doesn't sit under the interactive crosshair.
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

/// Prune history and announce a freshly written capture file.
fn publish_capture(app: &AppHandle, path: &std::path::Path) {
    let state = app.state::<AppState>();
    state.history.prune();
    let id = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    if let Some(entry) = state.history.resolve(&id) {
        let _ = app.emit("capture:new", &entry);
        show_overlay(app);
    }
}

// Single owner of the overlay's size: the conf window entry omits width/height
// and `show_overlay` always set_size's (scaled) before the first show.
const OVERLAY_BASE_WIDTH: f64 = 300.0;
const OVERLAY_BASE_HEIGHT: f64 = 264.0;

/// Cursor position in the global logical-point space that `monitor_from_point`
/// (CGDisplayBounds) uses. On macOS this must come from CoreGraphics —
/// tao's `cursor_position()` returns physical pixels scaled by the primary
/// monitor and mixes units in its Y-flip, so on any scaled/Retina display the
/// point lands outside every monitor's bounds and monitor lookup fails.
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

/// The monitor under the cursor, falling back to the primary one.
pub(crate) fn active_monitor(app: &AppHandle) -> Option<tauri::Monitor> {
    cursor_point(app)
        .and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
}

/// A monitor's position and size in the global logical-point space.
pub(crate) fn monitor_logical_bounds(
    monitor: &tauri::Monitor,
) -> (tauri::LogicalPosition<f64>, tauri::LogicalSize<f64>) {
    let scale = monitor.scale_factor();
    (
        monitor.position().to_logical::<f64>(scale),
        monitor.size().to_logical::<f64>(scale),
    )
}

/// Bottom-corner origin for the overlay inside a monitor's logical bounds.
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

/// Size the overlay per settings and place it at the configured corner of
/// `monitor`.
fn place_overlay(overlay: &tauri::WebviewWindow, monitor: &tauri::Monitor, settings: &Settings) {
    let width = OVERLAY_BASE_WIDTH * settings.overlay_size;
    let height = OVERLAY_BASE_HEIGHT * settings.overlay_size;
    let _ = overlay.set_size(tauri::LogicalSize::new(width, height));
    let (mon_pos, mon_size) = monitor_logical_bounds(monitor);
    let (x, y) = overlay_origin(
        settings.position,
        (mon_pos.x, mon_pos.y),
        (mon_size.width, mon_size.height),
        (width, height),
    );
    let _ = overlay.set_position(tauri::LogicalPosition::new(x, y));
}

/// Show the quick-access overlay at the configured corner of the active
/// monitor (the one under the cursor) or the primary one.
fn show_overlay(app: &AppHandle) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    let settings = app.state::<AppState>().settings.get();

    let active_monitor = if settings.move_to_active_screen {
        cursor_point(app).and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten())
    } else {
        None
    };
    let monitor = active_monitor.or_else(|| overlay.primary_monitor().ok().flatten());

    match monitor {
        Some(monitor) => place_overlay(&overlay, &monitor, &settings),
        None => {
            let width = OVERLAY_BASE_WIDTH * settings.overlay_size;
            let height = OVERLAY_BASE_HEIGHT * settings.overlay_size;
            let _ = overlay.set_size(tauri::LogicalSize::new(width, height));
        }
    }
    let _ = overlay.show();
    follow_active_monitor(app);
}

/// Whether the left mouse button is currently held anywhere on screen. Used
/// to pause overlay following so a drag-out (or any drag) never yanks the
/// panel across screens mid-gesture.
#[cfg(target_os = "macos")]
fn left_mouse_button_down() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        // CGEventSourceButtonState(kCGEventSourceStateCombinedSessionState = 0,
        //                          kCGMouseButtonLeft = 0)
        fn CGEventSourceButtonState(state_id: i32, button: u32) -> bool;
    }
    unsafe { CGEventSourceButtonState(0, 0) }
}

#[cfg(not(target_os = "macos"))]
fn left_mouse_button_down() -> bool {
    false
}

/// While the overlay stays visible, keep it on the monitor under the cursor:
/// poll every 400 ms and re-place the panel once the cursor has settled on a
/// different monitor for two consecutive ticks (so merely passing through a
/// screen doesn't bounce it). Respects the `move_to_active_screen` setting
/// live and pauses while the mouse button is down.
fn follow_active_monitor(app: &AppHandle) {
    use std::sync::atomic::Ordering;
    let epochs = app.state::<AppState>().overlay_follow_epoch.clone();
    let epoch = epochs.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        let mut pending: Option<(i32, i32)> = None;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            if epochs.load(Ordering::SeqCst) != epoch {
                return;
            }
            let Some(overlay) = app.get_webview_window("overlay") else {
                return;
            };
            if !overlay.is_visible().unwrap_or(false) {
                return;
            }
            let settings = app.state::<AppState>().settings.get();
            if !settings.move_to_active_screen || left_mouse_button_down() {
                pending = None;
                continue;
            }
            let Some(target) = cursor_point(&app)
                .and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten())
            else {
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
                place_overlay(&overlay, &target, &settings);
                pending = None;
            } else {
                pending = Some(key);
            }
        }
    });
}

#[tauri::command]
pub fn get_editor_prefs(state: State<AppState>) -> EditorPrefs {
    state.editor_prefs.get()
}

#[tauri::command]
pub fn set_editor_prefs(state: State<AppState>, prefs: EditorPrefs) -> Result<EditorPrefs, String> {
    state.editor_prefs.set(prefs).map_err(|e| e.to_string())
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
    let saved = state.settings.set(settings).map_err(|e| e.to_string())?;
    if saved.language != old_language {
        crate::i18n::set_language(crate::i18n::resolve(&saved.language));
        // Menu and title APIs must run on the main thread on macOS.
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
    let _ = app.emit("settings:changed", &saved);
    Ok(saved)
}

/// Rebind one capture action's global shortcut: validate, swap the OS
/// registration (rolled back on failure), persist, and refresh the tray so its
/// accelerator labels match. Returns the saved settings like `set_settings`.
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
    let saved = state.settings.set(settings).map_err(|e| e.to_string())?;
    // Menu APIs must run on the main thread on macOS.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Err(err) = crate::tray::refresh(&handle) {
            eprintln!("failed to rebuild tray menu: {err}");
        }
    });
    let _ = app.emit("settings:changed", &saved);
    Ok(saved)
}

/// The language tag the webviews should render in ("en-GB", "es", …), with the
/// "system" setting already resolved against the OS locale.
#[tauri::command]
pub fn resolved_language() -> String {
    crate::i18n::current().tag().to_string()
}

/// Auto-close "Save and Close": copy the capture to the user's Desktop.
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
        .map_err(|e| e.to_string())?;
    std::fs::copy(&entry.path, &dest).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn open_editor(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    *state.editor_target.lock().unwrap() = Some(entry.id.clone());
    // On first open (or after a hard close) the page pulls the target via
    // `editor_target` once it has loaded. A warm window's listener is live,
    // so tell it to reload before revealing it (unminimize first in case it
    // was minimized).
    crate::windows::show_or_create(
        &app,
        "editor",
        "editor.html",
        &crate::i18n::t("window.editor"),
        (1200.0, 800.0),
        (700.0, 500.0),
        |editor| {
            editor
                .emit("editor:load", &entry)
                .map_err(|e| e.to_string())?;
            let _ = editor.unminimize();
            Ok(())
        },
    )
}

/// The capture the editor should currently display (set by `open_editor`).
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

/// Return a capture's raw PNG bytes. The editor loads these as a same-origin
/// `blob:` URL — loading via the `asset://` protocol instead taints the Konva
/// canvas, which makes `toDataURL()` fail and export impossible.
#[tauri::command]
pub fn read_capture_bytes(
    state: State<AppState>,
    id: String,
) -> Result<tauri::ipc::Response, String> {
    let entry = resolve(&state.history, &id)?;
    let bytes = std::fs::read(&entry.path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportAction {
    /// Put the PNG on the clipboard.
    Copy,
    /// Write the PNG to a user-chosen path.
    SaveTo { dest: String },
    /// Replace an existing capture with the annotated version.
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
        .map_err(|e| e.to_string())?;
    // Never let an empty/invalid export through — a blank toDataURL() would
    // otherwise silently overwrite the capture with 0 bytes and destroy it.
    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
    if !bytes.starts_with(PNG_MAGIC) {
        return Err(format!(
            "export produced invalid image data ({} bytes) — annotation not saved",
            bytes.len()
        ));
    }
    match action {
        ExportAction::Copy => copy_png_to_clipboard(&app, &bytes),
        ExportAction::SaveTo { dest } => std::fs::write(&dest, bytes).map_err(|e| e.to_string()),
        ExportAction::Overwrite { id } => {
            let entry = resolve(&state.history, &id)?;
            std::fs::write(&entry.path, bytes).map_err(|e| e.to_string())?;
            // Refresh the overlay thumbnail with the annotated version.
            let _ = app.emit("capture:new", &entry);
            Ok(())
        }
    }
}

#[tauri::command]
pub fn list_captures(state: State<AppState>) -> Vec<CaptureEntry> {
    state.history.list()
}

/// Put PNG bytes on the system clipboard as an image.
fn copy_png_to_clipboard(app: &AppHandle, bytes: &[u8]) -> Result<(), String> {
    let image = tauri::image::Image::from_bytes(bytes).map_err(|e| e.to_string())?;
    app.clipboard()
        .write_image(&image)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_capture(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    let bytes = std::fs::read(&entry.path).map_err(|e| e.to_string())?;
    copy_png_to_clipboard(&app, &bytes)
}

#[tauri::command]
pub fn save_capture_to(state: State<AppState>, id: String, dest: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    std::fs::copy(&entry.path, &dest)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reveal_capture(state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    tauri_plugin_opener::reveal_item_in_dir(entry.path).map_err(|e| e.to_string())
}

/// Tray entry point: stage the duration and show the countdown window.
/// Starting a new timer replaces a running one.
pub fn start_timed_capture(app: &AppHandle, seconds: u32) {
    if let Some(existing) = app.get_webview_window("timer") {
        let _ = existing.destroy();
    }
    *app.state::<AppState>().timer_seconds.lock().unwrap() = seconds;
    if let Err(err) = crate::windows::open_timer(app) {
        eprintln!("failed to open timer window: {err}");
    }
}

/// Duration for the countdown window (pull model, like `editor_target`).
#[tauri::command]
pub fn timer_duration(state: State<AppState>) -> u32 {
    *state.timer_seconds.lock().unwrap()
}

/// Countdown reached zero: tear the window down, wait a beat so it cannot
/// appear in the shot, then run the normal fullscreen path.
#[tauri::command]
pub fn timed_capture_fire(app: AppHandle) {
    if let Some(window) = app.get_webview_window("timer") {
        let _ = window.destroy();
    }
    let mode = *app.state::<AppState>().last_capture_mode.lock().unwrap();
    spawn_capture(app, mode, Some(std::time::Duration::from_millis(150)));
}

/// Selection rect in scrollcap-window-local logical pixels (CSS px).
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
    state
        .scroll_stop
        .store(true, std::sync::atomic::Ordering::Relaxed);
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

    // Window-local logical rect → global points. The window covers the whole
    // monitor, so window origin + local point = global point, in the same
    // logical space screencapture -R and CGDisplayBounds use.
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let origin = window
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let region = ScrollRegion {
        x: origin.x + rect.x,
        y: origin.y + rect.y,
        width: rect.width,
        height: rect.height,
    };

    // Single-flight guard: a second run would destroy the live pill window,
    // race the shared stop flag, and interleave writes to the tmp frame file.
    let running = app.state::<AppState>().scroll_running.clone();
    if running
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_err()
    {
        return Err("a scrolling capture is already running".into());
    }

    if !crate::capture::scroll_input::ensure_accessibility() {
        running.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = window.destroy();
        app.dialog()
            .message(crate::i18n::t("perm.accessibility_body"))
            .title(crate::i18n::t("perm.accessibility_title"))
            .kind(MessageDialogKind::Warning)
            .show(|_| {});
        return Ok(());
    }

    // The overlay is content-protected (never in the frames), but hide it so
    // it stays out of the user's way while the page scrolls.
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }

    // Shrink the selection window to a Stop pill parked outside the rect so it
    // doesn't visually cover the content being scrolled. The window is
    // content-protected (windows.rs), so even when a tall selection forces an
    // overlap the pill never appears in the grabbed frames.
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
    // Keep the pill on-screen even when the selection hugs the right edge.
    let pill_x = region.x.min(mon_right - PILL_W).max(mon_left);
    let _ = window.set_size(tauri::LogicalSize::new(PILL_W, PILL_H));
    let _ = window.set_position(tauri::LogicalPosition::new(pill_x, pill_y));
    let _ = app.emit_to("scrollcap", "scroll:running", ());

    let state = app.state::<AppState>();
    state
        .scroll_stop
        .store(false, std::sync::atomic::Ordering::Relaxed);
    let stop = state.scroll_stop.clone();

    tauri::async_runtime::spawn_blocking(move || {
        // Owns a clone of the flag so a panic anywhere below (e.g. an
        // `expect` inside the image crate) still releases it on unwind,
        // instead of permanently bricking the feature.
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
        // `_running_guard` drops here (or on panic-unwind) and releases the flag.
    });
    Ok(())
}

/// Resets the scroll_running flag even if the capture worker panics.
struct RunningGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
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
        // A secondary monitor to the right of a 1440p primary: the overlay
        // must land inside *its* bounds, not the primary's.
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
}
