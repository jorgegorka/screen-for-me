use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::capture::{self, CaptureError, CaptureMode, CaptureOutcome};
use crate::history::{CaptureEntry, History};

pub struct AppState {
    pub history: History,
}

/// Entry point shared by tray items, global shortcuts, and the IPC command.
/// Runs the (blocking, possibly interactive) capture off the main thread.
pub fn trigger_capture(app: &AppHandle, mode: CaptureMode) {
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
            Ok(())
        }
    }
}

/// Show the quick-access overlay at the bottom-left of the primary monitor.
fn show_overlay(app: &AppHandle) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    const MARGIN: f64 = 16.0;
    if let (Ok(Some(monitor)), Ok(size)) = (overlay.primary_monitor(), overlay.outer_size()) {
        let scale = monitor.scale_factor();
        let mon_pos = monitor.position().to_logical::<f64>(scale);
        let mon_size = monitor.size().to_logical::<f64>(scale);
        let win = size.to_logical::<f64>(scale);
        let _ = overlay.set_position(tauri::LogicalPosition::new(
            mon_pos.x + MARGIN,
            mon_pos.y + mon_size.height - win.height - MARGIN,
        ));
    }
    let _ = overlay.show();
}

#[tauri::command]
pub fn open_editor(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    if let Some(editor) = app.get_webview_window("editor") {
        editor.emit("editor:load", &entry).map_err(|e| e.to_string())?;
        editor.show().map_err(|e| e.to_string())?;
        editor.set_focus().map_err(|e| e.to_string())?;
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "editor",
            tauri::WebviewUrl::App(format!("editor.html?id={}", entry.id).into()),
        )
        .title("Screen for me — Annotate")
        .inner_size(1200.0, 800.0)
        .min_inner_size(700.0, 500.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_capture(state: State<AppState>, id: String) -> Result<CaptureEntry, String> {
    resolve(&state.history, &id)
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
    match action {
        ExportAction::Copy => {
            let image = tauri::image::Image::from_bytes(&bytes).map_err(|e| e.to_string())?;
            app.clipboard().write_image(&image).map_err(|e| e.to_string())
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
    app.clipboard().write_image(&image).map_err(|e| e.to_string())
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

fn resolve(history: &History, id: &str) -> Result<CaptureEntry, String> {
    history
        .resolve(id)
        .ok_or_else(|| format!("unknown capture: {id}"))
}
