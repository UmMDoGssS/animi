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
            /slider, arrow keys, or step buttons/i,
        );
        expect(
            screen.queryByRole("button", { name: "Remove from library" }),
        ).not.toBeInTheDocument();

        await user.click(screen.getByRole("radio", { name: /watching/i }));
        await fireEvent.input(
            screen.getByRole("slider", { name: "Episodes watched" }),
            {
                target: { value: "14" },
            },
        );
        await user.click(
            screen.getByRole("button", {
                name: "Set personal score to 9 out of 10",
            }),
        );
        await user.type(
            screen.getByLabelText(/private notes/i),
            "Unknown total, known favorite.",
        );
        await user.click(
            screen.getByRole("button", { name: "Add to library" }),
        );

        expect(saveEntry).toHaveBeenCalledWith({
            anilistId: 154_587,
            mediaKind: "ANIME",
            status: "WATCHING",
            progress: 14,
            score: 9,
            notes: "Unknown total, known favorite.",
        });
        expect(onSaved).toHaveBeenCalledWith(
            expect.objectContaining({ progress: 14, score: 9 }),
        );
        expect(screen.getByText("Added to your library.")).toBeInTheDocument();
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
        expect(
            screen.getByRole("slider", { name: "Episodes watched" }),
        ).toHaveValue("3");
        const scoreGroup = screen.getByRole("group", {
            name: "Personal score",
        });
        expect(
            within(scoreGroup)
                .getByRole("button", {
                    name: "Set personal score to 9 out of 10",
                })
                .querySelector(".star-fill"),
        ).toHaveStyle({ width: "50%" });
        expect(screen.getByText("8.5 / 10")).toBeInTheDocument();
        expect(screen.getByLabelText(/private notes/i)).toHaveValue(
            "Quietly excellent.",
        );
        expect(
            screen.getByText(/known episode total: 28/i),
        ).toBeInTheDocument();
        const saveButton = screen.getByRole("button", {
            name: "Changes saved",
        });
        expect(saveButton).toBeDisabled();
        expect(saveButton).toHaveClass("glass-control", "text-muted");
        expect(saveButton).not.toHaveClass("glass-accent", "opacity-55");

        await user.click(screen.getByRole("radio", { name: /completed/i }));
        await fireEvent.input(
            screen.getByRole("slider", { name: "Episodes watched" }),
            {
                target: { value: "28" },
            },
        );
        await user.click(
            screen.getByRole("button", {
                name: "Set personal score to 10 out of 10",
            }),
        );
        await user.clear(screen.getByLabelText(/private notes/i));
        await user.type(screen.getByLabelText(/private notes/i), "A keeper.");
        expect(saveButton).toBeEnabled();
        expect(saveButton).toHaveAccessibleName("Save changes");
        expect(saveButton).toHaveClass("glass-accent");
        expect(saveButton).not.toHaveClass("glass-control", "text-muted");
        await user.click(saveButton);

        expect(saveEntry).toHaveBeenCalledWith({
            anilistId: existingEntry.anilistId,
            mediaKind: "ANIME",
            status: "COMPLETED",
            progress: 28,
            score: 10,
            notes: "A keeper.",
        });
        expect(saveButton).toBeDisabled();
        expect(saveButton).toHaveAccessibleName("Changes saved");
        expect(saveButton).toHaveClass("glass-control", "text-muted");
        expect(saveButton).not.toHaveClass("glass-accent", "opacity-55");
        expect(screen.queryByText("Changes saved.")).not.toBeInTheDocument();
    });

    it("keeps slider and star drafts visible when notes validation fails", async () => {
        const user = userEvent.setup();
        const saveEntry = vi.fn();
        const oversizedNotes = "n".repeat(10_001);

        render(LibraryEntryEditor, {
            anilistId: existingEntry.anilistId,
            knownTotal: null,
            entry: existingEntry,
            saveEntry,
        });

        const progress = screen.getByRole("slider", {
            name: "Episodes watched",
        });
        const notes = screen.getByLabelText(/private notes/i);
        await fireEvent.input(progress, { target: { value: "11" } });
        await user.click(
            screen.getByRole("button", {
                name: "Set personal score to 6 out of 10",
            }),
        );
        await fireEvent.input(notes, { target: { value: oversizedNotes } });
        await user.click(screen.getByRole("button", { name: "Save changes" }));

        expect(saveEntry).not.toHaveBeenCalled();
        expect(progress).toHaveValue("11");
        expect(screen.getByText("6 / 10")).toBeInTheDocument();
        expect(notes).toHaveValue(oversizedNotes);
        expect(notes).toHaveAttribute("aria-invalid", "true");
        expect(screen.getByText(/cannot exceed 10,000/i)).toBeInTheDocument();
        expect(screen.getByRole("alert")).toHaveTextContent(
            /your other edits are still here/i,
        );
    });

    it("bounds the episode slider and step controls by the known total", async () => {
        const user = userEvent.setup();
        const saveEntry = vi.fn();

        render(LibraryEntryEditor, {
            anilistId: existingEntry.anilistId,
            knownTotal: 12,
            entry: existingEntry,
            saveEntry,
        });

        const progress = screen.getByRole("slider", {
            name: "Episodes watched",
        });
        expect(progress).toHaveAttribute("max", "12");
        await fireEvent.input(progress, { target: { value: "12" } });
        expect(
            screen.getByRole("button", { name: "Increase episodes watched" }),
        ).toBeDisabled();
        await user.click(screen.getByRole("button", { name: "Save changes" }));

        expect(saveEntry).toHaveBeenCalledWith(
            expect.objectContaining({ progress: 12 }),
        );
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
        await fireEvent.input(
            screen.getByRole("slider", { name: "Episodes watched" }),
            {
                target: { value: "7" },
            },
        );
        await user.click(
            screen.getByRole("button", {
                name: "Set personal score to 7 out of 10",
            }),
        );
        await user.clear(screen.getByLabelText(/private notes/i));
        await user.type(
            screen.getByLabelText(/private notes/i),
            "Keep this draft.",
        );
        await user.click(screen.getByRole("button", { name: "Save changes" }));

        const alert = await screen.findByRole("alert");
        expect(alert).toHaveAttribute(
            "data-error-code",
            "DATABASE_UNAVAILABLE",
        );
        expect(alert).not.toHaveTextContent("DATABASE_UNAVAILABLE");
        expect(alert).toHaveTextContent(
            "The local library could not be saved.",
        );
        expect(screen.getByRole("radio", { name: /planning/i })).toBeChecked();
        expect(
            screen.getByRole("slider", { name: "Episodes watched" }),
        ).toHaveValue("7");
        expect(screen.getByText("7 / 10")).toBeInTheDocument();
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
            "This removes the title from your library.",
        );
        expect(confirmation).toHaveTextContent("Your watch history remains.");
        expect(
            screen.getByRole("button", { name: "Remove from library" }),
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
                name: "Remove from library",
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
        await user.type(
            screen.getByLabelText(/private notes/i),
            "Unsaved note",
        );
        await user.click(
            screen.getByRole("button", { name: "Remove from library" }),
        );
        await user.click(
            screen.getByRole("button", { name: "Remove from library" }),
        );

        const alert = await screen.findByRole("alert");
        expect(alert).toHaveAttribute(
            "data-error-code",
            "DATABASE_UNAVAILABLE",
        );
        expect(alert).toHaveTextContent("anime could not be removed");
        expect(screen.getByLabelText(/private notes/i)).toHaveValue(
            "Unsaved note",
        );
        expect(
            screen.getByRole("button", { name: "Remove from library" }),
        ).toBeInTheDocument();
    });
});
