use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

/// Windows that hide (rather than destroy) on close, handled in lib.rs's
/// `on_window_event`; `timer` and `scrollcap` are transient and excluded.
pub const HIDE_ON_CLOSE: &[&str] = &["main", "editor", "history"];

/// Show a window that hides (rather than destroys) on close, creating it once.
/// `on_reuse` runs against an already-existing (warm) window before it is
/// shown; a fresh window skips it.
pub fn show_or_create(
    app: &AppHandle,
    label: &str,
    url: &str,
    title: &str,
    size: (f64, f64),
    min_size: (f64, f64),
    on_reuse: impl FnOnce(&tauri::WebviewWindow) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        on_reuse(&window)?;
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
        &crate::i18n::t("window.history"),
        (860.0, 620.0),
        (520.0, 400.0),
        |_| Ok(()),
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
    .title(crate::i18n::t("window.timer"))
    .inner_size(SIZE, SIZE)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .accept_first_mouse(true)
    // Never in a shot: the timer must not appear if a capture fires under it.
    .content_protected(true);
    if let Some(monitor) = crate::commands::active_monitor(app) {
        let (pos, size) = crate::commands::monitor_logical_bounds(&monitor);
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
    .title(crate::i18n::t("window.scrollcap"))
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
        let (pos, size) = crate::commands::monitor_logical_bounds(&monitor);
        builder = builder
            .position(pos.x, pos.y)
            .inner_size(size.width, size.height);
    }
    builder.build()?;
    Ok(())
}

/// Check for updates against the GitHub Releases manifest. In `silent` mode
/// (launch/daily auto-check) "up to date" and network errors produce no UI —
/// only an actual update shows the install prompt. The manual tray item
/// (`silent = false`) reports every outcome in a dialog.
pub fn check_for_updates(app: &AppHandle, silent: bool) {
    use tauri_plugin_updater::UpdaterExt;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let dialog = app.dialog().clone();
        let result = match app.updater() {
            Ok(updater) => updater.check().await,
            Err(err) => {
                if silent {
                    eprintln!("update auto-check failed: {err}");
                    return;
                }
                dialog
                    .message(crate::i18n::t_with(
                        "updates.check_failed",
                        &[("err", &err.to_string())],
                    ))
                    .title(crate::i18n::t("updates.title"))
                    .kind(MessageDialogKind::Warning)
                    .show(|_| {});
                return;
            }
        };
        match result {
            Ok(Some(update)) => prompt_and_install(app, update),
            Ok(None) => {
                if !silent {
                    dialog
                        .message(crate::i18n::t("updates.latest"))
                        .title(crate::i18n::t("updates.title"))
                        .kind(MessageDialogKind::Info)
                        .show(|_| {});
                }
            }
            Err(err) => {
                if silent {
                    eprintln!("update auto-check failed: {err}");
                    return;
                }
                dialog
                    .message(crate::i18n::t_with(
                        "updates.unreachable",
                        &[("err", &err.to_string())],
                    ))
                    .title(crate::i18n::t("updates.title"))
                    .kind(MessageDialogKind::Warning)
                    .show(|_| {});
            }
        }
    });
}

/// Offer to install a found update; on confirmation download it, verify the
/// minisign signature (done by the plugin), swap the .app and relaunch.
fn prompt_and_install(app: AppHandle, update: tauri_plugin_updater::Update) {
    use tauri_plugin_dialog::MessageDialogButtons;
    app.dialog()
        .message(crate::i18n::t_with(
            "updates.available",
            &[("version", &update.version)],
        ))
        .title(crate::i18n::t("updates.available_title"))
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            crate::i18n::t("updates.install"),
            crate::i18n::t("updates.later"),
        ))
        .show(move |install| {
            if !install {
                return;
            }
            tauri::async_runtime::spawn(async move {
                match update.download_and_install(|_, _| {}, || {}).await {
                    Ok(()) => app.restart(),
                    Err(err) => {
                        app.dialog()
                            .message(crate::i18n::t_with(
                                "updates.install_failed",
                                &[("err", &err.to_string())],
                            ))
                            .title(crate::i18n::t("updates.title"))
                            .kind(MessageDialogKind::Warning)
                            .show(|_| {});
                    }
                }
            });
        });
}
