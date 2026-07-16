import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { applyThemePreference, semanticColorTokens, themeIds } from "./theme";

const appStyles = readFileSync(resolve("src/app.css"), "utf8");

describe("theme contract", () => {
  it.each(themeIds)("defines every semantic token for %s", (themeId) => {
    const selector = `:root[data-theme="${themeId}"]`;
    const selectorStart = appStyles.indexOf(selector);
    const blockStart = appStyles.indexOf("{", selectorStart);
    const blockEnd = appStyles.indexOf("}", blockStart);
    const themeBlock = appStyles.slice(blockStart, blockEnd);

    expect(selectorStart).toBeGreaterThanOrEqual(0);
    for (const token of semanticColorTokens) {
      expect(themeBlock).toContain(`--${token}:`);
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
});
