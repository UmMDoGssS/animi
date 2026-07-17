import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  HomeAiringItem,
  HomeAiringResponse,
  LibraryEntry,
} from "$lib/ipc/types";

import { setTestViewport, testViewports } from "../../../test/viewports";

import HomePage from "./HomePage.svelte";

const checkedAt = "2026-07-17T08:00:00.000Z";

function entry(id: number, progress: number): LibraryEntry {
  return {
    anilistId: id,
    mediaKind: "ANIME",
    status: "WATCHING",
    progress,
    score: null,
    notes: null,
    createdAt: checkedAt,
    updatedAt: checkedAt,
  };
}

function item(
  id: number,
  title: string,
  progress: number,
  nextEpisode: number | null,
  stale = false,
): HomeAiringItem {
  return {
    library: {
      entry: entry(id, progress),
      media: {
        title,
        coverUrl: null,
        mediaFormat: "TV",
        genres: ["Drama"],
        tags: [{ name: "Time", rank: 80 }],
        status: "RELEASING",
        episodes: 12,
        durationMinutes: 24,
        cachedAt: checkedAt,
        expiresAt: "2026-07-18T08:00:00.000Z",
        isStale: false,
      },
    },
    nextAiring:
      nextEpisode === null
        ? null
        : {
            episode: nextEpisode,
            airingAt: new Date(Date.now() + 2 * 60 * 60 * 1000).toISOString(),
          },
    scheduleFreshness: {
      fetchedAt: checkedAt,
      expiresAt: "2026-07-17T08:30:00.000Z",
      isStale: stale,
      refreshError: null,
    },
    scheduleError: null,
  };
}

function response(items: HomeAiringItem[]): HomeAiringResponse {
  return { items, checkedAt };
}

function homeProps(items: HomeAiringItem[] = []) {
  return {
    loadAiring: vi.fn().mockResolvedValue(response(items)),
    advanceProgress: vi.fn(async ({ anilistId }: { anilistId: number }) => {
      const current = items.find(
        (candidate) => candidate.library.entry.anilistId === anilistId,
      );
      if (!current) throw new Error("missing fixture");
      return {
        ...current.library.entry,
        progress: current.library.entry.progress + 1,
        updatedAt: "2026-07-17T09:00:00.000Z",
      };
    }),
    openDiscovery: vi.fn(),
    refreshIntervalMs: 30 * 60 * 1000,
  };
}

afterEach(() => {
  vi.useRealTimers();
});

describe("HomePage", () => {
  it.each(testViewports)(
    "renders the live airing carousel at the $name viewport",
    async (viewport) => {
      setTestViewport(viewport);
      const props = homeProps([
        item(1, "Moonlit Post Office", 8, 9),
        item(2, "Last Train to Aomori", 5, 8, true),
        item(3, "Signal Lost at 2AM", 3, null),
      ]);
      render(HomePage, props);

      expect(
        screen.getByRole("heading", { level: 1, name: "Home" }),
      ).toBeInTheDocument();
      const carousel = await screen.findByRole("region", {
        name: "Airing anime",
      });
      expect(carousel).toHaveAttribute("aria-roledescription", "carousel");
      expect(
        within(carousel).getByRole("group", {
          name: /1 of 3: Moonlit Post Office/i,
        }),
      ).toBeInTheDocument();
      expect(screen.getByText("3").closest("p")).toHaveTextContent("watching");
      expect(screen.getByText("2").closest("p")).toHaveTextContent("waiting");
      expect(screen.getByText("1").closest("p")).toHaveTextContent("unknown");
      expect(screen.queryByText(/stale/i)).not.toBeInTheDocument();
      expect(props.loadAiring).toHaveBeenCalledOnce();
    },
  );

  it("moves between one active slide and keeps unknown schedules explicit", async () => {
    const user = userEvent.setup();
    render(
      HomePage,
      homeProps([
        item(1, "Moonlit Post Office", 8, 9),
        item(2, "Signal Lost at 2AM", 3, null),
      ]),
    );
    await screen.findByRole("region", { name: "Airing anime" });

    await user.click(screen.getByRole("button", { name: "Next anime" }));
    const unknown = screen.getByRole("group", {
      name: /2 of 2: Signal Lost at 2AM/i,
    });
    expect(
      within(unknown).getAllByText("schedule unknown").length,
    ).toBeGreaterThan(0);
    expect(
      within(unknown).getByText(/no upcoming episode date/i),
    ).toBeInTheDocument();
    expect(
      within(unknown).getByRole("button", { name: "schedule unavailable" }),
    ).toBeDisabled();

    const next = screen.getByRole("button", { name: "Next anime" });
    next.focus();
    await user.keyboard("{Home}");
    expect(
      screen.getByRole("group", { name: /1 of 2: Moonlit Post Office/i }),
    ).toBeInTheDocument();
  });

  it("persists bounded catch-up progress through the quick command", async () => {
    const user = userEvent.setup();
    const props = homeProps([item(2, "Last Train to Aomori", 5, 8)]);
    render(HomePage, props);
    const slide = await screen.findByRole("group", {
      name: /Last Train to Aomori/i,
    });
    expect(within(slide).getByText(/2 episodes waiting/i)).toBeInTheDocument();

    await user.click(
      within(slide).getByRole("button", {
        name: "mark episode 6 watched",
      }),
    );
    expect(props.advanceProgress).toHaveBeenCalledWith({ anilistId: 2 });
    expect(within(slide).getByText("06 / 12")).toBeInTheDocument();
    expect(within(slide).getByText(/1 episode waiting/i)).toBeInTheDocument();
  });

  it("checks at launch, an allowed manual action, and the 30-minute interval", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-17T08:00:00.000Z"));
    const props = homeProps([item(1, "Moonlit Post Office", 8, 9)]);
    render(HomePage, props);
    await vi.advanceTimersByTimeAsync(0);
    expect(props.loadAiring).toHaveBeenCalledOnce();

    await vi.advanceTimersByTimeAsync(30 * 60 * 1000);
    expect(props.loadAiring).toHaveBeenCalledTimes(2);

    await fireEvent.click(screen.getByRole("button", { name: "refresh" }));
    expect(props.loadAiring).toHaveBeenCalledTimes(3);
  });

  it("shows the no-active-anime state and opens Discover", async () => {
    const user = userEvent.setup();
    const props = homeProps();
    render(HomePage, props);
    expect(
      await screen.findByRole("heading", {
        level: 3,
        name: "Nothing is in your Watching shelf.",
      }),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /visit Discover/i }));
    expect(props.openDiscovery).toHaveBeenCalledOnce();
  });
});
