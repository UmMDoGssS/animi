import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

type Palette = Record<string, string>;

const css = readFileSync(join(process.cwd(), "src/app.css"), "utf8");
const appHtml = readFileSync(join(process.cwd(), "src/app.html"), "utf8");

function parsePalette(selector: string): Palette {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const block = css.match(
    new RegExp(`${escaped}\\s*\\{([\\s\\S]*?)\\n\\}`, "m"),
  );
  expect(block, `missing ${selector} palette`).not.toBeNull();
  return Object.fromEntries(
    [...(block?.[1] ?? "").matchAll(/--([\w-]+):\s*(#[0-9a-f]{6});/gi)].map(
      ([, name, value]) => [name, value],
    ),
  );
}

function luminance(hex: string): number {
  const channels = [1, 3, 5].map((start) =>
    Number.parseInt(hex.slice(start, start + 2), 16),
  );
  const [red, green, blue] = channels.map((channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return red * 0.2126 + green * 0.7152 + blue * 0.0722;
}

function contrast(first: string, second: string): number {
  const firstLuminance = luminance(first);
  const secondLuminance = luminance(second);
  return (
    (Math.max(firstLuminance, secondLuminance) + 0.05) /
    (Math.min(firstLuminance, secondLuminance) + 0.05)
  );
}

describe("release hardening", () => {
  it.each([
    [':root[data-theme="latte"]', "surface-raised"],
    [':root[data-theme="macchiato"]', "surface-raised"],
  ])(
    "keeps semantic text readable in the %s palette",
    (selector, backgroundToken) => {
      const palette = parsePalette(selector);
      for (const token of [
        "text",
        "text-muted",
        "accent",
        "info",
        "success",
        "warning",
        "danger",
      ]) {
        expect(
          contrast(palette[token], palette[backgroundToken]),
          `${token} must meet WCAG AA against ${backgroundToken}`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    },
  );

  it("retains visible focus and collapses animation for reduced motion", () => {
    expect(css).toMatch(/:focus-visible\s*\{[\s\S]*?outline:\s*2px solid/);
    expect(css).toMatch(
      /@media \(prefers-reduced-motion: reduce\)[\s\S]*?transition-duration:\s*0\.01ms !important/,
    );
    expect(css).toMatch(/animation-iteration-count:\s*1 !important/);
    expect(css).toMatch(/scroll-behavior:\s*auto !important/);
  });

  it("ships Animi identity instead of scaffold metadata", () => {
    expect(appHtml).toContain("<title>Animi</title>");
    expect(appHtml).not.toMatch(/Tauri \+ SvelteKit|Typescript App/i);
  });
});
