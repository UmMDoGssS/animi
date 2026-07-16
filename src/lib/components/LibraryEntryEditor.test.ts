import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type {
  CommandError,
  LibraryEntry,
  UpsertLibraryEntryRequest,
} from "$lib/ipc/types";

import LibraryEntryEditor from "./LibraryEntryEditor.svelte";

const existingEntry: LibraryEntry = {
  anilistId: 154_587,
  mediaKind: "ANIME",
  status: "WATCHING",
  progress: 3,
  score: 8.5,
  notes: "Quietly excellent.",
  createdAt: "2026-07-16T08:00:00.000Z",
  updatedAt: "2026-07-17T08:00:00.000Z",
};

function savedEntry(
  request: UpsertLibraryEntryRequest,
  original: LibraryEntry | null = existingEntry,
): LibraryEntry {
  return {
    anilistId: request.anilistId,
    mediaKind: request.mediaKind,
    status: request.status,
    progress: request.progress,
    score: request.score,
    notes: request.notes,
    createdAt: original?.createdAt ?? "2026-07-17T08:00:00.000Z",
    updatedAt: "2026-07-17T09:00:00.000Z",
  };
}

describe("LibraryEntryEditor", () => {
  it("adds an anime with explicit unknown-total guidance and typed drafts", async () => {
    const user = userEvent.setup();
    const saveEntry = vi.fn(async (request: UpsertLibraryEntryRequest) =>
      savedEntry(request, null),
    );
    const onSaved = vi.fn();

    render(LibraryEntryEditor, {
      anilistId: 154_587,
      knownTotal: null,
      entry: null,
      saveEntry,
      onSaved,
    });

    expect(
      screen.getByRole("heading", { name: "Add to your library" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/episode total is unknown/i)).toHaveTextContent(
      /still record progress/i,
    );
    expect(
      screen.queryByRole("button", { name: "Remove from library" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /watching/i }));
    await user.clear(screen.getByLabelText("Episodes watched"));
    await user.type(screen.getByLabelText("Episodes watched"), "14");
    await user.type(screen.getByLabelText(/personal score/i), "9.25");
    await user.type(
      screen.getByLabelText(/private notes/i),
      "Unknown total, known favorite.",
    );
    await user.click(screen.getByRole("button", { name: "Add to library" }));

    expect(saveEntry).toHaveBeenCalledWith({
      anilistId: 154_587,
      mediaKind: "ANIME",
      status: "WATCHING",
      progress: 14,
      score: 9.25,
      notes: "Unknown total, known favorite.",
    });
    expect(onSaved).toHaveBeenCalledWith(
      expect.objectContaining({ progress: 14, score: 9.25 }),
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      "Added to your local library.",
    );
  });

  it("renders existing values and updates every editable field", async () => {
    const user = userEvent.setup();
    const saveEntry = vi.fn(async (request: UpsertLibraryEntryRequest) =>
      savedEntry(request),
    );

    render(LibraryEntryEditor, {
      anilistId: existingEntry.anilistId,
      knownTotal: 28,
      entry: existingEntry,
      saveEntry,
    });

    expect(
      screen.getByRole("heading", { name: "Edit library entry" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Episodes watched")).toHaveValue("3");
    expect(screen.getByLabelText(/personal score/i)).toHaveValue("8.5");
    expect(screen.getByLabelText(/private notes/i)).toHaveValue(
      "Quietly excellent.",
    );
    expect(screen.getByText(/known episode total: 28/i)).toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /completed/i }));
    await user.clear(screen.getByLabelText("Episodes watched"));
    await user.type(screen.getByLabelText("Episodes watched"), "28");
    await user.clear(screen.getByLabelText(/personal score/i));
    await user.type(screen.getByLabelText(/personal score/i), "10");
    await user.clear(screen.getByLabelText(/private notes/i));
    await user.type(screen.getByLabelText(/private notes/i), "A keeper.");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    expect(saveEntry).toHaveBeenCalledWith({
      anilistId: existingEntry.anilistId,
      mediaKind: "ANIME",
      status: "COMPLETED",
      progress: 28,
      score: 10,
      notes: "A keeper.",
    });
    expect(screen.getByRole("status")).toHaveTextContent(
      "Library changes saved on this device.",
    );
  });

  it("keeps malformed drafts visible and reports every client-side error", async () => {
    const user = userEvent.setup();
    const saveEntry = vi.fn();
    const oversizedNotes = "n".repeat(10_001);

    render(LibraryEntryEditor, {
      anilistId: existingEntry.anilistId,
      knownTotal: null,
      entry: existingEntry,
      saveEntry,
    });

    const progress = screen.getByLabelText("Episodes watched");
    const score = screen.getByLabelText(/personal score/i);
    const notes = screen.getByLabelText(/private notes/i);
    await user.clear(progress);
    await user.type(progress, "2.5");
    await user.clear(score);
    await user.type(score, "eleven");
    await fireEvent.input(notes, { target: { value: oversizedNotes } });
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    expect(saveEntry).not.toHaveBeenCalled();
    expect(progress).toHaveValue("2.5");
    expect(score).toHaveValue("eleven");
    expect(notes).toHaveValue(oversizedNotes);
    expect(progress).toHaveAttribute("aria-invalid", "true");
    expect(score).toHaveAttribute("aria-invalid", "true");
    expect(notes).toHaveAttribute("aria-invalid", "true");
    expect(
      screen.getByText("Enter progress as a non-negative whole number."),
    ).toBeInTheDocument();
    expect(screen.getByText(/score from 0 to 10/i)).toBeInTheDocument();
    expect(screen.getByText(/cannot exceed 10,000/i)).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      /your other edits are still here/i,
    );
  });

  it("enforces a known episode bound without changing the draft", async () => {
    const user = userEvent.setup();
    const saveEntry = vi.fn();

    render(LibraryEntryEditor, {
      anilistId: existingEntry.anilistId,
      knownTotal: 12,
      entry: existingEntry,
      saveEntry,
    });

    const progress = screen.getByLabelText("Episodes watched");
    await user.clear(progress);
    await user.type(progress, "13");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    expect(saveEntry).not.toHaveBeenCalled();
    expect(progress).toHaveValue("13");
    expect(
      screen.getByText(/cannot exceed the known total of 12/i),
    ).toBeInTheDocument();
  });

  it("shows a structured backend error without discarding any edits", async () => {
    const user = userEvent.setup();
    const backendError: CommandError = {
      code: "DATABASE_UNAVAILABLE",
      message: "The local library could not be saved.",
      retryable: true,
    };
    const saveEntry = vi.fn().mockRejectedValue(backendError);

    render(LibraryEntryEditor, {
      anilistId: existingEntry.anilistId,
      knownTotal: 28,
      entry: existingEntry,
      saveEntry,
    });

    await user.click(screen.getByRole("radio", { name: /planning/i }));
    await user.clear(screen.getByLabelText("Episodes watched"));
    await user.type(screen.getByLabelText("Episodes watched"), "7");
    await user.clear(screen.getByLabelText(/personal score/i));
    await user.type(screen.getByLabelText(/personal score/i), "7.75");
    await user.clear(screen.getByLabelText(/private notes/i));
    await user.type(
      screen.getByLabelText(/private notes/i),
      "Keep this draft.",
    );
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveAttribute("data-error-code", "DATABASE_UNAVAILABLE");
    expect(alert).toHaveTextContent("DATABASE_UNAVAILABLE");
    expect(alert).toHaveTextContent("The local library could not be saved.");
    expect(screen.getByRole("radio", { name: /planning/i })).toBeChecked();
    expect(screen.getByLabelText("Episodes watched")).toHaveValue("7");
    expect(screen.getByLabelText(/personal score/i)).toHaveValue("7.75");
    expect(screen.getByLabelText(/private notes/i)).toHaveValue(
      "Keep this draft.",
    );
  });

  it("requires an explicit second removal action and explains retained data", async () => {
    const user = userEvent.setup();
    const deleteEntry = vi.fn().mockResolvedValue({ removed: true });
    const onRemoved = vi.fn();

    render(LibraryEntryEditor, {
      anilistId: existingEntry.anilistId,
      knownTotal: 28,
      entry: existingEntry,
      saveEntry: vi.fn(),
      deleteEntry,
      onRemoved,
    });

    const removeButton = screen.getByRole("button", {
      name: "Remove from library",
    });
    await user.click(removeButton);

    expect(deleteEntry).not.toHaveBeenCalled();
    const confirmation = screen.getByRole("heading", {
      name: "Remove this library entry?",
    }).parentElement;
    expect(confirmation).not.toBeNull();
    expect(confirmation).toHaveTextContent(
      "This removes only the local library record.",
    );
    expect(confirmation).toHaveTextContent("AniList is unaffected.");
    expect(confirmation).toHaveTextContent(
      "Recorded watch activity is retained.",
    );
    expect(
      screen.getByRole("button", { name: "Confirm local removal" }),
    ).toHaveFocus();

    await user.click(screen.getByRole("button", { name: "Keep entry" }));
    expect(deleteEntry).not.toHaveBeenCalled();
    const restoredRemoveButton = screen.getByRole("button", {
      name: "Remove from library",
    });
    expect(restoredRemoveButton).toHaveFocus();

    await user.click(restoredRemoveButton);
    const confirmationRegion = screen
      .getByRole("heading", { name: "Remove this library entry?" })
      .closest("section");
    expect(confirmationRegion).not.toBeNull();
    await user.click(
      within(confirmationRegion as HTMLElement).getByRole("button", {
        name: "Confirm local removal",
      }),
    );

    expect(deleteEntry).toHaveBeenCalledOnce();
    expect(deleteEntry).toHaveBeenCalledWith({
      anilistId: existingEntry.anilistId,
      mediaKind: "ANIME",
      confirmed: true,
    });
    expect(onRemoved).toHaveBeenCalledOnce();
  });

  it("keeps the confirmation and all drafts when removal fails", async () => {
    const user = userEvent.setup();
    const deleteEntry = vi.fn().mockRejectedValue({
      code: "DATABASE_UNAVAILABLE",
      message: "The local library could not be changed.",
      retryable: true,
    } satisfies CommandError);

    render(LibraryEntryEditor, {
      anilistId: existingEntry.anilistId,
      knownTotal: 28,
      entry: existingEntry,
      saveEntry: vi.fn(),
      deleteEntry,
    });

    await user.clear(screen.getByLabelText(/private notes/i));
    await user.type(screen.getByLabelText(/private notes/i), "Unsaved note");
    await user.click(
      screen.getByRole("button", { name: "Remove from library" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Confirm local removal" }),
    );

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveAttribute("data-error-code", "DATABASE_UNAVAILABLE");
    expect(alert).toHaveTextContent("removal failed");
    expect(screen.getByLabelText(/private notes/i)).toHaveValue("Unsaved note");
    expect(
      screen.getByRole("button", { name: "Confirm local removal" }),
    ).toBeInTheDocument();
  });
});
