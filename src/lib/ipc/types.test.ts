import { describe, expect, it } from "vitest";

import { isCommandError } from "./types";

import type {
  LibraryEntry,
  LibraryItem,
  MediaDetailsResponse,
  WatchActivity,
} from "./types";

describe("isCommandError", () => {
  it("accepts only the complete structured error shape", () => {
    expect(
      isCommandError({
        code: "OFFLINE",
        message: "No connection",
        retryable: true,
      }),
    ).toBe(true);
    expect(isCommandError({ code: "OFFLINE", message: "No connection" })).toBe(
      false,
    );
    expect(isCommandError("OFFLINE")).toBe(false);
  });
});

describe("AniList IPC contracts", () => {
  it("carry explicit cache freshness and unknown sequel dates", () => {
    const response: MediaDetailsResponse = {
      media: {
        anilistId: 21,
        title: "Incomplete Anime",
        coverUrl: null,
        mediaFormat: "TV",
        status: "RELEASING",
        episodes: null,
        durationMinutes: null,
        genres: ["Fantasy"],
        tags: [],
        trending: 12,
        sequels: [
          {
            anilistId: 22,
            title: "Unannounced Sequel",
            releaseDate: null,
          },
        ],
      },
      freshness: {
        fetchedAt: "2026-07-16T08:00:00.000Z",
        expiresAt: "2026-07-17T08:00:00.000Z",
        isStale: true,
        refreshError: {
          code: "ANILIST_OFFLINE",
          message: "AniList is unavailable.",
          retryable: true,
        },
      },
    };

    expect(response.media.episodes).toBeNull();
    expect(response.media.durationMinutes).toBeNull();
    expect(response.media.sequels[0]?.releaseDate).toBeNull();
    expect(response.freshness.isStale).toBe(true);
  });
});

describe("library IPC contracts", () => {
  it("use camelCase fields and UTC RFC 3339 timestamp strings", () => {
    const entry: LibraryEntry = {
      anilistId: 101,
      mediaKind: "ANIME",
      status: "WATCHING",
      progress: 3,
      score: 8.5,
      notes: null,
      createdAt: "2026-07-16T08:00:00.000Z",
      updatedAt: "2026-07-16T08:30:00.000Z",
    };

    expect(entry.mediaKind).toBe("ANIME");
    expect(entry.updatedAt.endsWith("Z")).toBe(true);
  });

  it("carry optional cached media and explicit freshness", () => {
    const item: LibraryItem = {
      entry: {
        anilistId: 101,
        mediaKind: "ANIME",
        status: "WATCHING",
        progress: 3,
        score: null,
        notes: null,
        createdAt: "2026-07-16T08:00:00.000Z",
        updatedAt: "2026-07-16T08:30:00.000Z",
      },
      media: {
        title: "A Test Anime",
        coverUrl: "https://example.invalid/cover.jpg",
        mediaFormat: "TV",
        genres: ["Slice of Life"],
        tags: [{ name: "Cats", rank: 99 }],
        status: "RELEASING",
        episodes: 12,
        durationMinutes: 24,
        cachedAt: "2026-07-16T08:00:00.000Z",
        expiresAt: "2026-07-17T08:00:00.000Z",
        isStale: false,
      },
    };

    const missingMedia: LibraryItem = {
      entry: item.entry,
      media: null,
    };

    expect(item.media?.durationMinutes).toBe(24);
    expect(item.media?.isStale).toBe(false);
    expect(missingMedia.media).toBeNull();
  });

  it("represent progress activity without inventing unknown duration", () => {
    const activity: WatchActivity = {
      anilistId: 101,
      mediaKind: "ANIME",
      progressDelta: 2,
      episodeDurationMinutes: null,
      occurredAt: "2026-07-16T08:30:00.000Z",
    };

    expect(activity.progressDelta).toBe(2);
    expect(activity.episodeDurationMinutes).toBeNull();
    expect(activity.occurredAt.endsWith("Z")).toBe(true);
  });
});
