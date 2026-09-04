export type CaptureKind = "image" | "video";

export interface CaptureEntry {
  path: string;
  id: string;
  created_ms: number;
  kind: CaptureKind;
  poster: string | null;
}

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
  shortcut_record: string;
}

export interface RecorderPrefs {
  microphone: boolean;
}
