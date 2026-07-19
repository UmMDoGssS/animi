import {
    fireEvent,
    render,
    screen,
    waitFor,
    within,
} from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type {
    DashboardInsights,
    LibraryEntry,
    MediaSummary,
    ThemePreference,
    UpsertLibraryEntryRequest,
} from "$lib/ipc/types";

import { setTestViewport, testViewports } from "../../test/viewports";

import AppShell from "./AppShell.svelte";

const appShellMedia: MediaSummary = {
    anilistId: 101,
    title: "Paper Cranes at Dawn",
    coverUrl: null,
    mediaFormat: "TV",
    status: "RELEASING",
    episodes: 12,
    durationMinutes: 24,
    releaseDate: null,
    genres: ["Drama"],
    tags: [{ name: "Found Family", rank: 88 }],
    trending: 90,
};

const emptyDashboard: DashboardInsights = {
    mediaKind: "ANIME",
    libraryItems: [],
    libraryTotals: {
        tracked: 0,
        watching: 0,
        completed: 0,
        planning: 0,
        currentProgress: 0,
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
        coveredFrom: null,
        coveredTo: null,
        monthly: [],
        yearly: [],
        events: [],
    },
};

function p4Services() {
    return {
        loadTrending: vi.fn().mockResolvedValue({
            items: [appShellMedia],
            hasNextPage: false,
            freshness: {
                fetchedAt: "2026-07-17T08:00:00.000Z",
                expiresAt: "2026-07-17T14:00:00.000Z",
                isStale: false,
                refreshError: null,
            },
        }),
        searchAnime: vi.fn(),
        loadLibrary: vi.fn().mockResolvedValue([]),
        loadDetails: vi.fn().mockResolvedValue({
            media: { ...appShellMedia, sequels: [] },
            freshness: {
                fetchedAt: "2026-07-17T08:00:00.000Z",
                expiresAt: "2026-07-18T08:00:00.000Z",
                isStale: false,
                refreshError: null,
            },
        }),
        saveEntry: vi.fn(async (request: UpsertLibraryEntryRequest) => ({
            ...request,
            createdAt: "2026-07-17T08:00:00.000Z",
            updatedAt: "2026-07-17T08:00:00.000Z",
        })) as (request: UpsertLibraryEntryRequest) => Promise<LibraryEntry>,
        deleteEntry: vi.fn().mockResolvedValue({ removed: true }),
        loadInsights: vi.fn().mockResolvedValue(emptyDashboard),
        refreshMetadata: vi.fn(),
    };
}

function p6Services() {
    return {
        saveBackup: vi.fn().mockResolvedValue(null),
        chooseImport: vi.fn().mockResolvedValue(null),
        restoreBackup: vi.fn(),
        clearData: vi.fn(),
    };
}

describe("AppShell", () => {
    it.each(testViewports)(
        "renders all four page destinations at the $name viewport",
        async (viewport) => {
            setTestViewport(viewport);
            render(AppShell, {
                loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
                saveThemePreference: vi.fn(),
                p4Services: p4Services(),
                p6Services: p6Services(),
            });

            const tabs = screen.getAllByRole("tab");
            expect(tabs).toHaveLength(4);
            expect(
                screen.getByRole("tablist", { name: "Primary pages" }),
            ).toHaveAttribute("aria-orientation", "horizontal");
            expect(screen.getByRole("tab", { name: "Home" })).toHaveAttribute(
                "aria-selected",
                "true",
            );
            expect(
                screen.getByRole("tab", { name: "Home" }),
            ).not.toHaveAttribute("title");
            expect(
                within(screen.getByRole("tab", { name: "Home" })).getByText(
                    "Home",
                ),
            ).toHaveAttribute("aria-hidden", "true");
            expect(
                screen.queryByText("Private & local"),
            ).not.toBeInTheDocument();
            await waitFor(() =>
                expect(
                    document.querySelector('pre[aria-hidden="true"]'),
                ).not.toBeInTheDocument(),
            );
            expect(
                within(screen.getByRole("tab", { name: "Home" })).getByText(
                    "Home",
                ),
            ).toHaveClass("top-0.5");
            expect(
                screen.getByRole("heading", { level: 1, name: "Home" }),
            ).toBeInTheDocument();
            expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(
                1,
            );
            expect(screen.getByRole("tabpanel")).toHaveAttribute(
                "id",
                "home-panel",
            );
            const appChrome = document.querySelector(".app-chrome");
            expect(appChrome).toHaveAttribute("data-page", "home");
            expect(document.querySelector(".rainbow-band")).toBeInTheDocument();
            expect(
                screen.getByRole("link", { name: "Skip to content" }),
            ).toHaveAttribute("href", "#main-content");
            expect(screen.getByRole("main")).toHaveAttribute("tabindex", "-1");
            expect(
                screen.getByRole("banner", { name: "Application header" }),
            ).toHaveClass("justify-between");
            expect(
                screen.getByRole("banner", { name: "Application header" }),
            ).not.toHaveClass("sticky", "fixed");
            expect(
                screen.queryByRole("complementary", {
                    name: "Primary navigation rail",
                }),
            ).not.toBeInTheDocument();
            expect(
                screen.queryByText(
                    "Keep the week in view, catch the episodes that slipped past, and log one more without leaving your queue.",
                ),
            ).not.toBeInTheDocument();

            for (const [destination, pageId] of [
                ["Discover", "discovery"],
                ["Dashboard", "dashboard"],
                ["Settings", "settings"],
            ] as const) {
                await fireEvent.click(
                    screen.getByRole("tab", { name: destination }),
                );
                expect(appChrome).toHaveAttribute("data-page", pageId);
                expect(
                    screen.getByRole("heading", {
                        level: 1,
                        name: destination,
                    }),
                ).toBeInTheDocument();
                expect(
                    screen.getAllByRole("heading", { level: 1 }),
                ).toHaveLength(1);
            }
        },
    );

    it("supports roving keyboard navigation and activation", async () => {
        const user = userEvent.setup();
        render(AppShell, {
            loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
            saveThemePreference: vi.fn(),
            p6Services: p6Services(),
        });

        const homeTab = screen.getByRole("tab", { name: "Home" });
        const discoverTab = screen.getByRole("tab", { name: "Discover" });
        const dashboardTab = screen.getByRole("tab", { name: "Dashboard" });
        const settingsTab = screen.getByRole("tab", { name: "Settings" });

        expect(homeTab).toHaveAttribute("tabindex", "0");
        expect(homeTab).toHaveAttribute("aria-controls", "home-panel");
        expect(discoverTab).toHaveAttribute("tabindex", "-1");
        homeTab.focus();

        await user.keyboard("{ArrowRight}");
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

    it("opens each primary destination at the top of the page", async () => {
        render(AppShell, {
            loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
            saveThemePreference: vi.fn(),
            p6Services: p6Services(),
        });
        document.documentElement.scrollTop = 640;
        document.body.scrollTop = 640;

        await fireEvent.click(screen.getByRole("tab", { name: "Settings" }));

        expect(document.documentElement.scrollTop).toBe(0);
        expect(document.body.scrollTop).toBe(0);
        expect(
            screen.getByRole("heading", { level: 1, name: "Settings" }),
        ).toBeInTheDocument();
    });

    it("forces and replays shared app states from Settings", async () => {
        const user = userEvent.setup();
        render(AppShell, {
            loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
            saveThemePreference: vi.fn(),
            p4Services: p4Services(),
            p6Services: p6Services(),
        });

        await user.click(screen.getByRole("tab", { name: "Settings" }));
        await user.click(screen.getByRole("radio", { name: /^Loading/ }));
        expect(screen.getByText("Settings preview")).toBeInTheDocument();
        expect(
            document.querySelector("[data-sleeping-cat-loader]"),
        ).toBeInTheDocument();

        await user.click(screen.getByRole("tab", { name: "Home" }));
        expect(document.querySelector(".app-chrome")).toHaveAttribute(
            "data-debug-presentation",
            "loading",
        );
        expect(screen.getByText("Debug override active")).toBeInTheDocument();
        expect(screen.getByRole("status")).toHaveAttribute(
            "data-status",
            "loading",
        );
        await user.click(
            screen.getByRole("button", { name: "Replay animation" }),
        );
        expect(
            document.querySelector("[data-sleeping-cat-loader]"),
        ).toBeInTheDocument();

        await user.click(
            screen.getByRole("button", { name: "Debug settings" }),
        );
        await user.click(screen.getByRole("radio", { name: /^Error/ }));
        await user.click(screen.getByRole("tab", { name: "Dashboard" }));
        expect(screen.getByRole("alert")).toHaveAttribute(
            "data-status",
            "error",
        );
        expect(screen.getByRole("alert")).toHaveTextContent(
            /simulated error from the Settings state debugger/i,
        );

        await user.click(screen.getByRole("tab", { name: "Settings" }));
        await user.click(screen.getByRole("radio", { name: /^Live app/ }));
        await user.click(screen.getByRole("tab", { name: "Home" }));
        expect(document.querySelector(".app-chrome")).toHaveAttribute(
            "data-debug-presentation",
            "off",
        );
        expect(
            screen.queryByText("Debug override active"),
        ).not.toBeInTheDocument();
    });

    it("keeps Discover selected while media details open secondarily", async () => {
        const user = userEvent.setup();
        render(AppShell, {
            loadThemePreference: vi.fn().mockResolvedValue("SYSTEM"),
            saveThemePreference: vi.fn(),
            p4Services: p4Services(),
            p6Services: p6Services(),
        });

        const discoverTab = screen.getByRole("tab", { name: "Discover" });
        await user.click(discoverTab);
        await user.click(
            screen.getByRole("button", {
                name: "Open details for Paper Cranes at Dawn",
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
            p6Services: p6Services(),
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
            expect(document.documentElement).toHaveAttribute(
                "data-theme",
                "latte",
            ),
        );
        expect(saveThemePreference).toHaveBeenCalledWith("LATTE");

        await fireEvent.click(screen.getByRole("radio", { name: /System/ }));
        await waitFor(() =>
            expect(document.documentElement).not.toHaveAttribute("data-theme"),
        );
        expect(saveThemePreference).toHaveBeenCalledWith("SYSTEM");
    });

    it("keeps the active theme when saving a preference fails", async () => {
        const saveThemePreference = vi
            .fn()
            .mockRejectedValueOnce({
                code: "DATABASE_UNAVAILABLE",
                message: "Animi could not save the theme preference.",
                retryable: true,
            })
            .mockResolvedValueOnce("MACCHIATO");
        render(AppShell, {
            loadThemePreference: vi.fn().mockResolvedValue("LATTE"),
            saveThemePreference,
            p6Services: p6Services(),
        });

        await waitFor(() =>
            expect(document.documentElement).toHaveAttribute(
                "data-theme",
                "latte",
            ),
        );
        await fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
        await fireEvent.click(screen.getByRole("radio", { name: /Dark/ }));

        expect(await screen.findByRole("alert")).toHaveTextContent(
            "Animi could not save the theme preference.",
        );
        expect(document.documentElement).toHaveAttribute("data-theme", "latte");
        expect(screen.getByRole("radio", { name: /Light/ })).toBeChecked();

        await fireEvent.click(
            screen.getByRole("button", { name: "Retry preference" }),
        );
        await waitFor(() =>
            expect(document.documentElement).toHaveAttribute(
                "data-theme",
                "macchiato",
            ),
        );
        expect(screen.getByRole("radio", { name: /Dark/ })).toBeChecked();
        expect(saveThemePreference).toHaveBeenCalledTimes(2);
        expect(saveThemePreference).toHaveBeenNthCalledWith(1, "MACCHIATO");
        expect(saveThemePreference).toHaveBeenNthCalledWith(2, "MACCHIATO");
    });

    it("retries a failed theme load only when the error is retryable", async () => {
        const loadThemePreference = vi
            .fn()
            .mockRejectedValueOnce({
                code: "DATABASE_UNAVAILABLE",
                message: "Animi could not load the theme preference.",
                retryable: true,
            })
            .mockResolvedValueOnce("MACCHIATO");
        render(AppShell, {
            loadThemePreference,
            saveThemePreference: vi.fn(),
            p6Services: p6Services(),
        });

        await fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "Animi could not load the theme preference.",
        );
        expect(screen.getByRole("radio", { name: /System/ })).toBeChecked();

        await fireEvent.click(
            screen.getByRole("button", { name: "Retry preference" }),
        );

        await waitFor(() =>
            expect(document.documentElement).toHaveAttribute(
                "data-theme",
                "macchiato",
            ),
        );
        expect(loadThemePreference).toHaveBeenCalledTimes(2);
        expect(screen.getByRole("radio", { name: /Dark/ })).toBeChecked();
    });

    it("does not offer a retry for a terminal theme error", async () => {
        render(AppShell, {
            loadThemePreference: vi.fn().mockRejectedValue({
                code: "INVALID_THEME_PREFERENCE",
                message: "The saved theme preference is invalid.",
                retryable: false,
            }),
            saveThemePreference: vi.fn(),
            p6Services: p6Services(),
        });

        await fireEvent.click(screen.getByRole("tab", { name: "Settings" }));
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "The saved theme preference is invalid.",
        );
        expect(
            screen.queryByRole("button", { name: "Retry preference" }),
        ).not.toBeInTheDocument();
    });
});
