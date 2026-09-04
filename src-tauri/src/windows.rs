use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

pub const HIDE_ON_CLOSE: &[&str] = &["main", "editor", "history"];

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

type Builder<'a> = tauri::WebviewWindowBuilder<'a, tauri::Wry, AppHandle>;

fn window_builder<'a>(app: &'a AppHandle, label: &str, url: &str, title_key: &str) -> Builder<'a> {
    tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App(url.into()))
        .title(crate::i18n::t(title_key))
}

pub fn show_or_create(
    app: &AppHandle,
    label: &str,
    url: &str,
    title_key: &str,
    configure: impl FnOnce(Builder) -> Builder,
    on_reuse: impl FnOnce(&tauri::WebviewWindow) -> Result<(), String>,
) -> Result<(), String> {
    let window = match app.get_webview_window(label) {
        Some(window) => {
            on_reuse(&window)?;
            window
        }
        None => configure(window_builder(app, label, url, title_key))
            .build()
            .map_err(|e| e.to_string())?,
    };
    bring_to_front(&window)
}

#[tauri::command]
pub fn open_history(app: AppHandle) -> Result<(), String> {
    show_or_create(
        &app,
        "history",
        "history.html",
        "window.history",
        |b| b.inner_size(860.0, 620.0).min_inner_size(520.0, 400.0),
        |_| Ok(()),
    )
}

#[tauri::command]
pub fn open_welcome(app: AppHandle) -> Result<(), String> {
    show_or_create(
        &app,
        "welcome",
        "welcome.html",
        "window.welcome",
        |b| b.inner_size(520.0, 620.0).resizable(false).maximizable(false),
        |_| Ok(()),
    )
}

pub fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = bring_to_front(&window);
    }
}

fn transient_window<'a>(app: &'a AppHandle, label: &str, url: &str, title_key: &str) -> Builder<'a> {
    window_builder(app, label, url, title_key)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .accept_first_mouse(true)
        .content_protected(true)
}

pub fn open_timer(app: &AppHandle) -> tauri::Result<()> {
    const SIZE: f64 = 180.0;
    let mut builder = transient_window(app, "timer", "timer.html", "window.timer").inner_size(SIZE, SIZE);
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

pub fn open_scrollcap(app: &AppHandle) -> tauri::Result<()> {
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
    let mut builder =
        transient_window(app, "scrollcap", "scrollcap.html", "window.scrollcap").focused(true);
    if let Some(monitor) = crate::commands::active_monitor(app) {
        let (pos, size) = crate::commands::monitor_logical_bounds(&monitor);
        builder = builder
            .position(pos.x, pos.y)
            .inner_size(size.width, size.height);
    }
    builder.build()?;
    Ok(())
}

pub fn open_recorder(app: &AppHandle) -> tauri::Result<()> {
    const WIDTH: f64 = 320.0;
    const HEIGHT: f64 = 164.0;
    if let Some(existing) = app.get_webview_window("recorder") {
        let _ = existing.destroy();
    }
    let mut builder = transient_window(app, "recorder", "recorder.html", "window.recorder")
        .focused(true)
        .inner_size(WIDTH, HEIGHT);
    if let Some(monitor) = crate::commands::active_monitor(app) {
        let (pos, size) = crate::commands::monitor_logical_bounds(&monitor);
        builder = builder.position(
            pos.x + (size.width - WIDTH) / 2.0,
            pos.y + (size.height - HEIGHT) / 2.0,
        );
    }
    builder.build()?;
    Ok(())
}

pub fn check_for_updates(app: &AppHandle, silent: bool) {
    use tauri_plugin_updater::UpdaterExt;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = match app.updater() {
            Ok(updater) => updater
                .check()
                .await
                .map_err(|err| ("updates.unreachable", err.to_string())),
            Err(err) => Err(("updates.check_failed", err.to_string())),
        };
        match result {
            Ok(Some(update)) => prompt_and_install(app, update),
            Ok(None) if !silent => {
                update_dialog(&app, MessageDialogKind::Info, crate::i18n::t("updates.latest"))
            }
            Ok(None) => {}
            Err((key, err)) if !silent => update_dialog(
                &app,
                MessageDialogKind::Warning,
                crate::i18n::t_with(key, &[("err", &err)]),
            ),
            Err((_, err)) => eprintln!("update auto-check failed: {err}"),
        }
    });
}

fn update_dialog(app: &AppHandle, kind: MessageDialogKind, text: String) {
    app.dialog()
        .message(text)
        .title(crate::i18n::t("updates.title"))
        .kind(kind)
        .show(|_| {});
}

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
                    Err(err) => update_dialog(
                        &app,
                        MessageDialogKind::Warning,
                        crate::i18n::t_with("updates.install_failed", &[("err", &err.to_string())]),
                    ),
                }
            });
        });
}

#[cfg(target_os = "macos")]
fn relaunch_and_exit(app: &AppHandle) {
    use std::os::unix::process::CommandExt;

    let bundle = std::env::current_exe().ok().and_then(|e| bundle_root(&e));
    let Some(bundle) = bundle else {
        app.restart();
    };

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
