/** Mirrors the Rust struct in src-tauri/src/history.rs. */
export interface CaptureEntry {
  path: string;
  id: string;
  created_ms: number;
}

/** Mirrors the Rust settings struct; persisted app settings. */
export interface Settings {
  position: "left" | "center" | "right";
  move_to_active_screen: boolean;
  overlay_size: number;
  auto_close_enabled: boolean;
  auto_close_action: "close" | "save_and_close";
  auto_close_seconds: number;
  close_after_drag: boolean;
  copy_to_clipboard: boolean;
  language: "system" | "en-GB" | "es" | "fr" | "de" | "it";
  shortcut_area: string;
  shortcut_window: string;
  shortcut_fullscreen: string;
}
