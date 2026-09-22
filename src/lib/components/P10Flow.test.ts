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
    MediaKind,
    MediaSummary,
    UpsertLibraryEntryRequest,
} from "$lib/ipc/types";
import AppShell from "./AppShell.svelte";

const manga: MediaSummary = {
    mediaKind: "MANGA",
    anilistId: 55,
    title: "The Last Garden",
    coverUrl: null,
    mediaFormat: "NOVEL",
    status: "RELEASING",
    episodes: null,
    chapters: null,
    volumes: 8,
    durationMinutes: null,
    releaseDate: "2024-02",
    genres: ["Fantasy"],
    tags: [{ name: "Gardening", rank: 90 }],
    trending: 12,
};
const anime: LibraryEntry = {
    mediaKind: "ANIME",
    anilistId: 12,
    status: "WATCHING",
    progress: 3,
    volumeProgress: null,
    score: null,
    notes: "Keep this",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
};

function services() {
    let entry: LibraryEntry | null = null;
    let offline = false;
    let sequence = 0;
    const freshness = () => ({
        fetchedAt: "2026-09-10T00:00:00Z",
        expiresAt: "2026-09-11T00:00:00Z",
        isStale: offline,
        refreshError: offline
            ? { code: "ANILIST_OFFLINE", message: "Offline", retryable: true }
            : null,
    });
    const items = () =>
        entry
            ? [
                  {
                      entry,
                      media: {
                          ...manga,
                          cachedAt: freshness().fetchedAt,
                          expiresAt: freshness().expiresAt,
                          isStale: offline,
                      },
                  },
              ]
            : [];
    const insights = (kind: MediaKind): DashboardInsights => ({
        mediaKind: kind,
        libraryItems:
            kind === "MANGA" ? items() : [{ entry: anime, media: null }],
        libraryTotals: {
            tracked: kind === "MANGA" ? Number(entry !== null) : 1,
            watching: entry?.status === "WATCHING" ? 1 : 0,
            completed: 0,
            planning: 0,
            currentProgress: kind === "MANGA" ? (entry?.progress ?? 0) : 3,
            currentVolumeProgress:
                kind === "MANGA" ? (entry?.volumeProgress ?? 0) : 0,
        },
        preferenceSignals: {
            eligibleTitles: 0,
            metadataTitles: 0,
            missingMetadataTitles: 0,
            staleMetadataTitles: 0,
            genres: [],
            tags: [],
        },
        activity: {
            eventCount: 0,
            recordedEpisodes: 0,
            knownDurationEpisodes: 0,
            recordedMinutes: 0,
            unknownDurationEpisodes: 0,
            recordedChapters: 0,
            recordedVolumes: 0,
            coveredFrom: null,
            coveredTo: null,
            monthly: [],
            yearly: [],
            events: [],
        },
    });
    return {
        goOffline: () => (offline = true),
        services: {
            loadTrending: vi.fn(
                async ({ mediaKind }: { mediaKind: MediaKind }) => ({
                    items: mediaKind === "MANGA" ? [manga] : [],
                    hasNextPage: false,
                    freshness: freshness(),
                }),
            ),
            searchAnime: vi.fn(async () => ({
                items: [manga],
                hasNextPage: false,
                freshness: freshness(),
            })),
            loadLibrary: vi.fn(async () => [
                ...items(),
                { entry: anime, media: null },
            ]),
            loadDetails: vi.fn(async () => ({
                media: { ...manga, episodesAired: null, sequels: [] },
                freshness: freshness(),
            })),
            loadInsights: vi.fn(
                async ({ mediaKind }: { mediaKind: MediaKind }) =>
                    insights(mediaKind),
            ),
            refreshMetadata: vi.fn(async () => ({
                attempted: 1,
                updated: offline ? 0 : 1,
                stale: offline ? 1 : 0,
                failures: [],
            })),
            saveEntry: vi.fn(async (request: UpsertLibraryEntryRequest) => {
                entry = {
                    ...request,
                    createdAt: entry?.createdAt ?? "2026-09-10T00:00:00Z",
                    updatedAt: `2026-09-10T00:00:0${++sequence}Z`,
                };
                return entry;
            }),
            deleteEntry: vi.fn(async () => {
                entry = null;
                return { removed: true };
            }),
        },
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

describe("P10 media-aware shell", () => {
    // This multi-page integration flow includes remounts and many real user events.
    it(
        "adds, corrects, reloads, refreshes and removes manga while preserving anime",
        { timeout: 15_000 },
        async () => {
            const user = userEvent.setup();
            const state = services();
            const props = {
                loadThemePreference: async () => "SYSTEM" as const,
                p4Services: state.services,
            };
            const first = render(AppShell, props);
            expect(screen.getByRole("radio", { name: "Anime" })).toBeChecked();
            await user.click(screen.getByRole("tab", { name: "Discover" }));
            await user.click(screen.getByRole("radio", { name: "Manga" }));
            await user.type(
                screen.getByRole("searchbox", { name: "Manga title" }),
                "Garden{Enter}",
            );
            const results = await screen.findByRole("list", {
                name: "Manga search results for Garden",
            });
            expect(state.services.searchAnime).toHaveBeenCalledWith(
                expect.objectContaining({ mediaKind: "MANGA" }),
            );
            await user.click(
                within(results).getByRole("button", {
                    name: "Open details for The Last Garden",
                }),
            );
            let dialog = await screen.findByRole("dialog");
            expect(
                within(dialog).getByText("Manga details"),
            ).toBeInTheDocument();
            expect(
                within(dialog).queryByText("duration", { exact: true }),
            ).not.toBeInTheDocument();
            expect(within(dialog).getByText(/novel/)).toBeInTheDocument();
            await user.click(
                within(dialog).getByRole("radio", { name: /Reading/ }),
            );
            await fireEvent.input(
                within(dialog).getByRole("slider", { name: "Chapters read" }),
                { target: { value: "42" } },
            );
            const volumes = within(dialog).getByRole("spinbutton", {
                name: "Volumes read",
            });
            await user.clear(volumes);
            await user.type(volumes, "6");
            await user.click(
                within(dialog).getByRole("button", { name: "Add to library" }),
            );
            await within(dialog).findByRole("button", {
                name: "Changes saved",
            });
            const correctedVolumes = within(dialog).getByRole("spinbutton", {
                name: "Volumes read",
            });
            await user.clear(correctedVolumes);
            await user.type(correctedVolumes, "5");
            await user.click(
                within(dialog).getByRole("button", { name: "Save changes" }),
            );
            expect(state.services.saveEntry).toHaveBeenLastCalledWith(
                expect.objectContaining({
                    mediaKind: "MANGA",
                    progress: 42,
                    volumeProgress: 5,
                }),
            );
            await user.click(
                within(dialog).getByRole("button", { name: "Close details" }),
            );
            first.unmount();

            render(AppShell, props);
            expect(screen.getByRole("radio", { name: "Anime" })).toBeChecked();
            await user.click(screen.getByRole("tab", { name: "Dashboard" }));
            await user.click(screen.getByRole("radio", { name: "Manga" }));
            expect(
                screen.queryByRole("region", { name: "Your library" }),
            ).not.toBeInTheDocument();
            expect(
                screen.queryByRole("list", { name: "Library index results" }),
            ).not.toBeInTheDocument();
            expect(
                screen.queryByText("known watch time"),
            ).not.toBeInTheDocument();
            await user.click(
                screen.getByRole("button", { name: "refresh manga details" }),
            );
            await waitFor(() =>
                expect(state.services.refreshMetadata).toHaveBeenCalledWith({
                    mediaKind: "MANGA",
                }),
            );
            state.goOffline();
            await user.click(screen.getByRole("tab", { name: "Home" }));
            const library = await screen.findByRole("list", {
                name: "Library index results",
            });
            expect(library).toHaveTextContent("42 / ?? chapters");
            expect(library).toHaveTextContent("5 / 8 volumes");
            expect(library).not.toHaveTextContent("AniList #12");
            await user.click(
                within(library).getByRole("button", {
                    name: "Open library details for The Last Garden",
                }),
            );
            dialog = await screen.findByRole("dialog");
            expect(
                await within(dialog).findByText("may be out of date"),
            ).toBeInTheDocument();
            await user.click(
                within(dialog).getByRole("button", {
                    name: "Remove from library",
                }),
            );
            expect(state.services.deleteEntry).not.toHaveBeenCalled();
            await user.click(
                within(dialog).getByRole("button", {
                    name: "Remove from library",
                }),
            );
            expect(state.services.deleteEntry).toHaveBeenCalledWith({
                anilistId: 55,
                mediaKind: "MANGA",
                confirmed: true,
            });
            await user.click(
                within(dialog).getByRole("button", { name: "Close details" }),
            );
            await user.click(screen.getByRole("radio", { name: "Anime" }));
            expect(
                await screen.findByRole("list", {
                    name: "Library index results",
                }),
            ).toHaveTextContent("3 / ?? eps");
            expect(anime.notes).toBe("Keep this");
        },
    );

    it("resets search and ignores old responses when switching with the keyboard", async () => {
        const user = userEvent.setup();
        const state = services();
        let finish:
            | ((value: {
                  items: MediaSummary[];
                  hasNextPage: boolean;
                  freshness: {
                      fetchedAt: string;
                      expiresAt: string;
                      isStale: boolean;
                      refreshError: null;
                  };
              }) => void)
            | undefined;
        const pending = new Promise<{
            items: MediaSummary[];
            hasNextPage: boolean;
            freshness: {
                fetchedAt: string;
                expiresAt: string;
                isStale: boolean;
                refreshError: null;
            };
        }>((resolve) => (finish = resolve));
        state.services.searchAnime.mockImplementationOnce(() => pending);
        render(AppShell, {
            loadThemePreference: async () => "SYSTEM",
            p4Services: state.services,
        });
        await user.click(screen.getByRole("tab", { name: "Discover" }));
        await user.type(
            screen.getByRole("searchbox", { name: "Anime title" }),
            "Old search{Enter}",
        );
        await user.click(screen.getByRole("radio", { name: "Anime" }));
        await user.keyboard("{ArrowRight}");
        expect(screen.getByRole("radio", { name: "Manga" })).toBeChecked();
        expect(screen.getByRole("radio", { name: "Manga" })).toHaveFocus();
        expect(
            screen.getByRole("searchbox", { name: "Manga title" }),
        ).toHaveValue("");
        finish?.({
            items: [
                { ...manga, title: "Obsolete response", mediaKind: "ANIME" },
            ],
            hasNextPage: false,
            freshness: {
                fetchedAt: "2026-09-10T00:00:00Z",
                expiresAt: "2026-09-11T00:00:00Z",
                isStale: false,
                refreshError: null,
            },
        });
        await screen.findByRole("list", { name: "Trending manga" });
        expect(screen.queryByText("Obsolete response")).not.toBeInTheDocument();
        expect(screen.queryByText("Search results")).not.toBeInTheDocument();
    });
});
