import { save } from "@tauri-apps/plugin-dialog";

/** Open a "save as PNG" dialog; resolves to the chosen path or null on cancel. */
export function savePngAs(defaultPath: string): Promise<string | null> {
  return save({
    defaultPath,
    filters: [{ name: "PNG image", extensions: ["png"] }],
  });
}
