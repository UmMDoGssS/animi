import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  HomeAiringItem,
  HomeAiringResponse,
  LibraryEntry,
  MediaDetailsResponse,
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
  mediaStatus = "RELEASING",
  episodes = 12,
  airingInHours = 2,
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
        status: mediaStatus,
        episodes,
        durationMinutes: 24,
        releaseDate: null,
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
            airingAt: new Date(
              Date.now() + airingInHours * 60 * 60 * 1000,
            ).toISOString(),
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

function detailsFor(homeItem: HomeAiringItem): MediaDetailsResponse {
  return {
    media: {
      anilistId: homeItem.library.entry.anilistId,
      title: homeItem.library.media?.title ?? "Unknown anime",
      coverUrl: homeItem.library.media?.coverUrl ?? null,
      mediaFormat: homeItem.library.media?.mediaFormat ?? null,
      status: homeItem.library.media?.status ?? null,
      episodes: homeItem.library.media?.episodes ?? null,
      durationMinutes: homeItem.library.media?.durationMinutes ?? null,
      releaseDate: homeItem.library.media?.releaseDate ?? null,
      genres: homeItem.library.media?.genres ?? [],
      tags: homeItem.library.media?.tags ?? [],
      trending: 0,
      sequels: [],
    },
    freshness: {
      fetchedAt: checkedAt,
      expiresAt: "2026-07-18T08:00:00.000Z",
      isStale: false,
      refreshError: null,
    },
  };
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
    loadDetails: vi.fn(async ({ anilistId }: { anilistId: number }) => {
      const current = items.find(
        (candidate) => candidate.library.entry.anilistId === anilistId,
      );
      if (!current) throw new Error("missing fixture");
      return detailsFor(current);
    }),
    openDiscovery: vi.fn(),
    refreshIntervalMs: 30 * 60 * 1000,
  };
}

afterEach(() => {
  vi.useRealTimers();
});

describe("HomePage", () => {
  it("prioritizes unwatched episodes, including finished anime, then the next airing time", async () => {
    const props = homeProps([
      item(1, "Caught Up Soon", 4, 5, false, "RELEASING", 12, 1),
      item(2, "Two Waiting Later", 4, 7, false, "RELEASING", 12, 8),
      item(3, "Finished With Four", 8, null, false, "FINISHED", 12),
      item(4, "One Waiting Soon", 4, 6, false, "RELEASING", 12, 2),
      item(5, "Schedule Unknown", 3, null),
    ]);
    render(HomePage, props);

    const carousel = await screen.findByRole("region", {
      name: "Airing anime",
    });
    expect(
      within(carousel)
        .getAllByRole("group")
        .map((slide) => slide.getAttribute("aria-label")),
    ).toEqual([
      "1 of 5: Finished With Four",
      "2 of 5: Two Waiting Later",
      "3 of 5: One Waiting Soon",
      "4 of 5: Caught Up Soon",
      "5 of 5: Schedule Unknown",
    ]);
  });

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
      expect(carousel).toHaveClass(
        "overflow-x-auto",
        "snap-x",
        "scrollbar-hidden",
      );
      expect(within(carousel).getAllByRole("group")).toHaveLength(3);
      const firstSlide = within(carousel).getByRole("group", {
        name: /1 of 3: Last Train to Aomori/i,
      });
      expect(firstSlide).toBeInTheDocument();
      const cover = within(firstSlide).getByRole("img", {
        name: /cover unavailable for Last Train to Aomori/i,
      });
      expect(cover.parentElement).toHaveClass("h-full");
      expect(cover.parentElement?.parentElement).toHaveClass(
        "aspect-[2/3]",
        "sm:aspect-auto",
        "sm:self-stretch",
      );
      expect(screen.getByText("3").closest("p")).toHaveTextContent("watching");
      expect(screen.getByText("2").closest("p")).toHaveTextContent("waiting");
      expect(screen.getByText("1").closest("p")).toHaveTextContent("unknown");
      expect(document.querySelector('[data-stat="watching"]')).toHaveClass(
        "text-info",
      );
      expect(document.querySelector('[data-stat="waiting"]')).toHaveClass(
        "text-warning",
      );
      expect(document.querySelector('[data-stat="unknown"]')).toHaveClass(
        "text-danger",
      );
      expect(screen.queryByText(/stale/i)).not.toBeInTheDocument();
      expect(props.loadAiring).toHaveBeenCalledOnce();
    },
  );

  it("keeps schedule messaging separate from episode progress", async () => {
    const user = userEvent.setup();
    const props = homeProps([
      item(1, "Moonlit Post Office", 8, 9),
      item(2, "Signal Lost at 2AM", 3, null),
    ]);
    render(HomePage, props);
    const carousel = await screen.findByRole("region", {
      name: "Airing anime",
    });
    const scrollTo = vi.fn();
    carousel.scrollTo = scrollTo;

    await user.click(screen.getByRole("button", { name: "Next anime" }));
    expect(scrollTo).toHaveBeenCalledWith({ left: 0, behavior: "smooth" });
    const unknown = screen.getByRole("group", {
      name: /2 of 2: Signal Lost at 2AM/i,
    });
    expect(
      within(unknown).getAllByText("schedule unknown").length,
    ).toBeGreaterThan(0);
    expect(
      within(unknown).getByText(/no upcoming episode date/i),
    ).toBeInTheDocument();
    const watchButton = within(unknown).getByRole("button", {
      name: "watch ep 4",
    });
    expect(watchButton).toBeEnabled();
    await user.click(watchButton);
    expect(props.advanceProgress).toHaveBeenCalledWith({ anilistId: 2 });

    const next = screen.getByRole("button", { name: "Next anime" });
    next.focus();
    await user.keyboard("{Home}");
    expect(
      screen.getByRole("group", { name: /1 of 2: Moonlit Post Office/i }),
    ).toBeInTheDocument();
  });

  it("opens anime details from a carousel cover and restores focus", async () => {
    const user = userEvent.setup();
    const props = homeProps([item(1, "Moonlit Post Office", 8, 9)]);
    render(HomePage, props);

    const coverButton = await screen.findByRole("button", {
      name: "Open details for Moonlit Post Office",
    });
    expect(coverButton).toHaveAttribute("aria-haspopup", "dialog");

    await user.click(coverButton);

    const dialog = await screen.findByRole("dialog");
    expect(props.loadDetails).toHaveBeenCalledWith({ anilistId: 1 });
    expect(
      within(dialog).getByRole("heading", { name: "Moonlit Post Office" }),
    ).toBeInTheDocument();

    await user.click(
      within(dialog).getByRole("button", { name: "Close details" }),
    );
    expect(coverButton).toHaveFocus();
  });

  it("keeps the selected slide in sync with horizontal scrolling", async () => {
    const props = homeProps([
      item(1, "Moonlit Post Office", 8, 9),
      item(2, "Signal Lost at 2AM", 3, null),
    ]);
    render(HomePage, props);

    const carousel = await screen.findByRole("region", {
      name: "Airing anime",
    });
    const slides = within(carousel).getAllByRole("group");
    vi.spyOn(carousel, "getBoundingClientRect").mockReturnValue({
      left: 0,
    } as DOMRect);
    vi.spyOn(slides[0]!, "getBoundingClientRect").mockReturnValue({
      left: -320,
    } as DOMRect);
    vi.spyOn(slides[1]!, "getBoundingClientRect").mockReturnValue({
      left: 0,
    } as DOMRect);
    Object.defineProperty(carousel, "scrollLeft", {
      configurable: true,
      value: 320,
    });

    await fireEvent.scroll(carousel);

    expect(slides[1]).toHaveAttribute("aria-current", "true");
    expect(screen.getByText(/02 \/\/ 02/)).toBeInTheDocument();
  });

  it("labels finished anime and uses total progress for the quick action", async () => {
    const props = homeProps([
      item(4, "After the Last Bell", 10, null, false, "FINISHED"),
      item(5, "Complete Catnap", 12, null, false, "FINISHED"),
    ]);
    const user = userEvent.setup();
    render(HomePage, props);

    const unfinished = await screen.findByRole("group", {
      name: /After the Last Bell/i,
    });
    expect(
      within(unfinished).getByText(/2 episodes waiting/i),
    ).toBeInTheDocument();
    expect(
      within(unfinished).getByText("Finished airing."),
    ).toBeInTheDocument();
    const watchNext = within(unfinished).getByRole("button", {
      name: "watch ep 11",
    });
    expect(watchNext).toBeEnabled();
    expect(watchNext).toHaveClass("glass-accent");
    expect(watchNext).not.toHaveClass("glass-control");

    await user.click(screen.getByRole("button", { name: "Next anime" }));
    const complete = screen.getByRole("group", { name: /Complete Catnap/i });
    expect(within(complete).getByText("finished")).toBeInTheDocument();
    expect(
      within(complete).getByText("You have watched every episode."),
    ).toBeInTheDocument();
    const caughtUp = within(complete).getByRole("button", {
      name: "all caught up",
    });
    expect(caughtUp).toBeDisabled();
    expect(caughtUp).toHaveClass("glass-control", "text-muted");
    expect(caughtUp).not.toHaveClass("glass-accent", "opacity-55");
  });

  it("shows all caught up when every currently aired episode is watched", async () => {
    const props = homeProps([
      item(6, "Two Episodes into Summer", 2, 3, false, "RELEASING", 12),
    ]);
    render(HomePage, props);

    const slide = await screen.findByRole("group", {
      name: /Two Episodes into Summer/i,
    });
    expect(
      within(slide).getByText(/you are right on time/i),
    ).toBeInTheDocument();
    const caughtUp = within(slide).getByRole("button", {
      name: "all caught up",
    });
    expect(caughtUp).toBeDisabled();
    expect(
      within(slide).queryByRole("button", { name: "watch ep 3" }),
    ).not.toBeInTheDocument();
    expect(props.advanceProgress).not.toHaveBeenCalled();
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
        name: "watch ep 6",
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

    const refreshButton = screen.getByRole("button", { name: "refresh" });
    expect(refreshButton).toHaveTextContent("");
    expect(refreshButton.querySelector("svg")).toBeInTheDocument();
    await fireEvent.click(refreshButton);
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
