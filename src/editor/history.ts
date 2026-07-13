/** Snapshot-based undo/redo stack. `T` must be treated as immutable. */
export class UndoStack<T> {
  private past: T[] = [];
  private future: T[] = [];

  constructor(private present: T) {}

  get current(): T {
    return this.present;
  }

  get canUndo(): boolean {
    return this.past.length > 0;
  }

  get canRedo(): boolean {
    return this.future.length > 0;
  }

  /** Record a new state; clears the redo branch. No-op if unchanged. */
  commit(next: T): void {
    if (next === this.present) return;
    this.past.push(this.present);
    this.present = next;
    this.future = [];
  }

  undo(): T | null {
    const prev = this.past.pop();
    if (prev === undefined) return null;
    this.future.push(this.present);
    this.present = prev;
    return prev;
  }

  redo(): T | null {
    const next = this.future.pop();
    if (next === undefined) return null;
    this.past.push(this.present);
    this.present = next;
    return next;
  }
}
