import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import {
  applyThemePreference,
  pageAccentTokens,
  semanticColorTokens,
  themeIds,
  translucentSurfaceTokens,
} from "./theme";

const appStyles = readFileSync(resolve("src/app.css"), "utf8");

type ThemeId = (typeof themeIds)[number];
type Rgb = readonly [number, number, number];

const surfaces = ["surface", "surface-raised", "surface-sunken"] as const;
const textColors = [
  "text",
  "text-muted",
  "accent",
  "info",
  "success",
  "warning",
  "danger",
  "stat-tracked",
  "stat-watching",
  "stat-completed",
  "stat-planning",
  ...pageAccentTokens,
] as const;

function cssBlock(marker: string): string {
  const markerStart = appStyles.indexOf(marker);
  if (markerStart < 0) {
    throw new Error(`Missing CSS marker: ${marker}`);
  }

  const blockStart = appStyles.indexOf("{", markerStart);
  if (blockStart < 0) {
    throw new Error(`Missing CSS block: ${marker}`);
  }

  let depth = 0;
  for (let index = blockStart; index < appStyles.length; index += 1) {
    if (appStyles[index] === "{") {
      depth += 1;
    } else if (appStyles[index] === "}") {
      depth -= 1;
      if (depth === 0) {
        return appStyles.slice(blockStart + 1, index);
      }
    }
  }

  throw new Error(`Unclosed CSS block: ${marker}`);
}

function themeBlock(themeId: ThemeId): string {
  return cssBlock(`:root[data-theme="${themeId}"]`);
}

function solidHexToken(block: string, token: string): string {
  const value = block.match(
    new RegExp(`--${token}:\\s*(#[0-9a-f]{6})\\s*;`, "i"),
  )?.[1];
  if (!value) {
    throw new Error(`Token --${token} must be a six-digit solid hex color.`);
  }

  return value;
}

function tokenValue(block: string, token: string): string {
  const value = block.match(new RegExp(`--${token}:\\s*([^;]+);`, "i"))?.[1];
  if (!value) {
    throw new Error(`Missing token --${token}.`);
  }

  return value.trim();
}

function rgb(hex: string): Rgb {
  return [
    Number.parseInt(hex.slice(1, 3), 16),
    Number.parseInt(hex.slice(3, 5), 16),
    Number.parseInt(hex.slice(5, 7), 16),
  ];
}

function relativeLuminance(hex: string): number {
  function linearize(channel: number): number {
    const normalized = channel / 255;
    return normalized <= 0.04045
      ? normalized / 12.92
      : ((normalized + 0.055) / 1.055) ** 2.4;
  }

  const [red, green, blue] = rgb(hex);

  return (
    0.2126 * linearize(red) +
    0.7152 * linearize(green) +
    0.0722 * linearize(blue)
  );
}

function contrastRatio(first: string, second: string): number {
  const lighter = Math.max(relativeLuminance(first), relativeLuminance(second));
  const darker = Math.min(relativeLuminance(first), relativeLuminance(second));
  return (lighter + 0.05) / (darker + 0.05);
}

describe("theme contract", () => {
  it.each(themeIds)("defines every semantic token for %s", (themeId) => {
    const block = themeBlock(themeId);

    for (const token of semanticColorTokens) {
      expect(block).toContain(`--${token}:`);
    }
  });

  it.each(themeIds)("uses one glass density across %s surfaces", (themeId) => {
    const block = themeBlock(themeId);

    for (const token of translucentSurfaceTokens) {
      expect(tokenValue(block, token)).toContain("var(--glass-density)");
    }

    const density = Number.parseFloat(tokenValue(block, "glass-density"));
    expect(density).toBeGreaterThanOrEqual(50);
    expect(density).toBeLessThanOrEqual(55);
  });

  it("keeps colored action and selected navigation fills fully opaque", () => {
    const accentAction = cssBlock(".glass-accent,");
    const selectedTab = cssBlock('.glass-tab[data-page][aria-selected="true"]');

    expect(accentAction).toContain("background: var(--accent);");
    expect(selectedTab).toContain("background: var(--tab-accent);");
    expect(appStyles).not.toContain("--accent-glass:");
  });

  it("defines the askew Catppuccin rainbow as a decorative shell layer", () => {
    const block = cssBlock(".rainbow-band");

    for (const token of [
      "rainbow-red",
      "rainbow-peach",
      "rainbow-yellow",
      "rainbow-green",
      "rainbow-sky",
      "rainbow-blue",
      "rainbow-mauve",
    ]) {
      expect(block).toContain(`var(--${token})`);
    }

    expect(block).toContain("rotate(-8deg)");
  });

  it.each(themeIds)("keeps %s text colors readable", (themeId) => {
    const block = themeBlock(themeId);

    for (const textColor of textColors) {
      for (const surface of surfaces) {
        const ratio = contrastRatio(
          solidHexToken(block, textColor),
          solidHexToken(block, surface),
        );
        expect(
          ratio,
          `${themeId} --${textColor} on --${surface}`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    }

    expect(
      contrastRatio(
        solidHexToken(block, "on-accent"),
        solidHexToken(block, "accent"),
      ),
      `${themeId} --on-accent on --accent`,
    ).toBeGreaterThanOrEqual(4.5);

    for (const pageAccent of pageAccentTokens) {
      expect(
        contrastRatio(
          solidHexToken(block, "on-accent"),
          solidHexToken(block, pageAccent),
        ),
        `${themeId} --on-accent on --${pageAccent}`,
      ).toBeGreaterThanOrEqual(4.5);
    }
  });

  it.each(themeIds)("keeps %s control boundaries visible", (themeId) => {
    const block = themeBlock(themeId);
    for (const surface of surfaces) {
      const ratio = contrastRatio(
        solidHexToken(block, "control-border"),
        solidHexToken(block, surface),
      );
      expect(
        ratio,
        `${themeId} --control-border on --${surface}`,
      ).toBeGreaterThanOrEqual(3);
    }
  });

  it("uses Macchiato when the OS prefers dark and no override exists", () => {
    expect(appStyles).toContain(":root:not([data-theme])");
    expect(appStyles).toContain("@media (prefers-color-scheme: dark)");
  });

  it("applies manual palettes and removes the override for system mode", () => {
    const root = document.createElement("html");

    applyThemePreference("LATTE", root);
    expect(root.dataset.theme).toBe("latte");

    applyThemePreference("MACCHIATO", root);
    expect(root.dataset.theme).toBe("macchiato");

    applyThemePreference("SYSTEM", root);
    expect(root).not.toHaveAttribute("data-theme");
  });

  it("globally reduces animation and transition motion", () => {
    const block = cssBlock("@media (prefers-reduced-motion: reduce)");

    expect(block).toContain("*::before");
    expect(block).toContain("*::after");
    expect(block).toContain("scroll-behavior: auto !important;");
    expect(block).toContain("transition-duration: 0.01ms !important;");
    expect(block).toContain("animation-duration: 0.01ms !important;");
    expect(block).toContain("animation-iteration-count: 1 !important;");
  });

  it("keeps global scrollbars visible", () => {
    const universal = cssBlock("* {");
    expect(universal).toContain("scrollbar-width: thin;");
    expect(universal).not.toMatch(/scrollbar-width\s*:\s*none\b/i);

    const webkitScrollbar = cssBlock("*::-webkit-scrollbar");
    expect(webkitScrollbar).not.toMatch(/display:\s*none\b/i);
    expect(webkitScrollbar).not.toMatch(
      /(?:width|height)\s*:\s*0(?:[a-z%]+)?(?:\s*!important)?\s*;/i,
    );
  });
});
