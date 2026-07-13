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
}

/// Entry point shared by tray items, global shortcuts, and the IPC command.
/// Runs the (blocking, possibly interactive) capture off the main thread.
pub fn trigger_capture(app: &AppHandle, mode: CaptureMode) {
    *app.state::<AppState>().last_capture_mode.lock().unwrap() = mode;
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(err) = capture_and_publish(&app, mode) {
            eprintln!("capture failed: {err}");
            let _ = app.emit("capture:error", err.to_string());
        }
    });
}

fn capture_and_publish(app: &AppHandle, mode: CaptureMode) -> Result<(), CaptureError> {
    // The overlay must not appear in the shot.
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

/// Show the quick-access overlay at the configured corner of the active
/// monitor (the one under the cursor) or the primary one.
fn show_overlay(app: &AppHandle) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    let settings = app.state::<AppState>().settings.get();
    let width = OVERLAY_BASE_WIDTH * settings.overlay_size;
    let height = OVERLAY_BASE_HEIGHT * settings.overlay_size;
    let _ = overlay.set_size(tauri::LogicalSize::new(width, height));

    let active_monitor = if settings.move_to_active_screen {
        cursor_point(app).and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten())
    } else {
        None
    };
    let monitor = active_monitor.or_else(|| overlay.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        const MARGIN: f64 = 16.0;
        let scale = monitor.scale_factor();
        let mon_pos = monitor.position().to_logical::<f64>(scale);
        let mon_size = monitor.size().to_logical::<f64>(scale);
        let x = match settings.position {
            OverlayPosition::Left => mon_pos.x + MARGIN,
            OverlayPosition::Center => mon_pos.x + (mon_size.width - width) / 2.0,
            OverlayPosition::Right => mon_pos.x + mon_size.width - width - MARGIN,
        };
        let y = mon_pos.y + mon_size.height - height - MARGIN;
        let _ = overlay.set_position(tauri::LogicalPosition::new(x, y));
    }
    let _ = overlay.show();
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
    let saved = state.settings.set(settings).map_err(|e| e.to_string())?;
    let _ = app.emit("settings:changed", &saved);
    Ok(saved)
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
    if let Some(editor) = app.get_webview_window("editor") {
        // Window kept warm across sessions: its listener is live, so tell it
        // to reload, then reveal it (unminimize first in case it was minimized).
        editor
            .emit("editor:load", &entry)
            .map_err(|e| e.to_string())?;
        let _ = editor.unminimize();
        editor.show().map_err(|e| e.to_string())?;
        editor.set_focus().map_err(|e| e.to_string())?;
    } else {
        // First open (or after a hard close): the page pulls the target via
        // `editor_target` once it has loaded.
        tauri::WebviewWindowBuilder::new(
            &app,
            "editor",
            tauri::WebviewUrl::App("editor.html".into()),
        )
        .title("Screen for me — Annotate")
        .inner_size(1200.0, 800.0)
        .min_inner_size(700.0, 500.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
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

#[tauri::command]
pub fn get_capture(state: State<AppState>, id: String) -> Result<CaptureEntry, String> {
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
        ExportAction::Copy => {
            let image = tauri::image::Image::from_bytes(&bytes).map_err(|e| e.to_string())?;
            app.clipboard()
                .write_image(&image)
                .map_err(|e| e.to_string())
        }
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
pub fn capture_screen(app: AppHandle, mode: CaptureMode) {
    trigger_capture(&app, mode);
}

#[tauri::command]
pub fn list_captures(state: State<AppState>) -> Vec<CaptureEntry> {
    state.history.list()
}

#[tauri::command]
pub fn delete_capture(state: State<AppState>, id: String) -> bool {
    state.history.delete(&id)
}

#[tauri::command]
pub fn copy_capture(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    let bytes = std::fs::read(&entry.path).map_err(|e| e.to_string())?;
    let image = tauri::image::Image::from_bytes(&bytes).map_err(|e| e.to_string())?;
    app.clipboard()
        .write_image(&image)
        .map_err(|e| e.to_string())
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
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        if let Err(err) = capture_and_publish(&app, mode) {
            eprintln!("timed capture failed: {err}");
            let _ = app.emit("capture:error", err.to_string());
        }
    });
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
    direction: String,
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
    direction: String,
) -> Result<(), String> {
    use crate::capture::scrolling::{self, ScrollRegion};
    use crate::capture::stitch::ScrollDirection;
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

    let direction = match direction.as_str() {
        "up" => ScrollDirection::Up,
        "down" => ScrollDirection::Down,
        "left" => ScrollDirection::Left,
        "right" => ScrollDirection::Right,
        other => return Err(format!("unknown scroll direction: {other}")),
    };
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
            .message(
                "Scrolling Capture needs Accessibility access to scroll the page for you.\n\n\
                 Enable \"Screen for me\" in System Settings → Privacy & Security → \
                 Accessibility, then try again.",
            )
            .title("Accessibility Permission Needed")
            .kind(MessageDialogKind::Warning)
            .show(|_| {});
        return Ok(());
    }

    // The overlay must not appear in frames either.
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }

    // Shrink the selection window to a Stop pill parked outside the rect so it
    // never appears in a frame. (If the rect spans the whole monitor there is
    // no outside; the pill may then overlap — accepted v1 limitation.)
    const PILL_W: f64 = 220.0;
    const PILL_H: f64 = 56.0;
    const GAP: f64 = 12.0;
    let (mon_left, mon_right, monitor_bottom) = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let s = m.scale_factor();
            let pos = m.position().to_logical::<f64>(s);
            let size = m.size().to_logical::<f64>(s);
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
            let _ = app.emit("capture:error", err.to_string());
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
