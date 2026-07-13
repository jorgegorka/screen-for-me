use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

/// Show a window that hides (rather than destroys) on close, creating it once.
fn show_or_create(
    app: &AppHandle,
    label: &str,
    url: &str,
    title: &str,
    size: (f64, f64),
    min_size: (f64, f64),
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
        tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App(url.into()))
            .title(title)
            .inner_size(size.0, size.1)
            .min_inner_size(min_size.0, min_size.1)
            .build()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_history(app: AppHandle) -> Result<(), String> {
    show_or_create(
        &app,
        "history",
        "history.html",
        "Screen for me — Capture History",
        (860.0, 620.0),
        (520.0, 400.0),
    )
}

pub fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Transient countdown window, centered on the active monitor. Destroyed (not
/// hidden) when the timer fires or is cancelled.
pub fn open_timer(app: &AppHandle) -> tauri::Result<()> {
    const SIZE: f64 = 180.0;
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "timer",
        tauri::WebviewUrl::App("timer.html".into()),
    )
    .title("Self-Timer")
    .inner_size(SIZE, SIZE)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .accept_first_mouse(true);
    if let Some(monitor) = crate::commands::active_monitor(app) {
        let scale = monitor.scale_factor();
        let pos = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        builder = builder.position(
            pos.x + (size.width - SIZE) / 2.0,
            pos.y + (size.height - SIZE) / 2.0,
        );
    }
    builder.build()?;
    Ok(())
}

/// Transient full-screen selection window for scrolling capture, covering the
/// active monitor. Destroyed when the run finishes or is cancelled.
pub fn open_scrollcap(app: &AppHandle) -> tauri::Result<()> {
    // A run in flight owns the scrollcap window (as the Stop pill); ignore the
    // tray click rather than destroying it out from under the run.
    let state = app.state::<crate::commands::AppState>();
    if state
        .scroll_running
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return Ok(());
    }
    if let Some(existing) = app.get_webview_window("scrollcap") {
        let _ = existing.destroy();
    }
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "scrollcap",
        tauri::WebviewUrl::App("scrollcap.html".into()),
    )
    .title("Scrolling Capture")
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .accept_first_mouse(true)
    // NSWindowSharingNone: screencapture never sees this window, so the Stop
    // pill can overlap the region (tall selections leave it no room outside)
    // without being baked into frame 1 or pinning the stitch offset at 0 as a
    // fake fixed header.
    .content_protected(true)
    .focused(true);
    if let Some(monitor) = crate::commands::active_monitor(app) {
        let scale = monitor.scale_factor();
        let pos = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        builder = builder
            .position(pos.x, pos.y)
            .inner_size(size.width, size.height);
    }
    builder.build()?;
    Ok(())
}

pub fn show_about(app: &AppHandle) {
    let package = app.package_info();
    let message = format!(
        "Screen for me\nVersion {}\n\nA screenshot app: capture, annotate, and share.",
        package.version
    );
    app.dialog()
        .message(message)
        .title("About Screen for me")
        .kind(MessageDialogKind::Info)
        .show(|_| {});
}

/// Check for updates against the configured endpoint and report the result in
/// a native dialog. Endpoint/signing are placeholders until a release pipeline
/// exists, so a failed check reports gracefully rather than erroring silently.
pub fn check_for_updates(app: &AppHandle) {
    use tauri_plugin_updater::UpdaterExt;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let dialog = app.dialog().clone();
        let result = match app.updater() {
            Ok(updater) => updater.check().await,
            Err(err) => {
                dialog
                    .message(format!("Could not check for updates:\n{err}"))
                    .title("Check for Updates")
                    .kind(MessageDialogKind::Warning)
                    .show(|_| {});
                return;
            }
        };
        match result {
            Ok(Some(update)) => {
                dialog
                    .message(format!(
                        "Version {} is available. Download and install it from the Screen for me website.",
                        update.version
                    ))
                    .title("Update Available")
                    .kind(MessageDialogKind::Info)
                    .show(|_| {});
            }
            Ok(None) => {
                dialog
                    .message("You're on the latest version.")
                    .title("Check for Updates")
                    .kind(MessageDialogKind::Info)
                    .show(|_| {});
            }
            Err(err) => {
                dialog
                    .message(format!(
                        "Couldn't reach the update server. Please try again later.\n\n({err})"
                    ))
                    .title("Check for Updates")
                    .kind(MessageDialogKind::Warning)
                    .show(|_| {});
            }
        }
    });
}
