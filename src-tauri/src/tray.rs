use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::capture::CaptureMode;
use crate::commands::{trigger_capture, AppState};
use crate::i18n::t;
use crate::shortcuts::ShortcutAction;
use crate::windows;

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    #[allow(unused_mut)]
    let mut area_label = t("tray.capture_area");
    #[allow(unused_mut)]
    let mut window_label = t("tray.capture_window");
    #[cfg(target_os = "linux")]
    {
        area_label.push('…');
        window_label.push('…');
    }

    let settings = app.state::<AppState>().settings.get();
    let area = MenuItem::with_id(
        app,
        "capture_area",
        &area_label,
        true,
        Some(settings.shortcut(ShortcutAction::Area)),
    )?;
    let window = MenuItem::with_id(
        app,
        "capture_window",
        &window_label,
        true,
        Some(settings.shortcut(ShortcutAction::Window)),
    )?;
    let fullscreen = MenuItem::with_id(
        app,
        "capture_fullscreen",
        t("tray.capture_fullscreen"),
        true,
        Some(settings.shortcut(ShortcutAction::Fullscreen)),
    )?;
    let item = |id: &str, key: &str| MenuItem::with_id(app, id, t(key), true, None::<&str>);
    #[cfg(target_os = "macos")]
    let scrolling = item("capture_scrolling", "tray.capture_scrolling")?;
    let timer_3 = item("timer_3", "tray.timer_3")?;
    let timer_5 = item("timer_5", "tray.timer_5")?;
    let timer_10 = item("timer_10", "tray.timer_10")?;
    let self_timer =
        Submenu::with_items(app, t("tray.self_timer"), true, &[&timer_3, &timer_5, &timer_10])?;
    let history = item("history", "tray.history")?;
    let updates = item("updates", "tray.updates")?;
    let welcome = item("welcome", "tray.welcome")?;
    let settings_item = MenuItem::with_id(app, "settings", t("tray.settings"), true, Some("CmdOrCtrl+,"))?;
    let quit = MenuItem::with_id(app, "quit", t("tray.quit"), true, Some("CmdOrCtrl+Q"))?;

    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let sep4 = PredefinedMenuItem::separator(app)?;
    let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        vec![&area, &window, &fullscreen, &sep1];
    #[cfg(target_os = "macos")]
    items.push(&scrolling);
    items.push(&self_timer);
    items.extend_from_slice(&[
        &sep2, &history, &sep3, &welcome, &updates, &sep4, &settings_item, &quit,
    ]);
    Menu::with_items(app, &items)
}

pub fn refresh(app: &AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(build_menu(app)?))?;
    }
    Ok(())
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("bundle has an icon").clone())
        .icon_as_template(true)
        .tooltip("Screen for me")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture_area" => trigger_capture(app, CaptureMode::Area),
            "capture_window" => trigger_capture(app, CaptureMode::Window),
            "capture_fullscreen" => trigger_capture(app, CaptureMode::Fullscreen),
            "capture_scrolling" => {
                if let Err(err) = windows::open_scrollcap(app) {
                    eprintln!("failed to open scrolling capture: {err}");
                }
            }
            id if id.starts_with("timer_") => {
                if let Ok(seconds) = id["timer_".len()..].parse() {
                    crate::commands::start_timed_capture(app, seconds);
                }
            }
            "history" => {
                if let Err(err) = windows::open_history(app.clone()) {
                    eprintln!("failed to open history: {err}");
                }
            }
            "welcome" => {
                if let Err(err) = windows::open_welcome(app.clone()) {
                    eprintln!("failed to open welcome window: {err}");
                }
            }
            "updates" => windows::check_for_updates(app, false),
            "settings" => windows::open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
