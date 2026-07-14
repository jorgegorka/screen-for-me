/** Next badge number: one past the max existing (1 when none). */
export function nextCounterNumber(existing: number[]): number {
  return existing.length === 0 ? 1 : Math.max(...existing) + 1;
}
