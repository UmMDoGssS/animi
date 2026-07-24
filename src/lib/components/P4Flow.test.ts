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
    DashboardInsights,
    LibraryEntry,
    LibraryItem,
    LibraryMediaSnapshot,
    MediaDetailsResponse,
    MediaSummary,
    UpsertLibraryEntryRequest,
    WatchActivity,
} from "$lib/ipc/types";

import AppShell from "./AppShell.svelte";

const media: MediaSummary = {
    anilistId: 101,
    title: "Paper Cranes at Dawn",
    coverUrl: null,
    mediaFormat: "TV",
    status: "RELEASING",
    episodes: 12,
    durationMinutes: 24,
    releaseDate: null,
    genres: ["Drama", "Slice of Life"],
    tags: [{ name: "Found Family", rank: 88 }],
    trending: 90,
};

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

function createStatefulP4Services() {
    let localEntry: LibraryEntry | null = null;
    let activityHighWater = 0;
    let updateSequence = 0;
    let detailsOffline = false;
    const activity: WatchActivity[] = [];

    const freshness = () => ({
        fetchedAt: "2026-07-17T08:00:00.000Z",
        expiresAt: "2026-07-18T08:00:00.000Z",
        isStale: detailsOffline,
        refreshError: detailsOffline
            ? {
                  code: "ANILIST_OFFLINE",
                  message: "AniList is unavailable.",
                  retryable: true,
              }
            : null,
    });

    const mediaSnapshot = (): LibraryMediaSnapshot => ({
        title: media.title,
        coverUrl: media.coverUrl,
        mediaFormat: media.mediaFormat,
        genres: media.genres,
        tags: media.tags,
        status: media.status,
        episodes: media.episodes,
        durationMinutes: media.durationMinutes,
        releaseDate: media.releaseDate,
        cachedAt: freshness().fetchedAt,
        expiresAt: freshness().expiresAt,
        isStale: detailsOffline,
    });

    const libraryItems = (): LibraryItem[] =>
        localEntry === null
            ? []
            : [{ entry: { ...localEntry }, media: mediaSnapshot() }];

    const dashboard = (): DashboardInsights => {
        const item = localEntry;
        const isWatching = item?.status === "WATCHING";
        const isCompleted = item?.status === "COMPLETED";
        const isPlanning = item?.status === "PLANNING";
        const eligible = isWatching || isCompleted;
        return {
            mediaKind: "ANIME",
            libraryItems: libraryItems(),
            libraryTotals: {
                tracked: item === null ? 0 : 1,
                watching: isWatching ? 1 : 0,
                completed: isCompleted ? 1 : 0,
                planning: isPlanning ? 1 : 0,
                currentProgress: item?.progress ?? 0,
            },
            preferenceSignals: {
                eligibleTitles: eligible ? 1 : 0,
                metadataTitles: eligible ? 1 : 0,
                missingMetadataTitles: 0,
                staleMetadataTitles: eligible && detailsOffline ? 1 : 0,
                genres: eligible
                    ? media.genres.map((label) => ({ label, titleCount: 1 }))
                    : [],
                tags: eligible
                    ? media.tags.map((tag) => ({
                          label: tag.name,
                          titleCount: 1,
                          weightedRank: tag.rank,
                      }))
                    : [],
            },
            activity: {
                eventCount: activity.length,
                recordedEpisodes: activity.reduce(
                    (total, event) => total + event.progressDelta,
                    0,
                ),
                knownDurationEpisodes: activity.reduce(
                    (total, event) =>
                        total +
                        (event.episodeDurationMinutes === null
                            ? 0
                            : event.progressDelta),
                    0,
                ),
                recordedMinutes: activity.reduce(
                    (total, event) =>
                        total +
                        event.progressDelta *
                            (event.episodeDurationMinutes ?? 0),
                    0,
                ),
                unknownDurationEpisodes: activity.reduce(
                    (total, event) =>
                        total +
                        (event.episodeDurationMinutes === null
                            ? event.progressDelta
                            : 0),
                    0,
                ),
                coveredFrom: activity.at(-1)?.occurredAt ?? null,
                coveredTo: activity[0]?.occurredAt ?? null,
                monthly: [],
                yearly: [],
                events: activity.map((event) => ({ ...event })),
            },
        };
    };

    const saveEntry = vi.fn(
        async (request: UpsertLibraryEntryRequest): Promise<LibraryEntry> => {
            const previous = localEntry;
            if (request.progress > activityHighWater) {
                activity.unshift({
                    anilistId: request.anilistId,
                    mediaKind: "ANIME",
                    progressDelta: request.progress - activityHighWater,
                    episodeDurationMinutes: media.durationMinutes,
                    occurredAt:
                        previous === null
                            ? "2026-07-17T08:30:00.000Z"
                            : "2026-07-17T10:00:00.000Z",
                });
                activityHighWater = request.progress;
            }

            updateSequence += 1;
            localEntry = {
                ...request,
                createdAt: previous?.createdAt ?? "2026-07-17T08:30:00.000Z",
                updatedAt: `2026-07-17T0${8 + updateSequence}:00:00.000Z`,
            };
            return { ...localEntry };
        },
    );

    const deleteEntry = vi.fn(async () => {
        const removed = localEntry !== null;
        localEntry = null;
        return { removed };
    });

    const pageResponse = () => ({
        items: [media],
        hasNextPage: false,
        freshness: freshness(),
    });

    return {
        services: {
            loadTrending: vi.fn(async () => pageResponse()),
            searchAnime: vi.fn(async () => pageResponse()),
            loadLibrary: vi.fn(async () => libraryItems()),
            loadDetails: vi.fn(async (): Promise<MediaDetailsResponse> => {
                if (detailsOffline) {
                    throw {
                        code: "ANILIST_OFFLINE",
                        message: "AniList is unavailable.",
                        retryable: true,
                    };
                }
                return {
                    media: {
                        ...media,
                        episodesAired: null,
                        sequels: [],
                    },
                    freshness: freshness(),
                };
            }),
            saveEntry,
            deleteEntry,
            loadInsights: vi.fn(async () => dashboard()),
            refreshMetadata: vi.fn(async () => ({
                attempted: localEntry === null ? 0 : 1,
                updated: 0,
                stale: detailsOffline && localEntry !== null ? 1 : 0,
                failures: [],
            })),
        },
        goOffline() {
            detailsOffline = true;
        },
        activity,
        deleteEntry,
        saveEntry,
    };
}

describe("P4 discovery-to-library flow", () => {
    it("persists edits across a restart, records initial and later progress, works from cache offline, and confirms removal", async () => {
        const user = userEvent.setup();
        const state = createStatefulP4Services();
        const shellProps = {
            loadThemePreference: vi.fn().mockResolvedValue("SYSTEM" as const),
            saveThemePreference: vi.fn(),
            p4Services: state.services,
        };

        const firstRun = render(AppShell, shellProps);
        await user.click(screen.getByRole("tab", { name: "Discover" }));
        await user.type(
            screen.getByRole("searchbox", { name: "Anime title" }),
            "Paper Cranes{Enter}",
        );
        const searchResults = await screen.findByRole("list", {
            name: "Anime search results for Paper Cranes",
        });
        await user.click(
            within(searchResults).getByRole("button", {
                name: "Open details for Paper Cranes at Dawn",
            }),
        );

        let dialog = await screen.findByRole("dialog", {
            name: "Paper Cranes at Dawn",
        });
        await user.click(
            within(dialog).getByRole("radio", { name: /Watching/ }),
        );
        await fireEvent.input(
            within(dialog).getByRole("slider", { name: "Episodes watched" }),
            { target: { value: "3" } },
        );
        await user.click(
            within(dialog).getByRole("button", {
                name: "Set personal score to 8 out of 10",
            }),
        );
        await user.type(
            within(dialog).getByRole("textbox", { name: /Private notes/ }),
            "Quiet first watch",
        );
        await user.click(
            within(dialog).getByRole("button", { name: "Add to library" }),
        );
        expect(
            await within(dialog).findByText(/Added to your library/i),
        ).toBeInTheDocument();
        expect(state.activity).toHaveLength(1);
        expect(state.activity[0]?.progressDelta).toBe(3);
        await user.click(
            within(dialog).getByRole("button", { name: "Close details" }),
        );
        await waitFor(() => expect(dialog).not.toBeInTheDocument());

        firstRun.unmount();

        render(AppShell, shellProps);
        await user.click(screen.getByRole("tab", { name: "Dashboard" }));
        const libraryIndex = await screen.findByRole("list", {
            name: "Library index results",
        });
        expect(libraryIndex).toHaveTextContent(/Paper Cranes at Dawn/);
        expect(libraryIndex).toHaveTextContent(/3 \/ 12 eps/);
        expect(
            screen.getByRole("group", { name: "Recorded activity summary" }),
        ).toHaveTextContent(/recorded episodes\s*3/i);
        expect(
            screen.getByRole("list", {
                name: "Recent recorded progress events",
            }),
        ).toHaveTextContent(/\+3 episodes.*1h 12m/i);

        await user.click(screen.getByRole("tab", { name: "Discover" }));
        const trending = await screen.findByRole("list", {
            name: "Trending anime",
        });
        await user.click(
            within(trending).getByRole("button", {
                name: "Open details for Paper Cranes at Dawn",
            }),
        );
        dialog = await screen.findByRole("dialog", {
            name: "Paper Cranes at Dawn",
        });
        await user.click(
            within(dialog).getByRole("radio", { name: /Completed/ }),
        );
        const progress = within(dialog).getByRole("slider", {
            name: "Episodes watched",
        });
        await fireEvent.input(progress, { target: { value: "5" } });
        await user.click(
            within(dialog).getByRole("button", {
                name: "Set personal score to 9 out of 10",
            }),
        );
        const notes = within(dialog).getByRole("textbox", {
            name: /Private notes/,
        });
        await user.clear(notes);
        await user.type(notes, "Finished the local edit pass");
        await user.click(
            within(dialog).getByRole("button", { name: "Save changes" }),
        );
        await waitFor(() => expect(state.activity).toHaveLength(2));
        expect(state.activity[0]?.progressDelta).toBe(2);
        expect(state.activity[1]?.progressDelta).toBe(3);
        await user.click(
            within(dialog).getByRole("button", { name: "Close details" }),
        );

        state.goOffline();
        await user.click(screen.getByRole("tab", { name: "Dashboard" }));
        const updatedLibraryIndex = await screen.findByRole("list", {
            name: "Library index results",
        });
        expect(updatedLibraryIndex).toHaveTextContent(/5 \/ 12 eps/);
        expect(updatedLibraryIndex).toHaveTextContent(/score \/\/ 9 \/ 10/i);
        expect(
            screen.getByRole("group", { name: "Recorded activity summary" }),
        ).toHaveTextContent(/recorded episodes\s*5/i);
        expect(
            screen.getByRole("list", {
                name: "Recent recorded progress events",
            }),
        ).toHaveTextContent(/\+2 episodes.*48m/i);

        await user.click(
            within(updatedLibraryIndex).getByRole("button", {
                name: "Open library details for Paper Cranes at Dawn",
            }),
        );
        dialog = await screen.findByRole("dialog", {
            name: "Paper Cranes at Dawn",
        });
        expect(
            await within(dialog).findByText("Showing available details"),
        ).toBeInTheDocument();
        expect(within(dialog).getByText("12 episodes")).toBeInTheDocument();

        await user.click(
            within(dialog).getByRole("button", { name: "Remove from library" }),
        );
        expect(state.deleteEntry).not.toHaveBeenCalled();
        expect(
            within(dialog).getByText(/Your watch history remains/i),
        ).toBeInTheDocument();
        await user.click(
            within(dialog).getByRole("button", { name: "Remove from library" }),
        );
        await waitFor(() => expect(state.deleteEntry).toHaveBeenCalledOnce());
        await user.click(
            within(dialog).getByRole("button", { name: "Close details" }),
        );

        expect(
            await screen.findByRole("heading", {
                name: "Your library has no anime yet.",
            }),
        ).toBeInTheDocument();
        expect(
            screen.getByRole("group", { name: "Recorded activity summary" }),
        ).toHaveTextContent(/updates\s*2/i);
    });
});
