import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ImportPreviewResponse } from "$lib/ipc/types";
import { setTestViewport, testViewports } from "../../../test/viewports";

import SettingsPage from "./SettingsPage.svelte";

const preview: ImportPreviewResponse = {
  fileName: "animi-recovery.json",
  schemaVersion: 1,
  exportedAt: "2026-07-17T08:00:00.000Z",
  settingsCount: 1,
  libraryEntryCount: 4,
  watchActivityCount: 7,
  changes: {
    settingsAdded: 0,
    settingsUpdated: 1,
    settingsKeptLocal: 0,
    libraryAdded: 2,
    libraryMerged: 1,
    libraryKeptLocal: 1,
    libraryConflicts: 2,
    activityAdded: 5,
    activityUnchanged: 2,
    unrelatedLocalLibraryEntriesRetained: 3,
  },
  validationErrors: [],
  canApply: true,
  importToken: "00000000-0000-4000-a000-000000000010",
  preservesUnrelatedLocalEntries: true,
};

function services() {
  return {
    saveBackup: vi.fn().mockResolvedValue({
      schemaVersion: 1,
      exportedAt: "2026-07-17T10:00:00.000Z",
      settingsCount: 1,
      libraryEntryCount: 4,
      watchActivityCount: 7,
    }),
    chooseImport: vi.fn().mockResolvedValue(preview),
    restoreBackup: vi.fn().mockResolvedValue({
      schemaVersion: 1,
      changes: preview.changes,
    }),
    clearData: vi.fn().mockResolvedValue({
      settingsRemoved: 1,
      libraryEntriesRemoved: 4,
      watchActivityRemoved: 7,
      cachedRecordsRemoved: 15,
    }),
    localStateChanged: vi.fn().mockResolvedValue(undefined),
  };
}

function renderSettings(
  selectThemePreference = vi.fn().mockResolvedValue(undefined),
  overrides: Partial<ReturnType<typeof services>> = {},
) {
  const recoveryServices = { ...services(), ...overrides };
  const setDebugPresentationMode = vi.fn();
  return {
    ...render(SettingsPage, {
      themePreference: "SYSTEM",
      themeStatus: "ready",
      themeError: null,
      selectThemePreference,
      retryThemePreference: vi.fn().mockResolvedValue(undefined),
      debugPresentationMode: "off",
      setDebugPresentationMode,
      ...recoveryServices,
    }),
    recoveryServices,
    selectThemePreference,
    setDebugPresentationMode,
  };
}

describe("SettingsPage", () => {
  it.each(testViewports)(
    "renders live local controls at the $name viewport",
    async (viewport) => {
      setTestViewport(viewport);
      renderSettings();

      expect(
        screen.getByRole("heading", { level: 1, name: "Settings" }),
      ).toBeInTheDocument();
      for (const sectionHeading of [
        "Appearance",
        "State debugger",
        "Backup & recovery",
        "Clear local data",
      ]) {
        expect(
          screen.getByRole("heading", { level: 2, name: sectionHeading }),
        ).toBeInTheDocument();
      }
      expect(screen.queryByText(/cache freshness/i)).not.toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: "Export backup" }),
      ).toBeEnabled();
      expect(
        screen.getByRole("button", { name: "Choose backup to restore" }),
      ).toBeEnabled();
      expect(
        screen.getByText("ฅ^•ﻌ•^ฅ").closest('[aria-hidden="true"]'),
      ).not.toBeNull();
    },
  );

  it("offers session-only forced states without changing durable settings", async () => {
    const user = userEvent.setup();
    const { setDebugPresentationMode, recoveryServices } = renderSettings();

    expect(screen.getByRole("radio", { name: /Live app/ })).toBeChecked();
    expect(screen.getByText("Session only")).toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /^Loading/ }));
    expect(setDebugPresentationMode).toHaveBeenCalledWith("loading");
    expect(recoveryServices.localStateChanged).not.toHaveBeenCalled();

    await user.click(screen.getByRole("radio", { name: /^Error/ }));
    expect(setDebugPresentationMode).toHaveBeenCalledWith("error");
    expect(recoveryServices.localStateChanged).not.toHaveBeenCalled();
  });

  it("keeps the persisted appearance control live and keyboard operable", async () => {
    const user = userEvent.setup();
    const selectThemePreference = vi.fn().mockResolvedValue(undefined);
    renderSettings(selectThemePreference);
    const dark = screen.getByRole("radio", { name: /Dark/ });
    dark.focus();
    await user.keyboard(" ");
    expect(dark).toHaveFocus();
    expect(selectThemePreference).toHaveBeenCalledWith("MACCHIATO");
  });

  it("exports and reports only user-owned backup counts", async () => {
    const user = userEvent.setup();
    const { recoveryServices } = renderSettings();
    await user.click(screen.getByRole("button", { name: "Export backup" }));
    expect(recoveryServices.saveBackup).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole("status")).toHaveTextContent(
      /4 library titles and 7 watch history updates/i,
    );
  });

  it("previews the recovery ledger before applying the opaque import token", async () => {
    const user = userEvent.setup();
    const { recoveryServices } = renderSettings();
    await user.click(
      screen.getByRole("button", { name: "Choose backup to restore" }),
    );

    expect(await screen.findByText("animi-recovery.json")).toBeInTheDocument();
    const ledger = screen.getByRole("group", { name: "Import change preview" });
    expect(ledger).toHaveTextContent(/new titles\s*2/i);
    expect(ledger).toHaveTextContent(/updated titles\s*1/i);
    expect(ledger).toHaveTextContent(/history updates\s*5/i);
    expect(
      screen.getByRole("note", { name: "Recovery safety guarantee" }),
    ).toHaveTextContent(/will stay in your library/i);

    await user.click(
      screen.getByRole("button", { name: "Restore this backup" }),
    );
    expect(recoveryServices.restoreBackup).toHaveBeenCalledWith({
      importToken: preview.importToken,
    });
    await waitFor(() =>
      expect(recoveryServices.localStateChanged).toHaveBeenCalledTimes(1),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      /recovery complete/i,
    );
  });

  it("shows validation errors and never offers apply for an invalid backup", async () => {
    const user = userEvent.setup();
    renderSettings(undefined, {
      chooseImport: vi.fn().mockResolvedValue({
        ...preview,
        canApply: false,
        importToken: null,
        validationErrors: [
          {
            code: "BACKUP_SCHEMA_VERSION_UNSUPPORTED",
            message: "Backup schema version 9 is not supported.",
            path: "schemaVersion",
          },
        ],
      }),
    });
    await user.click(
      screen.getByRole("button", { name: "Choose backup to restore" }),
    );
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(
      "Backup schema version 9 is not supported.",
    );
    expect(alert).not.toHaveTextContent("BACKUP_SCHEMA_VERSION_UNSUPPORTED");
    expect(
      screen.queryByRole("button", { name: "Restore this backup" }),
    ).not.toBeInTheDocument();
  });

  it("requires the exact destructive confirmation and keeps cancellation harmless", async () => {
    const user = userEvent.setup();
    const { recoveryServices } = renderSettings();
    await user.click(screen.getByRole("button", { name: "Clear local data" }));
    const confirmation = screen.getByRole("textbox", {
      name: /Type CLEAR MY LOCAL DATA/i,
    });
    const destructive = screen.getByRole("button", {
      name: "Permanently clear local data",
    });
    expect(destructive).toBeDisabled();

    await user.type(confirmation, "CLEAR MY DATA");
    expect(destructive).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Keep my data" }));
    expect(recoveryServices.clearData).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Clear local data" }));
    await user.type(
      screen.getByRole("textbox", { name: /Type CLEAR MY LOCAL DATA/i }),
      "CLEAR MY LOCAL DATA",
    );
    await user.click(
      screen.getByRole("button", { name: "Permanently clear local data" }),
    );
    expect(recoveryServices.clearData).toHaveBeenCalledWith({
      confirmation: "CLEAR MY LOCAL DATA",
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      /Removed 4 library titles and 7 watch history updates/i,
    );
    expect(recoveryServices.localStateChanged).toHaveBeenCalledTimes(1);
  });

  it("keeps backup actions visually distinct from the danger region", () => {
    renderSettings();
    const dangerRegion = screen.getByRole("region", {
      name: "Clear local data",
    });
    expect(dangerRegion).toHaveClass("border-danger/60");
    expect(
      within(dangerRegion).getByRole("button", { name: "Clear local data" }),
    ).toBeInTheDocument();
    expect(
      within(dangerRegion).queryByRole("button", { name: "Export backup" }),
    ).not.toBeInTheDocument();
  });
});
