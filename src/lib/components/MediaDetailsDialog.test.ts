import {
    fireEvent,
    render,
    screen,
    waitFor,
    within,
} from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

import type {
    CommandError,
    Freshness,
    LibraryEntry,
    MediaDetailsResponse,
    MediaSummary,
    UpsertLibraryEntryRequest,
} from "$lib/ipc/types";

import MediaDetailsDialog from "./MediaDetailsDialog.svelte";

const summary: MediaSummary = {
    anilistId: 101,
    title: "Paper Cranes at Dawn",
    coverUrl: null,
    mediaFormat: "TV",
    status: "RELEASING",
    episodes: 12,
    durationMinutes: 24,
    releaseDate: null,
    genres: ["Drama"],
    tags: [{ name: "Found Family", rank: 90 }],
    trending: 44,
};

const currentFreshness: Freshness = {
    fetchedAt: "2026-07-17T09:00:00.000Z",
    expiresAt: "2026-07-18T09:00:00.000Z",
    isStale: false,
    refreshError: null,
};

const staleFreshness: Freshness = {
    fetchedAt: "2026-07-15T09:00:00.000Z",
    expiresAt: "2026-07-16T09:00:00.000Z",
    isStale: true,
    refreshError: null,
};

const freshDetails: MediaDetailsResponse = {
    media: {
        ...summary,
        title: "Paper Cranes, Renewed",
        episodes: 13,
        durationMinutes: 25,
        releaseDate: "2026-04-03",
        genres: ["Drama", "Fantasy"],
        tags: [{ name: "Reunion", rank: 94 }],
        episodesAired: null,
        sequels: [],
    },
    freshness: currentFreshness,
};

const existingEntry: LibraryEntry = {
    anilistId: summary.anilistId,
    mediaKind: "ANIME",
    status: "WATCHING",
    progress: 3,
    score: 8.5,
    notes: "Waiting for the next episode.",
    createdAt: "2026-07-16T08:00:00.000Z",
    updatedAt: "2026-07-17T08:00:00.000Z",
};

function detailsResponse(mediaSummary = summary): MediaDetailsResponse {
    return {
        media: { ...mediaSummary, episodesAired: null, sequels: [] },
        freshness: currentFreshness,
    };
}

function savedEntry(request: UpsertLibraryEntryRequest): LibraryEntry {
    return {
        anilistId: request.anilistId,
        mediaKind: request.mediaKind,
        status: request.status,
        progress: request.progress,
        score: request.score,
        notes: request.notes,
        createdAt: "2026-07-17T08:00:00.000Z",
        updatedAt: "2026-07-17T10:00:00.000Z",
    };
}

const originalShowModal = Object.getOwnPropertyDescriptor(
    HTMLDialogElement.prototype,
    "showModal",
);
const originalClose = Object.getOwnPropertyDescriptor(
    HTMLDialogElement.prototype,
    "close",
);

beforeAll(() => {
    Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
        configurable: true,
        value(this: HTMLDialogElement) {
            this.setAttribute("open", "");
        },
    });
    Object.defineProperty(HTMLDialogElement.prototype, "close", {
        configurable: true,
        value(this: HTMLDialogElement) {
            this.removeAttribute("open");
            this.dispatchEvent(new Event("close"));
        },
    });
});

afterAll(() => {
    if (originalShowModal) {
        Object.defineProperty(
            HTMLDialogElement.prototype,
            "showModal",
            originalShowModal,
        );
    } else {
        Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
    }

    if (originalClose) {
        Object.defineProperty(
            HTMLDialogElement.prototype,
            "close",
            originalClose,
        );
    } else {
        Reflect.deleteProperty(HTMLDialogElement.prototype, "close");
    }
});

describe("MediaDetailsDialog", () => {
    it("keeps a stale fallback visible after an offline failure and replaces it on retry", async () => {
        const user = userEvent.setup();
        const offlineError: CommandError = {
            code: "ANILIST_OFFLINE",
            message: "AniList is unavailable.",
            retryable: true,
        };
        const loadDetails = vi
            .fn()
            .mockRejectedValueOnce(offlineError)
            .mockResolvedValueOnce(freshDetails);
        const onDetailsLoaded = vi.fn();

        render(MediaDetailsDialog, {
            summary,
            entry: null,
            fallbackFreshness: staleFreshness,
            loadDetails,
            saveEntry: vi.fn(),
            deleteEntry: vi.fn(),
            onDetailsLoaded,
            onClose: vi.fn(),
        });

        expect(
            await screen.findByRole("dialog", { name: summary.title }),
        ).toHaveAttribute("open");
        const fallbackNotice = (
            await screen.findByText("Showing available details")
        ).closest("[data-status]");
        expect(fallbackNotice).toHaveAttribute("data-status", "offline");
        expect(
            screen.getByRole("heading", { name: summary.title }),
        ).toBeVisible();
        expect(screen.getByText("may be out of date")).toBeInTheDocument();
        expect(screen.getByText("12 episodes")).toBeInTheDocument();
        expect(screen.getByText("Found Family")).toBeInTheDocument();

        await user.click(screen.getByRole("button", { name: "Retry details" }));

        expect(
            await screen.findByRole("dialog", {
                name: freshDetails.media.title,
            }),
        ).toBeInTheDocument();
        expect(
            screen.queryByText("may be out of date"),
        ).not.toBeInTheDocument();
        expect(screen.getByText("13 episodes")).toBeInTheDocument();
        expect(screen.getByText("Apr 3, 2026")).toBeInTheDocument();
        expect(screen.getByText("Apr 3, 2026").closest("time")).toHaveAttribute(
            "datetime",
            "2026-04-03",
        );
        expect(screen.getByText("Reunion")).toBeInTheDocument();
        expect(
            screen.queryByText("Showing available details"),
        ).not.toBeInTheDocument();
        expect(loadDetails).toHaveBeenCalledTimes(2);
        expect(loadDetails).toHaveBeenNthCalledWith(1, { anilistId: 101 });
        expect(loadDetails).toHaveBeenNthCalledWith(2, { anilistId: 101 });
        expect(onDetailsLoaded).toHaveBeenCalledOnce();
        expect(onDetailsLoaded).toHaveBeenCalledWith(freshDetails);
    });

    it("renders explicit unknown and empty metadata states", async () => {
        const onDetailsLoaded = vi.fn();
        const unknownDetails: MediaDetailsResponse = {
            media: {
                ...summary,
                title: "The Unscheduled Broadcast",
                episodes: null,
                episodesAired: 6,
                durationMinutes: null,
                genres: [],
                tags: [],
                sequels: [],
            },
            freshness: currentFreshness,
        };

        render(MediaDetailsDialog, {
            summary,
            entry: null,
            loadDetails: vi.fn().mockResolvedValue(unknownDetails),
            saveEntry: vi.fn(),
            deleteEntry: vi.fn(),
            onDetailsLoaded,
            onClose: vi.fn(),
        });

        await screen.findByRole("dialog", { name: unknownDetails.media.title });
        expect(screen.getByText("6 episodes aired")).toBeInTheDocument();
        expect(screen.getByText("Duration unknown")).toBeInTheDocument();
        expect(screen.getByText("release date")).toBeInTheDocument();
        expect(screen.getByText("Unknown")).toBeInTheDocument();
        const metadataList = screen.getByText("episodes").closest("dl");
        expect(metadataList).toHaveClass("grid", "divide-y", "divide-border");
        expect(metadataList).not.toHaveClass("border-t");
        expect(metadataList).not.toHaveClass("border-y");
        expect(metadataList?.children).toHaveLength(3);
        for (const label of ["episodes", "duration", "release date"]) {
            expect(screen.getByText(label).closest("div")).toHaveClass(
                "flex",
                "justify-between",
            );
        }
        expect(screen.getByText("Genres are unavailable.")).toBeInTheDocument();
        expect(
            screen.getByText("Ranked tags are unavailable."),
        ).toBeInTheDocument();
        expect(
            screen.getByText("No official sequel relation is available."),
        ).toBeInTheDocument();
        expect(onDetailsLoaded).toHaveBeenCalledOnce();
        expect(onDetailsLoaded).toHaveBeenCalledWith(unknownDetails);
    });

    it("renders genres and tags as compact wrapping badges", async () => {
        const badgeDetails: MediaDetailsResponse = {
            media: {
                ...summary,
                genres: ["Drama", "Fantasy"],
                tags: [
                    { name: "Found Family", rank: 90 },
                    { name: "Slow Burn", rank: 84 },
                ],
                episodesAired: null,
                sequels: [],
            },
            freshness: currentFreshness,
        };

        render(MediaDetailsDialog, {
            summary,
            entry: null,
            loadDetails: vi.fn().mockResolvedValue(badgeDetails),
            saveEntry: vi.fn(),
            deleteEntry: vi.fn(),
            onClose: vi.fn(),
        });

        await screen.findByText("Fantasy");

        for (const label of ["Drama", "Fantasy", "Found Family", "Slow Burn"]) {
            const badge = screen.getByText(label).closest("li");
            expect(badge).toHaveClass("px-2", "py-1", "text-xs");
            expect(badge?.parentElement).toHaveClass("flex", "flex-wrap");
        }
    });

    it("shows only official sequel dates and labels incomplete dates as unknown", async () => {
        const sequelDetails: MediaDetailsResponse = {
            media: {
                ...summary,
                episodesAired: null,
                sequels: [
                    {
                        anilistId: 201,
                        title: "Paper Cranes: Second Flight",
                        releaseDate: "2027-04-03",
                    },
                    {
                        anilistId: 202,
                        title: "Paper Cranes: Autumn Return",
                        releaseDate: "2027-10",
                    },
                    {
                        anilistId: 203,
                        title: "Paper Cranes: Someday",
                        releaseDate: "2028",
                    },
                    {
                        anilistId: 204,
                        title: "Paper Cranes: After Story",
                        releaseDate: null,
                    },
                ],
            },
            freshness: currentFreshness,
        };
        render(MediaDetailsDialog, {
            summary,
            entry: null,
            loadDetails: vi.fn().mockResolvedValue(sequelDetails),
            saveEntry: vi.fn(),
            deleteEntry: vi.fn(),
            onClose: vi.fn(),
        });

        await screen.findByRole("dialog", { name: summary.title });
        expect(
            await screen.findByText("Paper Cranes: Second Flight"),
        ).toBeInTheDocument();
        expect(screen.getByText(/Releases Apr 3, 2027/i)).toBeInTheDocument();
        expect(screen.getByText(/Releases Oct 2027/i)).toBeInTheDocument();
        expect(screen.getByText(/Releases 2028/i)).toBeInTheDocument();
        expect(
            screen.getByText("Paper Cranes: After Story"),
        ).toBeInTheDocument();
        expect(screen.getByText("Release date unknown")).toBeInTheDocument();
        expect(
            screen.queryByText(/estimated|predicted/i),
        ).not.toBeInTheDocument();
    });

    it("synchronizes an entry that arrives after a blocked library read", async () => {
        const loadingError: CommandError = {
            code: "LIBRARY_LOADING",
            message: "Reading the local library.",
            retryable: true,
        };
        const loadDetails = vi.fn().mockResolvedValue(detailsResponse());
        const saveEntry = vi.fn();
        const deleteEntry = vi.fn();
        const onClose = vi.fn();
        const { rerender } = render(MediaDetailsDialog, {
            summary,
            entry: null,
            trackingError: loadingError,
            loadDetails,
            saveEntry,
            deleteEntry,
            onClose,
        });

        expect(
            await screen.findByText("Loading your library entry"),
        ).toBeInTheDocument();
        expect(
            screen.queryByRole("button", { name: "Add to library" }),
        ).not.toBeInTheDocument();

        await rerender({
            summary,
            entry: existingEntry,
            trackingError: null,
            loadDetails,
            saveEntry,
            deleteEntry,
            onClose,
        });

        expect(
            await screen.findByRole("heading", { name: "Edit library entry" }),
        ).toBeInTheDocument();
        expect(screen.queryByText("Library")).not.toBeInTheDocument();
        expect(
            screen.getByRole("slider", { name: "Episodes watched" }),
        ).toHaveValue("3");
        expect(screen.getByText("8.5 / 10")).toBeInTheDocument();
        expect(screen.getByLabelText(/private notes/i)).toHaveValue(
            existingEntry.notes,
        );
        expect(screen.getByRole("radio", { name: /watching/i })).toBeChecked();
    });

    it("forwards saved and removed entries while keeping editor state in sync", async () => {
        const user = userEvent.setup();
        const saveEntry = vi.fn(async (request: UpsertLibraryEntryRequest) =>
            savedEntry(request),
        );
        const deleteEntry = vi.fn().mockResolvedValue({ removed: true });
        const onLibraryChanged = vi.fn();

        render(MediaDetailsDialog, {
            summary,
            entry: null,
            loadDetails: vi.fn().mockResolvedValue(detailsResponse()),
            saveEntry,
            deleteEntry,
            onLibraryChanged,
            onClose: vi.fn(),
        });

        const dialog = await screen.findByRole("dialog", {
            name: summary.title,
        });
        await user.click(
            within(dialog).getByRole("button", { name: "Add to library" }),
        );

        const saved = savedEntry({
            anilistId: summary.anilistId,
            mediaKind: "ANIME",
            status: "PLANNING",
            progress: 0,
            score: null,
            notes: "",
        });
        expect(
            await within(dialog).findByRole("heading", {
                name: "Edit library entry",
            }),
        ).toBeInTheDocument();
        expect(onLibraryChanged).toHaveBeenNthCalledWith(1, saved);
        expect(within(dialog).getByText("Added to your library.")).toHaveClass(
            "mb-4",
        );

        await user.type(
            within(dialog).getByLabelText(/private notes/i),
            "Watch again.",
        );
        const saveButton = within(dialog).getByRole("button", {
            name: "Save changes",
        });
        expect(saveButton).toBeEnabled();
        await user.click(saveButton);

        expect(saveButton).toBeDisabled();
        expect(saveButton).toHaveAccessibleName("Changes saved");
        expect(saveButton).toHaveClass("glass-control", "text-muted");
        expect(saveButton).not.toHaveClass("glass-accent", "opacity-55");
        expect(
            within(dialog).queryByText("Changes saved."),
        ).not.toBeInTheDocument();
        expect(onLibraryChanged).toHaveBeenNthCalledWith(
            2,
            expect.objectContaining({ notes: "Watch again." }),
        );

        await user.click(
            within(dialog).getByRole("button", { name: "Remove from library" }),
        );
        await user.click(
            within(dialog).getByRole("button", { name: "Remove from library" }),
        );

        expect(
            await within(dialog).findByRole("heading", {
                name: "Add to your library",
            }),
        ).toBeInTheDocument();
        expect(deleteEntry).toHaveBeenCalledWith({
            anilistId: summary.anilistId,
            mediaKind: "ANIME",
            confirmed: true,
        });
        expect(onLibraryChanged).toHaveBeenNthCalledWith(3, null);
        expect(
            within(dialog).getByText(
                "Removed from your library. Watch history was kept.",
            ),
        ).toHaveClass("mb-4");
    });

    it("closes through the close control and removes modal state", async () => {
        const user = userEvent.setup();
        const onClose = vi.fn();
        render(MediaDetailsDialog, {
            summary,
            entry: null,
            loadDetails: vi.fn().mockResolvedValue(detailsResponse()),
            saveEntry: vi.fn(),
            deleteEntry: vi.fn(),
            onClose,
        });

        const dialog = await screen.findByRole("dialog", {
            name: summary.title,
        });
        const closeButton = within(dialog).getByRole("button", {
            name: "Close details",
        });
        await waitFor(() => expect(closeButton).toHaveFocus());
        expect(document.documentElement).toHaveClass("has-modal");

        await user.click(closeButton);

        expect(dialog).not.toHaveAttribute("open");
        expect(document.documentElement).not.toHaveClass("has-modal");
        expect(onClose).toHaveBeenCalledOnce();
    });

    it("handles native cancel once and cleans up modal state", async () => {
        const onClose = vi.fn();
        render(MediaDetailsDialog, {
            summary,
            entry: null,
            loadDetails: vi.fn().mockResolvedValue(detailsResponse()),
            saveEntry: vi.fn(),
            deleteEntry: vi.fn(),
            onClose,
        });

        const dialog = await screen.findByRole("dialog", {
            name: summary.title,
        });
        expect(document.documentElement).toHaveClass("has-modal");
        const cancelEvent = new Event("cancel", { cancelable: true });

        await fireEvent(dialog, cancelEvent);

        expect(cancelEvent.defaultPrevented).toBe(true);
        expect(dialog).not.toHaveAttribute("open");
        expect(document.documentElement).not.toHaveClass("has-modal");
        expect(onClose).toHaveBeenCalledOnce();

        await fireEvent(dialog, new Event("close"));
        expect(onClose).toHaveBeenCalledOnce();
    });
});
