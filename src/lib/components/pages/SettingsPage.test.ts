import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { setTestViewport, testViewports } from "../../../test/viewports";

import SettingsPage from "./SettingsPage.svelte";

function renderSettings(
  selectThemePreference = vi.fn().mockResolvedValue(undefined),
) {
  return render(SettingsPage, {
    themePreference: "SYSTEM",
    themeStatus: "ready",
    themeError: null,
    selectThemePreference,
    retryThemePreference: vi.fn().mockResolvedValue(undefined),
  });
}

describe("SettingsPage", () => {
  it.each(testViewports)(
    "renders the truthful local settings presentation at the $name viewport",
    (viewport) => {
      setTestViewport(viewport);
      renderSettings();

      expect(
        screen.getByRole("heading", { level: 1, name: "Settings" }),
      ).toBeInTheDocument();
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(screen.getByRole("tabpanel")).toHaveAttribute(
        "id",
        "settings-panel",
      );
      for (const sectionHeading of [
        "Appearance",
        "Local by design",
        "Cache refresh policy",
        "Backup & recovery",
        "Clear local data",
      ]) {
        expect(
          screen.getByRole("heading", { level: 2, name: sectionHeading }),
        ).toBeInTheDocument();
      }
      expect(screen.getAllByRole("heading", { level: 2 })).toHaveLength(5);
      expect(
        screen.getByRole("heading", { level: 3, name: "Backup manifest" }),
      ).toBeInTheDocument();

      expect(
        screen.getByRole("note", { name: "Settings preview disclosure" }),
      ).toHaveTextContent(/appearance changes are saved on this device/i);
      expect(
        screen.getByRole("note", { name: "Settings preview disclosure" }),
      ).toHaveTextContent(/layout previews only/i);

      expect(screen.getAllByRole("radio")).toHaveLength(3);
      expect(screen.getByRole("radio", { name: /System/ })).toBeChecked();

      const localManifest = screen.getByRole("group", {
        name: "Local-only manifest",
      });
      expect(localManifest).toHaveTextContent(/storage\s*this Windows device/i);
      expect(localManifest).toHaveTextContent(/account\s*none/i);
      expect(localManifest).toHaveTextContent(/cloud sync\s*none/i);
      expect(localManifest).toHaveTextContent(/analytics\s*none/i);
      const localCat = screen.getByText("(=^._.^=) // device only");
      expect(localCat.closest('[aria-hidden="true"]')).not.toBeNull();

      const cachePolicy = screen.getByRole("group", {
        name: "AniList cache refresh policy",
      });
      expect(cachePolicy).toHaveTextContent(/airing data\s*30 minutes/i);
      expect(cachePolicy).toHaveTextContent(
        /discover \+ recommendations\s*6 hours/i,
      );
      expect(cachePolicy).toHaveTextContent(/media details\s*24 hours/i);
      expect(screen.getByText("no live values")).toBeInTheDocument();
      expect(
        screen.getByText(/does not inspect cache records/i),
      ).toBeInTheDocument();

      const included = screen.getByRole("list", {
        name: "Backup contents preview",
      });
      expect(
        within(included).getByText("Theme and local settings"),
      ).toBeInTheDocument();
      expect(within(included).getByText("Library entries")).toBeInTheDocument();
      expect(
        within(included).getByText("Recorded watch activity"),
      ).toBeInTheDocument();

      const excluded = screen.getByRole("list", {
        name: "Backup exclusions preview",
      });
      expect(
        within(excluded).getByText("Cached AniList responses"),
      ).toBeInTheDocument();
      expect(
        within(excluded).getByText("Downloaded images"),
      ).toBeInTheDocument();
      expect(
        screen.getByText(/validate the whole file and preview conflicts/i),
      ).toBeInTheDocument();
      expect(
        screen.getByText(/unrelated local library entries will stay in place/i),
      ).toBeInTheDocument();

      for (const action of [
        "Export backup // preview only",
        "Restore backup // preview only",
        "Clear local data // preview only",
      ]) {
        expect(screen.getByRole("button", { name: action })).toBeDisabled();
      }

      const dangerRegion = screen.getByRole("region", {
        name: "Clear local data",
      });
      expect(dangerRegion).toHaveClass("border-danger/60");
      expect(dangerRegion).toHaveTextContent(
        /require a specific confirmation/i,
      );
      expect(dangerRegion).toHaveTextContent(/will never run silently/i);
    },
  );

  it("keeps the persisted appearance control live", async () => {
    const user = userEvent.setup();
    const selectThemePreference = vi.fn().mockResolvedValue(undefined);
    renderSettings(selectThemePreference);

    await user.click(screen.getByRole("radio", { name: /Dark/ }));

    expect(selectThemePreference).toHaveBeenCalledWith("MACCHIATO");
  });

  it("supports keyboard theme selection and disables changes while saving", async () => {
    const user = userEvent.setup();
    const selectThemePreference = vi.fn().mockResolvedValue(undefined);
    const { rerender } = renderSettings(selectThemePreference);
    const lightOption = screen.getByRole("radio", { name: /Light/ });

    lightOption.focus();
    await user.keyboard(" ");

    expect(lightOption).toHaveFocus();
    expect(selectThemePreference).toHaveBeenCalledWith("LATTE");

    await rerender({
      themePreference: "LATTE",
      themeStatus: "saving",
      themeError: null,
      selectThemePreference,
      retryThemePreference: vi.fn().mockResolvedValue(undefined),
    });

    for (const option of screen.getAllByRole("radio")) {
      expect(option).toBeDisabled();
    }
    expect(screen.getByRole("status")).toHaveTextContent(
      /saving the palette on this device/i,
    );
    expect(
      screen
        .getByRole("heading", { level: 2, name: "Appearance" })
        .closest("section"),
    ).toHaveAttribute("aria-busy", "true");
  });
});
