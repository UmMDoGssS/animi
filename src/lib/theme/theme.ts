export const themeIds = ["latte", "macchiato"] as const;

export type ThemeId = (typeof themeIds)[number];

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
  "glass",
] as const;
