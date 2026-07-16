import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ThemePreference } from "$lib/ipc/types";

import { setTestViewport, testViewports } from "../../test/viewports";

import AppShell from "./AppShell.svelte";

describe("AppShell", () => {
  it.each(testViewports)(
    "renders all four page destinations at the $name viewport",
    async (viewport) => {
      setTestViewport(viewport);
      render(AppShell, {
        loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
        saveThemePreference: vi.fn(),
      });

      const tabs = screen.getAllByRole("tab");
      expect(tabs).toHaveLength(4);
      expect(
        screen.getByRole("tablist", { name: "Primary pages" }),
      ).toHaveAttribute("aria-orientation", "vertical");
      expect(screen.getByRole("tab", { name: "Home" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(screen.getByRole("tab", { name: "Home" })).not.toHaveAttribute(
        "title",
      );
      expect(
        within(screen.getByRole("tab", { name: "Home" })).getByText("Home"),
      ).toHaveAttribute("aria-hidden", "true");
      expect(screen.queryByText("Private & local")).not.toBeInTheDocument();
      const backgroundCat = document.querySelector('pre[aria-hidden="true"]');
      expect(backgroundCat).toHaveTextContent("__..--''``---....___");
      expect(backgroundCat).toHaveTextContent("``--...____..-' /// / //");
      expect(
        screen.getByRole("heading", { level: 1, name: "Home" }),
      ).toBeInTheDocument();
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(screen.getByRole("tabpanel")).toHaveAttribute("id", "home-panel");
      expect(
        screen.getByRole("link", { name: "Skip to content" }),
      ).toHaveAttribute("href", "#main-content");
      expect(
        screen.getByRole("complementary", {
          name: "Primary navigation rail",
        }),
      ).toHaveClass(
        "top-2",
        "h-[calc(100dvh-1rem)]",
        "self-start",
        "sm:top-4",
        "sm:h-[calc(100dvh-2rem)]",
      );

      for (const destination of ["Discover", "Dashboard", "Settings"]) {
        await fireEvent.click(screen.getByRole("tab", { name: destination }));
        expect(
          screen.getByRole("heading", { level: 1, name: destination }),
        ).toBeInTheDocument();
        expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      }
    },
  );

  it("supports roving keyboard navigation and activation", async () => {
    const user = userEvent.setup();
    render(AppShell, {
      loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
      saveThemePreference: vi.fn(),
    });

    const homeTab = screen.getByRole("tab", { name: "Home" });
    const discoverTab = screen.getByRole("tab", { name: "Discover" });
    const dashboardTab = screen.getByRole("tab", { name: "Dashboard" });
    const settingsTab = screen.getByRole("tab", { name: "Settings" });

    expect(homeTab).toHaveAttribute("tabindex", "0");
    expect(homeTab).toHaveAttribute("aria-controls", "home-panel");
    expect(discoverTab).toHaveAttribute("tabindex", "-1");
    homeTab.focus();

    await user.keyboard("{ArrowDown}");
    await waitFor(() => expect(discoverTab).toHaveFocus());
    expect(discoverTab).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("heading", { level: 1, name: "Discover" }),
    ).toBeInTheDocument();

    await user.keyboard("{End}");
    await waitFor(() => expect(settingsTab).toHaveFocus());
    expect(settingsTab).toHaveAttribute("aria-selected", "true");

    await user.keyboard("{Home}");
    await waitFor(() => expect(homeTab).toHaveFocus());

    settingsTab.focus();
    await user.keyboard(" ");
    expect(settingsTab).toHaveAttribute("aria-selected", "true");

    dashboardTab.focus();
    await user.keyboard("{Enter}");
    expect(dashboardTab).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("heading", { level: 1, name: "Dashboard" }),
    ).toBeInTheDocument();
  });

  it("keeps Discover selected while media details open secondarily", async () => {
    const user = userEvent.setup();
    render(AppShell, {
      loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
      saveThemePreference: vi.fn(),
    });

    const discoverTab = screen.getByRole("tab", { name: "Discover" });
    await user.click(discoverTab);
    await user.click(
      screen.getByRole("button", {
        name: "Open sample details for Paper Cranes at Dawn",
      }),
    );

    expect(discoverTab).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("dialog", { name: "Paper Cranes at Dawn" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Home" })).not.toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

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
    await fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
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
    await fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
    await fireEvent.click(screen.getByRole("radio", { name: /Dark/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Animi could not save the theme preference.",
    );
    expect(document.documentElement).toHaveAttribute("data-theme", "latte");
    expect(screen.getByRole("radio", { name: /Light/ })).toBeChecked();
  });
});
