import type { CaptureEntry } from "../shared/ipc";

// Pure stack-ordering logic for the overlay's panel column, kept free of
// Tauri/DOM imports so vitest covers it. Index 0 is the top of the visual
// stack (newest panel).

/** Push on top; an entry already in the stack moves up instead of duplicating. */
export function pushTop(stack: CaptureEntry[], entry: CaptureEntry): CaptureEntry[] {
  return [entry, ...stack.filter((e) => e.id !== entry.id)];
}

export function removeEntry(stack: CaptureEntry[], id: string): CaptureEntry[] {
  return stack.filter((e) => e.id !== id);
}

/** Keep the newest `max` panels, dropping from the bottom of the stack. */
export function trimStack(stack: CaptureEntry[], max: number): CaptureEntry[] {
  return stack.slice(0, Math.max(max, 0));
}
