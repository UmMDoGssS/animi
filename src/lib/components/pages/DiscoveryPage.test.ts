import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  vi,
} from "vitest";

import type {
  CommandError,
  Freshness,
  LibraryEntry,
  LibraryItem,
  MediaDetailsResponse,
  MediaPageResponse,
  MediaSummary,
  RecommendationsResponse,
  SearchMediaRequest,
  UpsertLibraryEntryRequest,
} from "$lib/ipc/types";

import { setTestViewport, testViewports } from "../../../test/viewports";

import DiscoveryPage from "./DiscoveryPage.svelte";

const currentFreshness: Freshness = {
  fetchedAt: "2026-07-17T08:00:00.000Z",
  expiresAt: "2026-07-17T14:00:00.000Z",
  isStale: false,
  refreshError: null,
};

const offlineError: CommandError = {
  code: "ANILIST_OFFLINE",
  message: "AniList is unavailable and no cached data could be used.",
  retryable: true,
};

const violetEvergreen: MediaSummary = {
  anilistId: 101,
  title: "Violet Evergreen",
  coverUrl: "https://example.invalid/covers/violet.jpg",
  mediaFormat: "TV",
  status: "FINISHED",
  episodes: 13,
  durationMinutes: 24,
  genres: ["Drama", "Fantasy"],
  tags: [{ name: "Letters", rank: 88 }],
  trending: 96,
};

const orbitalCats: MediaSummary = {
  anilistId: 202,
  title: "Orbital Cats",
  coverUrl: null,
  mediaFormat: "TV_SHORT",
  status: "RELEASING",
  episodes: null,
  durationMinutes: null,
  genres: ["Comedy"],
  tags: [{ name: "Space", rank: 92 }],
  trending: 84,
};

const existingEntry: LibraryEntry = {
  anilistId: violetEvergreen.anilistId,
  mediaKind: "ANIME",
  status: "WATCHING",
  progress: 3,
  score: 8,
  notes: null,
  createdAt: "2026-07-16T08:00:00.000Z",
  updatedAt: "2026-07-17T08:00:00.000Z",
};

function pageResponse(
  items: MediaSummary[],
  freshness: Freshness = currentFreshness,
): MediaPageResponse {
  return { items, hasNextPage: false, freshness };
}

function recommendationsResponse(
  items: MediaSummary[] = [orbitalCats],
  isFallback = false,
): RecommendationsResponse {
  return {
    items: items.map((media) => ({
      media,
      score: isFallback ? 0 : 0.82,
      explanationLabels: isFallback ? [] : ["Space"],
    })),
    isFallback,
    eligibleProfileTitles: isFallback ? 1 : 3,
    freshness: currentFreshness,
  };
}

function detailsResponse(summary: MediaSummary): MediaDetailsResponse {
  return {
    media: { ...summary, sequels: [] },
    freshness: currentFreshness,
  };
}

function libraryItem(entry: LibraryEntry): LibraryItem {
  return { entry, media: null };
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
    updatedAt: "2026-07-17T09:00:00.000Z",
  };
}

function discoveryProps(overrides: Record<string, unknown> = {}) {
  return {
    loadTrending: vi
      .fn()
      .mockResolvedValue(pageResponse([violetEvergreen, orbitalCats])),
    loadRecommendations: vi.fn().mockResolvedValue(recommendationsResponse()),
    searchAnime: vi.fn().mockResolvedValue(pageResponse([orbitalCats])),
    loadDetails: vi.fn().mockResolvedValue(detailsResponse(violetEvergreen)),
    loadLibrary: vi.fn().mockResolvedValue([libraryItem(existingEntry)]),
    saveEntry: vi.fn(async (request: UpsertLibraryEntryRequest) =>
      savedEntry(request),
    ),
    deleteEntry: vi.fn().mockResolvedValue({ removed: true }),
    debounceMs: 5,
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
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

afterEach(() => {
  vi.useRealTimers();
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

describe("DiscoveryPage", () => {
  it.each(testViewports)(
    "renders live trending cards at the $name viewport",
    async (viewport) => {
      setTestViewport(viewport);
      const props = discoveryProps();
      render(DiscoveryPage, props);

      expect(
        screen.getByRole("heading", { level: 1, name: "Discover" }),
      ).toBeInTheDocument();
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(
        screen.getByRole("search", { name: "Search anime" }),
      ).toBeInTheDocument();

      const trending = await screen.findByRole("list", {
        name: "Trending anime",
      });
      expect(props.loadTrending).toHaveBeenCalledWith({
        page: 1,
        perPage: 12,
      });
      expect(props.loadLibrary).toHaveBeenCalledOnce();
      expect(
        within(trending).getByRole("img", {
          name: "Cover artwork for Violet Evergreen",
        }),
      ).toHaveAttribute("src", violetEvergreen.coverUrl);
      expect(
        within(trending).getByRole("img", {
          name: "Cover unavailable for Orbital Cats",
        }),
      ).toBeInTheDocument();
      expect(
        within(trending).getByRole("button", {
          name: "Open details for Violet Evergreen",
        }),
      ).toHaveTextContent("tracked // watching");
      expect(
        within(trending).getByRole("button", {
          name: "Open details for Orbital Cats",
        }),
      ).toHaveTextContent("not tracked");
      expect(trending).toHaveClass("min-w-0", "grid-cols-2", "sm:grid-cols-3");
      expect(
        screen.queryByText(/recommendation layout preview/i),
      ).not.toBeInTheDocument();
    },
  );

  it("shows explainable local recommendations and an explicit trending fallback", async () => {
    const personalized = discoveryProps();
    const { unmount } = render(DiscoveryPage, personalized);
    const recommendations = await screen.findByRole("list", {
      name: "Recommended anime",
    });
    expect(personalized.loadRecommendations).toHaveBeenCalledWith({
      perPage: 18,
    });
    expect(
      within(recommendations).getByText("Matches Space"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Personalized from 3 local titles/i),
    ).toBeInTheDocument();
    unmount();

    render(
      DiscoveryPage,
      discoveryProps({
        loadRecommendations: vi
          .fn()
          .mockResolvedValue(recommendationsResponse([orbitalCats], true)),
      }),
    );
    expect(
      await screen.findByText(
        /Trending fallback \/\/ 1 eligible local titles/i,
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Trending while your local taste profile grows/i),
    ).toBeInTheDocument();
  });

  it("debounces trimmed searches and lets form submission search immediately", async () => {
    vi.useFakeTimers();
    const searchAnime = vi.fn().mockResolvedValue(pageResponse([orbitalCats]));
    render(DiscoveryPage, discoveryProps({ searchAnime, debounceMs: 350 }));

    const search = screen.getByRole("searchbox", { name: "Anime title" });
    await fireEvent.input(search, { target: { value: "  Orbital Cats  " } });
    expect(searchAnime).not.toHaveBeenCalled();
    expect(
      screen
        .getByText('Searching for "Orbital Cats"')
        .closest('[data-status="loading"]'),
    ).not.toBeNull();

    await vi.advanceTimersByTimeAsync(349);
    expect(searchAnime).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(searchAnime).toHaveBeenCalledWith({
      search: "Orbital Cats",
      page: 1,
      perPage: 12,
    });

    await fireEvent.input(search, { target: { value: "Violet Evergreen" } });
    await fireEvent.submit(search.closest("form") as HTMLFormElement);
    expect(searchAnime).toHaveBeenLastCalledWith({
      search: "Violet Evergreen",
      page: 1,
      perPage: 12,
    });
    expect(searchAnime).toHaveBeenCalledTimes(2);
  });

  it("keeps only the newest search response when requests finish out of order", async () => {
    const first = deferred<MediaPageResponse>();
    const second = deferred<MediaPageResponse>();
    const searchAnime = vi.fn((request: SearchMediaRequest) =>
      request.search === "first" ? first.promise : second.promise,
    );
    render(DiscoveryPage, discoveryProps({ searchAnime, debounceMs: 0 }));

    const search = screen.getByRole("searchbox", { name: "Anime title" });
    await fireEvent.input(search, { target: { value: "first" } });
    await waitFor(() => expect(searchAnime).toHaveBeenCalledTimes(1));
    await fireEvent.input(search, { target: { value: "second" } });
    await waitFor(() => expect(searchAnime).toHaveBeenCalledTimes(2));

    const secondResult = {
      ...orbitalCats,
      anilistId: 302,
      title: "Second Result",
    };
    second.resolve(pageResponse([secondResult]));
    expect(
      await screen.findByRole("button", {
        name: "Open details for Second Result",
      }),
    ).toBeInTheDocument();

    const firstResult = {
      ...violetEvergreen,
      anilistId: 301,
      title: "First Result",
    };
    first.resolve(pageResponse([firstResult]));
    await waitFor(() =>
      expect(
        screen.queryByRole("button", {
          name: "Open details for First Result",
        }),
      ).not.toBeInTheDocument(),
    );
    expect(
      screen.getByRole("button", { name: "Open details for Second Result" }),
    ).toBeInTheDocument();
  });

  it("shows an empty search result and clears back to trending", async () => {
    const user = userEvent.setup();
    const searchAnime = vi.fn().mockResolvedValue(pageResponse([]));
    render(DiscoveryPage, discoveryProps({ searchAnime, debounceMs: 1000 }));

    const search = screen.getByRole("searchbox", { name: "Anime title" });
    await user.type(search, "No Such Anime{Enter}");

    expect(
      await screen.findByText('No anime found for "No Such Anime"'),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("list", { name: "Trending anime" }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Clear search" }));
    expect(
      await screen.findByRole("list", { name: "Trending anime" }),
    ).toBeInTheDocument();
    expect(search).toHaveValue("");
    await waitFor(() => expect(search).toHaveFocus());
  });

  it("terminates a search error with offline copy and a retry", async () => {
    const user = userEvent.setup();
    const searchAnime = vi.fn().mockRejectedValue(offlineError);
    render(DiscoveryPage, discoveryProps({ searchAnime, debounceMs: 1000 }));

    await user.type(
      screen.getByRole("searchbox", { name: "Anime title" }),
      "Offline Title{Enter}",
    );

    const heading = await screen.findByText("Search could not finish");
    const notice = heading.closest('[data-status="offline"]');
    expect(notice).not.toBeNull();
    expect(notice).toHaveTextContent(offlineError.message);
    await user.click(
      within(notice as HTMLElement).getByRole("button", {
        name: "Retry search",
      }),
    );
    expect(searchAnime).toHaveBeenCalledTimes(2);
  });

  it("terminates a non-retryable search failure as an error", async () => {
    const user = userEvent.setup();
    const invalidResponseError: CommandError = {
      code: "ANILIST_RESPONSE_INVALID",
      message: "AniList returned metadata Animi could not safely use.",
      retryable: false,
    };
    const searchAnime = vi.fn().mockRejectedValue(invalidResponseError);
    render(DiscoveryPage, discoveryProps({ searchAnime, debounceMs: 1000 }));

    await user.type(
      screen.getByRole("searchbox", { name: "Anime title" }),
      "Malformed Title{Enter}",
    );

    const heading = await screen.findByText("Search could not finish");
    const notice = heading.closest('[data-status="error"]');
    expect(notice).not.toBeNull();
    expect(notice).toHaveTextContent(invalidResponseError.message);
    expect(
      within(notice as HTMLElement).queryByRole("button", {
        name: "Retry search",
      }),
    ).not.toBeInTheDocument();
  });

  it("keeps stale search data visible with explicit freshness", async () => {
    const user = userEvent.setup();
    const staleFreshness: Freshness = {
      ...currentFreshness,
      isStale: true,
      refreshError: offlineError,
    };
    const searchAnime = vi
      .fn()
      .mockResolvedValue(pageResponse([orbitalCats], staleFreshness));
    render(DiscoveryPage, discoveryProps({ searchAnime, debounceMs: 1000 }));

    await user.type(
      screen.getByRole("searchbox", { name: "Anime title" }),
      "Orbital{Enter}",
    );

    const staleHeading = await screen.findByText(
      "Showing cached search results",
    );
    expect(staleHeading.closest('[data-status="offline"]')).not.toBeNull();
    expect(
      screen.getByRole("button", { name: "Open details for Orbital Cats" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/the latest refresh did not replace them/i),
    ).toBeInTheDocument();
  });

  it("fetches details secondarily and restores focus when they close", async () => {
    const user = userEvent.setup();
    const loadDetails = vi
      .fn()
      .mockResolvedValue(detailsResponse(violetEvergreen));
    render(DiscoveryPage, discoveryProps({ loadDetails }));

    const opener = await screen.findByRole("button", {
      name: "Open details for Violet Evergreen",
    });
    await user.click(opener);

    const dialog = await screen.findByRole("dialog", {
      name: "Violet Evergreen",
    });
    expect(dialog).toHaveAttribute("open");
    expect(loadDetails).toHaveBeenCalledWith({ anilistId: 101 });
    const closeButton = within(dialog).getByRole("button", {
      name: "Close details",
    });
    await waitFor(() => expect(closeButton).toHaveFocus());
    await user.click(closeButton);

    await waitFor(() => expect(dialog).not.toBeInTheDocument());
    expect(document.documentElement).not.toHaveClass("has-modal");
    expect(opener).toHaveFocus();
  });

  it("updates card badges after child save and confirmed removal", async () => {
    const user = userEvent.setup();
    const saveEntry = vi.fn(async (request: UpsertLibraryEntryRequest) =>
      savedEntry(request),
    );
    const deleteEntry = vi.fn().mockResolvedValue({ removed: true });
    render(
      DiscoveryPage,
      discoveryProps({
        loadLibrary: vi.fn().mockResolvedValue([]),
        loadDetails: vi
          .fn()
          .mockResolvedValue(detailsResponse(violetEvergreen)),
        saveEntry,
        deleteEntry,
      }),
    );

    const opener = await screen.findByRole("button", {
      name: "Open details for Violet Evergreen",
    });
    expect(opener).toHaveTextContent("not tracked");
    await user.click(opener);

    const dialog = await screen.findByRole("dialog", {
      name: "Violet Evergreen",
    });
    await user.click(
      within(dialog).getByRole("button", { name: "Add to library" }),
    );
    await waitFor(() =>
      expect(opener).toHaveTextContent("tracked // planning"),
    );
    expect(
      within(dialog).getByRole("heading", { name: "Edit library entry" }),
    ).toBeInTheDocument();

    await user.click(
      within(dialog).getByRole("button", { name: "Remove from library" }),
    );
    await user.click(
      within(dialog).getByRole("button", { name: "Confirm local removal" }),
    );

    await waitFor(() => expect(opener).toHaveTextContent("not tracked"));
    expect(deleteEntry).toHaveBeenCalledWith({
      anilistId: violetEvergreen.anilistId,
      mediaKind: "ANIME",
      confirmed: true,
    });
    expect(
      within(dialog).getByRole("heading", { name: "Add to your library" }),
    ).toBeInTheDocument();
  });

  it("keeps metadata usable but blocks tracking when the library read fails", async () => {
    const user = userEvent.setup();
    const libraryError: CommandError = {
      code: "DATABASE_UNAVAILABLE",
      message: "The local library could not be read.",
      retryable: true,
    };
    render(
      DiscoveryPage,
      discoveryProps({ loadLibrary: vi.fn().mockRejectedValue(libraryError) }),
    );

    expect(
      await screen.findByText("Tracking controls are unavailable"),
    ).toBeInTheDocument();
    await user.click(
      await screen.findByRole("button", {
        name: "Open details for Violet Evergreen",
      }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "Violet Evergreen",
    });
    expect(within(dialog).getByText(/editing is paused/i)).toBeInTheDocument();
    expect(
      within(dialog).queryByRole("button", { name: "Add to library" }),
    ).not.toBeInTheDocument();
    expect(
      within(dialog).getByRole("button", { name: "Retry library" }),
    ).toBeInTheDocument();
  });
});
