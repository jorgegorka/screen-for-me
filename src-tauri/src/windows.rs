use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

/// Windows that hide (rather than destroy) on close, handled in lib.rs's
/// `on_window_event`; `timer` and `scrollcap` are transient and excluded.
pub const HIDE_ON_CLOSE: &[&str] = &["main", "editor", "history"];

/// Show a window and bring it in front of other apps' windows. An Accessory
/// app is never the active application, so `set_focus` alone can leave a
/// window opening *behind* whatever the user is working in — activate the app
/// first to force it forward.
pub fn bring_to_front(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if let Some(mtm) = objc2_foundation::MainThreadMarker::new() {
        #[allow(deprecated)]
        objc2_app_kit::NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    }
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

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
        bring_to_front(&window)?;
    } else {
        let window =
            tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App(url.into()))
                .title(title)
                .inner_size(size.0, size.1)
                .min_inner_size(min_size.0, min_size.1)
                .build()
                .map_err(|e| e.to_string())?;
        bring_to_front(&window)?;
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

/// Welcome window (shortcut onboarding): shown once on first launch and
/// reopenable from the tray. Deliberately NOT in HIDE_ON_CLOSE — closing
/// destroys it, it's rarely needed twice.
#[tauri::command]
pub fn open_welcome(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("welcome") {
        return bring_to_front(&window);
    }
    let window = tauri::WebviewWindowBuilder::new(
        &app,
        "welcome",
        tauri::WebviewUrl::App("welcome.html".into()),
    )
    .title(crate::i18n::t("window.welcome"))
    .inner_size(520.0, 620.0)
    .resizable(false)
    .maximizable(false)
    .build()
    .map_err(|e| e.to_string())?;
    bring_to_front(&window)
}

pub fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = bring_to_front(&window);
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
                    Ok(()) => relaunch_and_exit(&app),
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

/// Relaunch the app after an update, then exit the current process.
///
/// `AppHandle::restart` spawns the new binary as a direct child, which lands
/// in this process's process group. When the app was started by its autostart
/// LaunchAgent, launchd tears that whole process group down as soon as the
/// old process exits — the SIGTERM arrives while the kernel is still parked
/// in Gatekeeper's first-exec scan of the just-swapped bundle, macOS denies
/// the exec ("ASP: Security policy would not allow process") and the app
/// never comes back. Instead, spawn a shell in its *own* process group that
/// waits for this process to die and relaunches the bundle through
/// LaunchServices (`open`), which is immune to the group teardown and lets
/// Gatekeeper assess the new bundle normally.
#[cfg(target_os = "macos")]
fn relaunch_and_exit(app: &AppHandle) {
    use std::os::unix::process::CommandExt;

    let bundle = std::env::current_exe().ok().and_then(|e| bundle_root(&e));
    let Some(bundle) = bundle else {
        // Bare binary (dev build): no bundle to `open`, no LaunchAgent either.
        app.restart();
    };

    // Bundle path rides in as $0 so it needs no shell quoting. Bounded wait
    // (~30 s): if the old process somehow never dies, skip the relaunch
    // rather than start a second instance next to it.
    let script = format!(
        "n=0; while /bin/kill -0 {pid} 2>/dev/null && [ $n -lt 150 ]; do /bin/sleep 0.2; n=$((n+1)); done; \
         /bin/kill -0 {pid} 2>/dev/null || exec /usr/bin/open \"$0\"",
        pid = std::process::id()
    );
    let spawned = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg(bundle)
        .process_group(0)
        .spawn();
    match spawned {
        Ok(_) => app.exit(0),
        Err(_) => app.restart(),
    }
}

#[cfg(not(target_os = "macos"))]
fn relaunch_and_exit(app: &AppHandle) {
    app.restart();
}

/// `…/Foo.app/Contents/MacOS/binary` → `…/Foo.app`, or `None` when the
/// executable doesn't live in an app bundle (dev builds).
#[cfg(any(target_os = "macos", test))]
fn bundle_root(exe: &std::path::Path) -> Option<std::path::PathBuf> {
    let macos_dir = exe.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }
    let contents = macos_dir.parent()?;
    if contents.file_name()? != "Contents" {
        return None;
    }
    let root = contents.parent()?;
    if root.extension()? != "app" {
        return None;
    }
    Some(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::bundle_root;
    use std::path::{Path, PathBuf};

    #[test]
    fn bundle_root_resolves_app_bundle() {
        assert_eq!(
            bundle_root(Path::new(
                "/Applications/Screen for me.app/Contents/MacOS/screenforme"
            )),
            Some(PathBuf::from("/Applications/Screen for me.app"))
        );
    }

    #[test]
    fn bundle_root_rejects_bare_binary() {
        assert_eq!(
            bundle_root(Path::new(
                "/Users/x/repo/src-tauri/target/debug/screenforme"
            )),
            None
        );
    }

    #[test]
    fn bundle_root_rejects_wrong_layout() {
        assert_eq!(bundle_root(Path::new("/opt/MacOS/screenforme")), None);
        assert_eq!(
            bundle_root(Path::new("/Applications/Foo/Contents/MacOS/bin")),
            None
        );
    }
}
