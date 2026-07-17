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
  mediaStatus = "RELEASING",
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
      expect(carousel).toHaveClass(
        "overflow-x-auto",
        "snap-x",
        "scrollbar-hidden",
      );
      expect(within(carousel).getAllByRole("group")).toHaveLength(3);
      expect(
        within(carousel).getByRole("group", {
          name: /1 of 3: Moonlit Post Office/i,
        }),
      ).toBeInTheDocument();
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
    expect(within(unfinished).getAllByText("finished").length).toBeGreaterThan(
      0,
    );
    expect(
      within(unfinished).getByText("2 episodes left to watch."),
    ).toBeInTheDocument();
    expect(
      within(unfinished).getByRole("button", { name: "watch ep 11" }),
    ).toBeEnabled();

    await user.click(screen.getByRole("button", { name: "Next anime" }));
    const complete = screen.getByRole("group", { name: /Complete Catnap/i });
    expect(
      within(complete).getByRole("button", { name: "all caught up" }),
    ).toBeDisabled();
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
