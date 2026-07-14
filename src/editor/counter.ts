/** Next badge number: one past the max existing (1 when none). */
export function nextCounterNumber(existing: number[]): number {
  return existing.length === 0 ? 1 : Math.max(...existing) + 1;
}

/**
 * Text colour that stays legible on a badge of the given fill: black on
 * light fills (white, yellow), white on everything else. Threshold picked
 * so the palette's green/red/blue/purple keep white text.
 */
export function counterTextColor(fill: string): string {
  const luminance = relativeLuminance(fill);
  return luminance !== null && luminance > 0.6 ? "#000000" : "#ffffff";
}

/** WCAG relative luminance of a #rgb/#rrggbb hex colour, or null if unparseable. */
function relativeLuminance(hex: string): number | null {
  const match = /^#(?:([0-9a-f]{3})|([0-9a-f]{6}))$/i.exec(hex.trim());
  if (!match) return null;
  const digits = match[1]
    ? [...match[1]].map((d) => d + d).join("")
    : match[2];
  const channel = (i: number) => {
    const srgb = parseInt(digits.slice(i * 2, i * 2 + 2), 16) / 255;
    return srgb <= 0.04045 ? srgb / 12.92 : ((srgb + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2);
}
