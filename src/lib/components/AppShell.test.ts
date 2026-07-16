import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import type { ThemePreference } from "$lib/ipc/types";

import { setTestViewport, testViewports } from "../../test/viewports";

import AppShell from "./AppShell.svelte";

describe("AppShell", () => {
  it.each(testViewports)(
    "navigates between each shell view at the $name viewport",
    async (viewport) => {
      setTestViewport(viewport);
      render(AppShell, {
        loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
        saveThemePreference: vi.fn(),
      });

      expect(
        screen.getByRole("heading", { level: 1, name: "Your library" }),
      ).toBeInTheDocument();

      await fireEvent.click(screen.getByRole("button", { name: "Discover" }));
      expect(
        screen.getByRole("heading", { level: 1, name: "Discover anime" }),
      ).toBeInTheDocument();

      await fireEvent.click(
        screen.getByRole("button", { name: "Preview details view" }),
      );
      expect(
        screen.getByRole("heading", { level: 1, name: "Media details" }),
      ).toBeInTheDocument();

      await fireEvent.click(
        screen.getByRole("button", { name: "Back to discover" }),
      );

      await fireEvent.click(screen.getByRole("button", { name: "For you" }));
      expect(
        screen.getByRole("heading", { level: 1, name: "Recommendations" }),
      ).toBeInTheDocument();

      await fireEvent.click(screen.getByRole("button", { name: "Settings" }));
      expect(
        screen.getByRole("heading", { level: 1, name: "Settings & backup" }),
      ).toBeInTheDocument();
    },
  );

  it("loads, applies, and persists each theme preference", async () => {
    const saveThemePreference = vi.fn(
      async (preference: ThemePreference) => preference,
    );
    render(AppShell, {
      loadThemePreference: vi.fn().mockResolvedValue("MACCHIATO"),
      saveThemePreference,
    });

    await waitFor(() =>
      expect(document.documentElement).toHaveAttribute(
        "data-theme",
        "macchiato",
      ),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("radio", { name: /Dark/ })).toBeChecked();

    await fireEvent.click(screen.getByRole("radio", { name: /Light/ }));
    await waitFor(() =>
      expect(document.documentElement).toHaveAttribute("data-theme", "latte"),
    );
    expect(saveThemePreference).toHaveBeenCalledWith("LATTE");

    await fireEvent.click(screen.getByRole("radio", { name: /System/ }));
    await waitFor(() =>
      expect(document.documentElement).not.toHaveAttribute("data-theme"),
    );
    expect(saveThemePreference).toHaveBeenCalledWith("SYSTEM");
  });

  it("keeps the active theme when saving a preference fails", async () => {
    render(AppShell, {
      loadThemePreference: vi.fn().mockResolvedValue("LATTE"),
      saveThemePreference: vi.fn().mockRejectedValue({
        code: "DATABASE_UNAVAILABLE",
        message: "Animi could not save the theme preference.",
        retryable: true,
      }),
    });

    await waitFor(() =>
      expect(document.documentElement).toHaveAttribute("data-theme", "latte"),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    await fireEvent.click(screen.getByRole("radio", { name: /Dark/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Animi could not save the theme preference.",
    );
    expect(document.documentElement).toHaveAttribute("data-theme", "latte");
    expect(screen.getByRole("radio", { name: /Light/ })).toBeChecked();
  });
});
