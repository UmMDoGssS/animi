import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { LibraryItem } from "$lib/ipc/types";
import ReadingHome from "./ReadingHome.svelte";

function item(id = 1, total: number | null = null): LibraryItem {
    return {
        entry: {
            anilistId: id,
            mediaKind: "MANGA",
            status: "WATCHING",
            progress: 4,
            volumeProgress: 2,
            score: null,
            notes: null,
            createdAt: "2026-09-22T00:00:00Z",
            updatedAt: "2026-09-22T00:00:00Z",
        },
        media: {
            title: `Manga ${id}`,
            coverUrl: null,
            mediaFormat: "MANGA",
            genres: [],
            tags: [],
            status: "RELEASING",
            episodes: null,
            chapters: total,
            volumes: null,
            durationMinutes: null,
            releaseDate: null,
            cachedAt: "2000-01-01T00:00:00Z",
            expiresAt: "2000-01-02T00:00:00Z",
            isStale: true,
        },
    };
}

describe("Continue Reading", () => {
    it("records one chapter with keyboard control, preserves volumes, and announces it", async () => {
        const manga = item();
        const advanceProgress = vi.fn(async () => ({
            ...manga.entry,
            progress: 5,
        }));
        const onLibraryChanged = vi.fn();
        render(ReadingHome, {
            loadReading: async () => [manga],
            advanceProgress,
            onLibraryChanged,
        });
        const button = await screen.findByRole("button", {
            name: "Read one more chapter of Manga 1",
        });
        button.focus();
        await userEvent.keyboard("{Enter}");
        await waitFor(() =>
            expect(advanceProgress).toHaveBeenCalledWith({
                mediaKind: "MANGA",
                anilistId: 1,
            }),
        );
        expect(screen.getByText("5 / ? chapters")).toBeInTheDocument();
        expect(screen.getByText("2 volumes read")).toBeInTheDocument();
        expect(screen.getByRole("status")).toHaveTextContent(
            "chapter 5 recorded",
        );
        expect(onLibraryChanged).toHaveBeenCalledOnce();
        expect(screen.getByText("Chapter total unknown.")).toBeInTheDocument();
        expect(
            screen.getByText("Cached details may be out of date."),
        ).toBeInTheDocument();
    });

    it("stops at the known total while allowing missing metadata", async () => {
        const missing = { ...item(2), media: null };
        render(ReadingHome, { loadReading: async () => [item(1, 4), missing] });
        expect(
            await screen.findByRole("button", {
                name: "Read one more chapter of Manga 1",
            }),
        ).toBeDisabled();
        expect(
            screen.getByText("All known chapters read."),
        ).toBeInTheDocument();
        expect(
            screen.getByRole("button", {
                name: "Read one more chapter of AniList #2",
            }),
        ).toBeEnabled();
        expect(screen.getByText("Details are not cached.")).toBeInTheDocument();
        expect(
            screen.getByRole("img", { name: "Cover unavailable for Manga 1" }),
        ).toBeInTheDocument();
    });

    it("shows loading, recovers from a command error, and offers discovery for an empty shelf", async () => {
        const loadReading = vi
            .fn<() => Promise<LibraryItem[]>>()
            .mockRejectedValueOnce({
                code: "DATABASE_ERROR",
                message: "Try again",
                retryable: true,
            })
            .mockResolvedValue([]);
        const openDiscovery = vi.fn();
        render(ReadingHome, { loadReading, openDiscovery });
        expect(
            screen.getByText("Gathering your reading shelf."),
        ).toBeInTheDocument();
        await screen.findByText("Reading shelf needs attention");
        await userEvent.click(
            screen.getByRole("button", { name: "Try again" }),
        );
        await userEvent.click(
            await screen.findByRole("button", { name: "Find manga" }),
        );
        expect(openDiscovery).toHaveBeenCalledOnce();
    });

    it("keeps progress unchanged and exposes failures from a quick action", async () => {
        render(ReadingHome, {
            loadReading: async () => [item()],
            advanceProgress: async () => {
                throw {
                    code: "DATABASE_ERROR",
                    message: "Could not save",
                    retryable: true,
                };
            },
        });
        await userEvent.click(
            await screen.findByRole("button", {
                name: "Read one more chapter of Manga 1",
            }),
        );
        await screen.findByText("Could not save");
        expect(screen.getByText("4 / ? chapters")).toBeInTheDocument();
    });
});
