import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

import type {
  DashboardInsights,
  LibraryEntry,
  LibraryItem,
  LibraryMediaSnapshot,
  MediaDetailsResponse,
} from "$lib/ipc/types";

import { setTestViewport, testViewports } from "../../../test/viewports";

import DashboardPage from "./DashboardPage.svelte";

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
    Object.defineProperty(HTMLDialogElement.prototype, "close", originalClose);
  } else {
    Reflect.deleteProperty(HTMLDialogElement.prototype, "close");
  }
});

function entry(
  anilistId: number,
  status: LibraryEntry["status"],
  progress: number,
  score: number | null = null,
): LibraryEntry {
  return {
    anilistId,
    mediaKind: "ANIME",
    status,
    progress,
    score,
    notes: null,
    createdAt: "2026-07-15T08:00:00.000Z",
    updatedAt: "2026-07-16T08:30:00.000Z",
  };
}

function media(
  title: string,
  overrides: Partial<LibraryMediaSnapshot> = {},
): LibraryMediaSnapshot {
  return {
    title,
    coverUrl: `https://example.invalid/${title}.jpg`,
    mediaFormat: "TV",
    genres: ["Drama"],
    tags: [{ name: "Found Family", rank: 90 }],
    status: "RELEASING",
    episodes: 12,
    durationMinutes: 24,
    cachedAt: "2026-07-16T08:00:00.000Z",
    expiresAt: "2026-07-17T08:00:00.000Z",
    isStale: false,
    ...overrides,
  };
}

const libraryItems: LibraryItem[] = [
  {
    entry: entry(101, "WATCHING", 3),
    media: media("Paper Cranes at Dawn"),
  },
  {
    entry: entry(202, "COMPLETED", 1, 9),
    media: media("How to Befriend a Comet", {
      coverUrl: null,
      mediaFormat: "MOVIE",
      genres: ["Fantasy"],
      tags: [{ name: "Space", rank: 91 }],
      status: "FINISHED",
      episodes: 1,
      durationMinutes: 104,
      isStale: true,
    }),
  },
  {
    entry: entry(303, "PLANNING", 0, 7),
    media: media("Unknown Season", {
      coverUrl: null,
      status: "NOT_YET_RELEASED",
      episodes: null,
      durationMinutes: null,
    }),
  },
  {
    entry: entry(404, "WATCHING", 4),
    media: null,
  },
];

const populatedInsights: DashboardInsights = {
  mediaKind: "ANIME",
  libraryItems,
  libraryTotals: {
    tracked: 4,
    watching: 2,
    completed: 1,
    planning: 1,
    currentProgress: 8,
  },
  preferenceSignals: {
    eligibleTitles: 3,
    metadataTitles: 2,
    missingMetadataTitles: 1,
    staleMetadataTitles: 1,
    genres: [
      { label: "Drama", titleCount: 1 },
      { label: "Fantasy", titleCount: 1 },
    ],
    tags: [
      { label: "Space", titleCount: 1, weightedRank: 91 },
      { label: "Found Family", titleCount: 1, weightedRank: 90 },
    ],
  },
  activity: {
    eventCount: 2,
    recordedEpisodes: 3,
    knownDurationEpisodes: 2,
    recordedMinutes: 48,
    unknownDurationEpisodes: 1,
    events: [
      {
        anilistId: 101,
        mediaKind: "ANIME",
        progressDelta: 2,
        episodeDurationMinutes: 24,
        occurredAt: "2026-07-16T09:00:00.000Z",
      },
      {
        anilistId: 404,
        mediaKind: "ANIME",
        progressDelta: 1,
        episodeDurationMinutes: null,
        occurredAt: "2026-07-16T10:00:00.000Z",
      },
    ],
  },
};

const emptyInsights: DashboardInsights = {
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
    events: [],
  },
};

function detailsFor(item: LibraryItem): MediaDetailsResponse {
  const summary = item.media;
  return {
    media: {
      anilistId: item.entry.anilistId,
      title: summary?.title ?? `AniList #${item.entry.anilistId}`,
      coverUrl: summary?.coverUrl ?? null,
      mediaFormat: summary?.mediaFormat ?? null,
      status: summary?.status ?? null,
      episodes: summary?.episodes ?? null,
      durationMinutes: summary?.durationMinutes ?? null,
      genres: summary?.genres ?? [],
      tags: summary?.tags ?? [],
      trending: 0,
      sequels: [],
    },
    freshness: {
      fetchedAt: "2026-07-16T08:00:00.000Z",
      expiresAt: "2026-07-17T08:00:00.000Z",
      isStale: false,
      refreshError: null,
    },
  };
}

describe("DashboardPage", () => {
  it.each(testViewports)(
    "renders authoritative totals, signals, activity, and every shelf at the $name viewport",
    async (viewport) => {
      setTestViewport(viewport);
      render(DashboardPage, {
        loadInsights: vi.fn().mockResolvedValue(populatedInsights),
        refreshMetadata: vi.fn(),
      });

      expect(
        screen.getByRole("heading", { level: 1, name: "Dashboard" }),
      ).toBeInTheDocument();
      const totals = await screen.findByRole("group", {
        name: "Local library totals",
      });
      expect(totals).toHaveTextContent(/tracked\s*04/i);
      expect(totals).toHaveTextContent(/watching\s*02/i);
      expect(totals).toHaveTextContent(/completed\s*01/i);
      expect(totals).toHaveTextContent(/planning\s*01/i);
      expect(screen.getByText(/8 episodes/i)).toBeInTheDocument();
      expect(screen.getByText(/2 of 3 eligible titles/i)).toBeInTheDocument();
      expect(
        screen.getByRole("list", { name: "Genre preference signals" }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("list", { name: "Tag preference signals" }),
      ).toHaveTextContent(/Space.*1/i);

      const activity = screen.getByRole("group", {
        name: "Recorded activity summary",
      });
      expect(activity).toHaveTextContent(/events\s*2/i);
      expect(activity).toHaveTextContent(/recorded episodes\s*3/i);
      expect(activity).toHaveTextContent(/known watch time\s*48m/i);
      expect(activity).toHaveTextContent(/duration excluded\s*1 episode/i);
      expect(
        screen.getByRole("list", { name: "Recent recorded progress events" }),
      ).toHaveTextContent(/Paper Cranes at Dawn.*\+2 episodes.*48m/i);
      expect(screen.queryByRole("table")).not.toBeInTheDocument();

      expect(
        within(
          screen.getByRole("list", { name: "Watching anime" }),
        ).getAllByRole("listitem"),
      ).toHaveLength(2);
      expect(
        within(
          screen.getByRole("list", { name: "Completed anime" }),
        ).getAllByRole("listitem"),
      ).toHaveLength(1);
      expect(
        within(
          screen.getByRole("list", { name: "Planning anime" }),
        ).getAllByRole("listitem"),
      ).toHaveLength(1);
      expect(screen.getAllByText("AniList #404")).toHaveLength(2);
      expect(screen.getAllByText("episode total unknown")).toHaveLength(2);
      expect(screen.getByText(/score \/\/ 7 \/ 10/i)).toBeInTheDocument();
      expect(screen.getByText(/stale cache/i)).toBeInTheDocument();
    },
  );

  it("shows useful whole-library and activity empty states", async () => {
    const openDiscovery = vi.fn();
    const user = userEvent.setup();
    render(DashboardPage, {
      loadInsights: vi.fn().mockResolvedValue(emptyInsights),
      refreshMetadata: vi.fn(),
      openDiscovery,
    });

    expect(
      await screen.findByRole("heading", {
        level: 3,
        name: "Your library has no anime yet.",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "No progress activity has been recorded yet.",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/establish a baseline without fabricating watch time/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "check tracked metadata" }),
    ).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "open Discover" }));
    expect(openDiscovery).toHaveBeenCalledOnce();
  });

  it("retries a terminal initial load failure", async () => {
    const loadInsights = vi
      .fn()
      .mockRejectedValueOnce({
        code: "DATABASE_UNAVAILABLE",
        message: "The local database is busy.",
        retryable: true,
      })
      .mockResolvedValueOnce(emptyInsights);
    const user = userEvent.setup();
    render(DashboardPage, { loadInsights, refreshMetadata: vi.fn() });

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "The local database is busy.",
    );
    await user.click(screen.getByRole("button", { name: "Retry Dashboard" }));

    expect(
      await screen.findByRole("group", { name: "Local library totals" }),
    ).toHaveTextContent(/tracked\s*00/i);
    expect(loadInsights).toHaveBeenCalledTimes(2);
  });

  it("reports partial metadata refreshes while keeping cached shelves visible", async () => {
    const loadInsights = vi.fn().mockResolvedValue(populatedInsights);
    const refreshMetadata = vi.fn().mockResolvedValue({
      attempted: 4,
      updated: 1,
      stale: 2,
      failures: [
        {
          anilistId: 404,
          error: {
            code: "ANILIST_OFFLINE",
            message: "AniList is unavailable.",
            retryable: true,
          },
        },
      ],
    });
    const user = userEvent.setup();
    render(DashboardPage, { loadInsights, refreshMetadata });

    await user.click(
      await screen.findByRole("button", { name: "check tracked metadata" }),
    );

    expect(await screen.findByRole("status")).toHaveTextContent(
      /1 title updated, 2 titles stale, 1 title failed out of 4 tracked titles/i,
    );
    expect(
      screen.getByRole("list", { name: "Watching anime" }),
    ).toHaveTextContent("Paper Cranes at Dawn");
    expect(loadInsights).toHaveBeenCalledTimes(2);
  });

  it("keeps local data visible when a metadata check is offline", async () => {
    const refreshMetadata = vi.fn().mockRejectedValue({
      code: "ANILIST_OFFLINE",
      message: "AniList is unavailable.",
      retryable: true,
    });
    const user = userEvent.setup();
    render(DashboardPage, {
      loadInsights: vi.fn().mockResolvedValue(populatedInsights),
      refreshMetadata,
    });

    await user.click(
      await screen.findByRole("button", { name: "check tracked metadata" }),
    );

    expect(await screen.findByRole("status")).toHaveTextContent(
      /Your local edits and saved metadata remain unchanged/i,
    );
    expect(
      screen.getByRole("list", { name: "Completed anime" }),
    ).toHaveTextContent("How to Befriend a Comet");
  });

  it("opens cached library details and restores focus after closing", async () => {
    const user = userEvent.setup();
    const loadDetails = vi.fn().mockResolvedValue(detailsFor(libraryItems[0]!));
    render(DashboardPage, {
      loadInsights: vi.fn().mockResolvedValue(populatedInsights),
      refreshMetadata: vi.fn(),
      loadDetails,
      saveEntry: vi.fn(),
      deleteEntry: vi.fn(),
    });

    const opener = await screen.findByRole("button", {
      name: "Open library details for Paper Cranes at Dawn",
    });
    await user.click(opener);

    const dialog = screen.getByRole("dialog", {
      name: "Paper Cranes at Dawn",
    });
    expect(dialog).toHaveAttribute("open");
    expect(
      within(dialog).getByText("Update your local entry"),
    ).toBeInTheDocument();
    expect(loadDetails).toHaveBeenCalledWith({ anilistId: 101 });

    await user.click(
      within(dialog).getByRole("button", { name: "Close details" }),
    );
    await waitFor(() => expect(dialog).not.toBeInTheDocument());
    expect(opener).toHaveFocus();
  });

  it("keeps saved shelf mutations visible when the aggregate reload fails and restores focus after a move or removal", async () => {
    const user = userEvent.setup();
    const loadInsights = vi
      .fn()
      .mockResolvedValueOnce(populatedInsights)
      .mockRejectedValue({
        code: "DATABASE_UNAVAILABLE",
        message: "The aggregate snapshot could not reload.",
        retryable: true,
      });
    const saveEntry = vi.fn(async (request) => ({
      ...libraryItems[0]!.entry,
      ...request,
      updatedAt: "2026-07-17T11:00:00.000Z",
    }));
    const deleteEntry = vi.fn().mockResolvedValue({ removed: true });
    render(DashboardPage, {
      loadInsights,
      refreshMetadata: vi.fn(),
      loadDetails: vi.fn().mockResolvedValue(detailsFor(libraryItems[0]!)),
      saveEntry,
      deleteEntry,
    });

    await user.click(
      await screen.findByRole("button", {
        name: "Open library details for Paper Cranes at Dawn",
      }),
    );
    let dialog = await screen.findByRole("dialog", {
      name: "Paper Cranes at Dawn",
    });
    await user.click(within(dialog).getByRole("radio", { name: /Completed/ }));
    const progress = within(dialog).getByRole("textbox", {
      name: "Episodes watched",
    });
    await user.clear(progress);
    await user.type(progress, "5");
    await user.click(
      within(dialog).getByRole("button", { name: "Save changes" }),
    );

    expect(
      await screen.findByText(/saved library rows are updated/i),
    ).toBeInTheDocument();
    const movedCard = within(
      screen.getByRole("list", { name: "Completed anime" }),
    ).getByRole("button", {
      name: "Open library details for Paper Cranes at Dawn",
    });
    expect(movedCard).toHaveTextContent(/5 \/ 12 eps/);

    await user.click(
      within(dialog).getByRole("button", { name: "Close details" }),
    );
    await waitFor(() => expect(movedCard).toHaveFocus());

    await user.click(movedCard);
    dialog = await screen.findByRole("dialog", {
      name: "Paper Cranes at Dawn",
    });
    await user.click(
      within(dialog).getByRole("button", { name: "Remove from library" }),
    );
    expect(deleteEntry).not.toHaveBeenCalled();
    await user.click(
      within(dialog).getByRole("button", { name: "Confirm local removal" }),
    );
    await waitFor(() =>
      expect(
        screen.queryByRole("button", {
          name: "Open library details for Paper Cranes at Dawn",
        }),
      ).not.toBeInTheDocument(),
    );
    expect(
      screen.getByRole("group", { name: "Local library totals" }),
    ).toHaveTextContent(/tracked\s*03/i);
    await user.click(
      within(dialog).getByRole("button", { name: "Close details" }),
    );
    await waitFor(() =>
      expect(
        screen.getByRole("heading", {
          name: "Every local shelf stays in view.",
        }),
      ).toHaveFocus(),
    );
  });

  it("replaces a metadata-less card after details persist a fresh snapshot", async () => {
    const recoveredMedia = media("Recovered Metadata", {
      coverUrl: null,
      genres: ["Mystery"],
      tags: [{ name: "Archives", rank: 86 }],
      episodes: 24,
    });
    const recoveredInsights: DashboardInsights = {
      ...populatedInsights,
      libraryItems: populatedInsights.libraryItems.map((item) =>
        item.entry.anilistId === 404
          ? { ...item, media: recoveredMedia }
          : item,
      ),
    };
    const recoveredDetails = detailsFor({
      entry: libraryItems[3]!.entry,
      media: recoveredMedia,
    });
    const loadInsights = vi
      .fn()
      .mockResolvedValueOnce(populatedInsights)
      .mockResolvedValue(recoveredInsights);
    const user = userEvent.setup();
    render(DashboardPage, {
      loadInsights,
      refreshMetadata: vi.fn(),
      loadDetails: vi.fn().mockResolvedValue(recoveredDetails),
      saveEntry: vi.fn(),
      deleteEntry: vi.fn(),
    });

    await user.click(
      await screen.findByRole("button", {
        name: "Open library details for AniList #404",
      }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Recovered Metadata",
    });
    expect(within(dialog).getByText("24 episodes")).toBeInTheDocument();
    await user.click(
      within(dialog).getByRole("button", { name: "Close details" }),
    );

    const refreshedCard = await screen.findByRole("button", {
      name: "Open library details for Recovered Metadata",
    });
    expect(refreshedCard).toHaveTextContent(/4 \/ 24 eps/);
    expect(refreshedCard).toHaveTextContent(/cached/i);
    expect(loadInsights).toHaveBeenCalledTimes(2);
  });
});
