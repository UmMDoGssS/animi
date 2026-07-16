import type { ThemePreference } from "$lib/ipc/types";

export const themeIds = ["latte", "macchiato"] as const;

export type ThemeId = (typeof themeIds)[number];

export const themeOptions = [
  {
    value: "SYSTEM",
    label: "System",
    description: "Follow the Windows color preference.",
  },
  {
    value: "LATTE",
    label: "Light",
    description: "Always use Catppuccin Latte.",
  },
  {
    value: "MACCHIATO",
    label: "Dark",
    description: "Always use Catppuccin Macchiato.",
  },
] as const satisfies readonly {
  value: ThemePreference;
  label: string;
  description: string;
}[];

export function applyThemePreference(
  preference: ThemePreference,
  root: HTMLElement = document.documentElement,
): void {
  if (preference === "SYSTEM") {
    root.removeAttribute("data-theme");
    return;
  }

  root.dataset.theme = preference === "LATTE" ? "latte" : "macchiato";
}

export const semanticColorTokens = [
  "surface",
  "surface-raised",
  "surface-sunken",
  "overlay",
  "text",
  "text-muted",
  "accent",
  "accent-strong",
  "on-accent",
  "success",
  "warning",
  "danger",
  "border",
  "control-border",
  "glass",
  "selection",
] as const;
