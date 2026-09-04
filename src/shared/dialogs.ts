import { save } from "@tauri-apps/plugin-dialog";

import { t } from "./i18n";
import type { CaptureKind } from "./ipc";

export function saveCaptureAs(defaultPath: string, kind: CaptureKind): Promise<string | null> {
  const filter =
    kind === "video"
      ? { name: t("dialogs.mp4_filter"), extensions: ["mp4"] }
      : { name: t("dialogs.png_filter"), extensions: ["png"] };
  return save({ defaultPath, filters: [filter] });
}

export function savePngAs(defaultPath: string): Promise<string | null> {
  return saveCaptureAs(defaultPath, "image");
}
