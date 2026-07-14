import { save } from "@tauri-apps/plugin-dialog";

import { t } from "./i18n";

/** Open a "save as PNG" dialog; resolves to the chosen path or null on cancel. */
export function savePngAs(defaultPath: string): Promise<string | null> {
  return save({
    defaultPath,
    filters: [{ name: t("dialogs.png_filter"), extensions: ["png"] }],
  });
}
