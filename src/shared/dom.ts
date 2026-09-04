import type { Platform } from "./accelerator";

export const el = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;

export const PLATFORM: Platform = /mac/i.test(navigator.platform) ? "mac" : "other";
