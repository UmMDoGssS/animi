<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";

  import {
    getMediaDetails,
    getRecommendations,
    listLibrary,
    listTrending,
    normalizeCommandError,
    removeLibraryEntry,
    searchMedia,
    upsertLibraryEntry,
  } from "$lib/ipc/client";
  import type {
    CommandError,
    Freshness,
    LibraryEntry,
    LibraryItem,
    MediaDetailsRequest,
    MediaDetailsResponse,
    MediaPageRequest,
    MediaPageResponse,
    MediaSummary,
    Recommendation,
    RecommendationsRequest,
    RecommendationsResponse,
    RemoveLibraryEntryRequest,
    RemoveLibraryEntryResponse,
    SearchMediaRequest,
    UpsertLibraryEntryRequest,
  } from "$lib/ipc/types";

  import AsyncStateNotice from "../AsyncStateNotice.svelte";
  import MediaCover from "../MediaCover.svelte";
  import MediaDetailsDialog from "../MediaDetailsDialog.svelte";
  import PageHeader from "../PageHeader.svelte";

  interface Props {
    loadTrending?: (request: MediaPageRequest) => Promise<MediaPageResponse>;
    loadRecommendations?: (
      request: RecommendationsRequest,
    ) => Promise<RecommendationsResponse>;
    searchAnime?: (request: SearchMediaRequest) => Promise<MediaPageResponse>;
    loadDetails?: (
      request: MediaDetailsRequest,
    ) => Promise<MediaDetailsResponse>;
    loadLibrary?: () => Promise<LibraryItem[]>;
    saveEntry?: (request: UpsertLibraryEntryRequest) => Promise<LibraryEntry>;
    deleteEntry?: (
      request: RemoveLibraryEntryRequest,
    ) => Promise<RemoveLibraryEntryResponse>;
    debounceMs?: number;
  }

  let {
    loadTrending = listTrending,
    loadRecommendations = getRecommendations,
    searchAnime = searchMedia,
    loadDetails = getMediaDetails,
    loadLibrary = listLibrary,
    saveEntry = upsertLibraryEntry,
    deleteEntry = removeLibraryEntry,
    debounceMs = 350,
  }: Props = $props();

  const collectionRequest = { page: 1, perPage: 12 } as const;

  let query = $state("");
  let searchTerm = $state("");
  let searchResponse = $state<MediaPageResponse | null>(null);
  let searchError = $state<CommandError | null>(null);
  let searchLoading = $state(false);
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let searchRequestVersion = 0;

  let trendingResponse = $state<MediaPageResponse | null>(null);
  let trendingError = $state<CommandError | null>(null);
  let trendingLoading = $state(true);

  let recommendationsResponse = $state<RecommendationsResponse | null>(null);
  let recommendationsError = $state<CommandError | null>(null);
  let recommendationsLoading = $state(true);

  let libraryItems = $state<LibraryItem[]>([]);
  let libraryError = $state<CommandError | null>(null);
  let libraryLoading = $state(true);

  let searchInput = $state<HTMLInputElement | null>(null);
  let selectedSummary = $state<MediaSummary | null>(null);
  let selectedFreshness = $state<Freshness | null>(null);
  let detailsOpener: HTMLButtonElement | null = null;

  const trackingIssue = $derived<CommandError | null>(
    libraryError ??
      (libraryLoading
        ? {
            code: "LIBRARY_LOADING",
            message: "Animi is still reading the local library.",
            retryable: false,
          }
        : null),
  );

  onMount(() => {
    void fetchTrending();
    void fetchRecommendations();
    void fetchLibrary();
  });

  onDestroy(() => {
    cancelSearchTimer();
    searchRequestVersion += 1;
  });

  async function fetchTrending(): Promise<void> {
    trendingLoading = true;
    trendingError = null;

    try {
      trendingResponse = await loadTrending(collectionRequest);
    } catch (reason) {
      trendingError = normalizeCommandError(reason);
    } finally {
      trendingLoading = false;
    }
  }

  async function fetchRecommendations(): Promise<void> {
    recommendationsLoading = true;
    recommendationsError = null;
    try {
      recommendationsResponse = await loadRecommendations({ perPage: 18 });
    } catch (reason) {
      recommendationsError = normalizeCommandError(reason);
    } finally {
      recommendationsLoading = false;
    }
  }

  async function fetchLibrary(): Promise<void> {
    libraryLoading = true;
    libraryError = null;

    try {
      libraryItems = (await loadLibrary()).filter(
        (item) => item.entry.mediaKind === "ANIME",
      );
    } catch (reason) {
      libraryError = normalizeCommandError(reason);
    } finally {
      libraryLoading = false;
    }
  }

  function cancelSearchTimer(): void {
    if (searchTimer !== null) {
      clearTimeout(searchTimer);
      searchTimer = null;
    }
  }

  function invalidatePendingSearch(): void {
    cancelSearchTimer();
    searchRequestVersion += 1;
  }

  function resetSearchState(): void {
    invalidatePendingSearch();
    searchTerm = "";
    searchResponse = null;
    searchError = null;
    searchLoading = false;
  }

  function handleSearchInput(event: Event): void {
    query = (event.currentTarget as HTMLInputElement).value;
    const nextTerm = query.trim();
    invalidatePendingSearch();

    if (nextTerm.length === 0) {
      searchTerm = "";
      searchResponse = null;
      searchError = null;
      searchLoading = false;
      return;
    }

    searchTerm = nextTerm;
    searchResponse = null;
    searchError = null;
    searchLoading = true;
    searchTimer = setTimeout(() => {
      searchTimer = null;
      void executeSearch(nextTerm);
    }, debounceMs);
  }

  function submitSearch(event: SubmitEvent): void {
    event.preventDefault();
    const nextTerm = query.trim();
    if (nextTerm.length === 0) {
      resetSearchState();
      return;
    }

    cancelSearchTimer();
    searchTerm = nextTerm;
    searchLoading = true;
    void executeSearch(nextTerm);
  }

  async function executeSearch(term: string): Promise<void> {
    const requestVersion = ++searchRequestVersion;
    searchLoading = true;
    searchError = null;

    try {
      const response = await searchAnime({
        search: term,
        ...collectionRequest,
      });
      if (requestVersion !== searchRequestVersion) {
        return;
      }
      searchResponse = response;
    } catch (reason) {
      if (requestVersion !== searchRequestVersion) {
        return;
      }
      searchError = normalizeCommandError(reason);
    } finally {
      if (requestVersion === searchRequestVersion) {
        searchLoading = false;
      }
    }
  }

  function clearSearch(): void {
    query = "";
    resetSearchState();
    void tick().then(() => searchInput?.focus());
  }

  function entryFor(anilistId: number): LibraryEntry | null {
    return (
      libraryItems.find((item) => item.entry.anilistId === anilistId)?.entry ??
      null
    );
  }

  function openDetails(
    summary: MediaSummary,
    freshness: Freshness,
    event: MouseEvent,
  ): void {
    if (!(event.currentTarget instanceof HTMLButtonElement)) {
      return;
    }
    detailsOpener = event.currentTarget;
    selectedSummary = summary;
    selectedFreshness = freshness;
  }

  function closeDetails(): void {
    selectedSummary = null;
    selectedFreshness = null;
    const focusTarget =
      detailsOpener?.isConnected === true ? detailsOpener : searchInput;
    detailsOpener = null;
    void tick().then(() => focusTarget?.focus());
  }

  function handleLibraryChanged(entry: LibraryEntry | null): void {
    const anilistId = selectedSummary?.anilistId;
    if (anilistId === undefined) {
      return;
    }

    if (entry === null) {
      libraryItems = libraryItems.filter(
        (item) => item.entry.anilistId !== anilistId,
      );
      void fetchRecommendations();
      return;
    }

    const existingIndex = libraryItems.findIndex(
      (item) => item.entry.anilistId === entry.anilistId,
    );
    if (existingIndex === -1) {
      libraryItems = [...libraryItems, { entry, media: null }];
      void fetchRecommendations();
      return;
    }

    libraryItems = libraryItems.map((item, index) =>
      index === existingIndex ? { ...item, entry } : item,
    );
    void fetchRecommendations();
  }

  function formatLabel(value: string | null): string {
    return value === null
      ? "format unknown"
      : value.toLocaleLowerCase().replaceAll("_", " ");
  }

  function episodeLabel(episodes: number | null): string {
    return episodes === null
      ? "episode total unknown"
      : `${episodes} ${episodes === 1 ? "episode" : "episodes"}`;
  }

  function genreLabel(genres: string[]): string {
    return genres.length === 0
      ? "genres unavailable"
      : genres.slice(0, 2).join(" · ");
  }

  function formatTimestamp(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return value;
    }
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(date);
  }

  function isOfflineError(error: CommandError | null): boolean {
    return error?.code.toLocaleUpperCase().includes("OFFLINE") ?? false;
  }
</script>

<div
  id="discovery-panel"
  role="tabpanel"
  aria-labelledby="discovery-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <PageHeader
    eyebrow="catalog // find your next signal"
    title="Discover"
    description="Search AniList, browse what is trending, and keep your private library on this device."
  />

  {#if libraryError}
    <AsyncStateNotice
      status="error"
      title="Tracking controls are unavailable"
      description="Browsing still works, but Animi could not safely determine what is already in your local library."
      error={libraryError}
      onRetry={libraryError.retryable ? fetchLibrary : undefined}
      retryLabel="Retry library"
      compact
    />
  {/if}

  <section
    aria-labelledby="discover-search-heading"
    class="glass-panel panel-large border-border grid min-w-0 gap-4 border p-4 sm:p-6"
  >
    <div class="flex flex-wrap items-end justify-between gap-3">
      <div>
        <p class="text-accent text-[0.68rem] tracking-[0.14em] uppercase">
          search://anime
        </p>
        <h2
          id="discover-search-heading"
          class="text-foreground mt-1 text-lg font-light tracking-[0.03em] sm:text-xl"
        >
          Find an anime by title
        </h2>
      </div>
      <span
        class="control-rounded border-border bg-glass-control text-muted border px-2 py-1 text-[0.62rem] tracking-[0.12em] uppercase"
      >
        AniList metadata
      </span>
    </div>

    <form
      role="search"
      aria-label="Search anime"
      class="grid min-w-0 gap-2 sm:grid-cols-[minmax(0,1fr)_auto]"
      onsubmit={submitSearch}
    >
      <div class="min-w-0">
        <label
          for="discovery-search"
          class="text-muted mb-2 block text-xs tracking-[0.08em]"
        >
          Anime title
        </label>
        <div class="relative min-w-0">
          <span
            aria-hidden="true"
            class="text-accent pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-sm"
          >
            &gt;_
          </span>
          <input
            bind:this={searchInput}
            id="discovery-search"
            type="search"
            value={query}
            autocomplete="off"
            maxlength="200"
            placeholder="Try Frieren or Cowboy Bebop"
            class="glass-control control-rounded border-border text-foreground placeholder:text-muted min-h-11 w-full min-w-0 border py-2 pr-11 pl-10 text-sm outline-none"
            oninput={handleSearchInput}
          />
          {#if query.length > 0}
            <button
              type="button"
              aria-label="Clear search"
              class="glass-interactive control-rounded text-muted absolute top-1/2 right-0.5 grid size-10 -translate-y-1/2 place-items-center transition-colors"
              onclick={clearSearch}
            >
              <span aria-hidden="true">&times;</span>
            </button>
          {/if}
        </div>
      </div>

      <button
        type="submit"
        class="glass-accent control-rounded min-h-11 self-end px-5 py-2.5 text-xs tracking-[0.08em]"
      >
        Search now
      </button>
    </form>

    <div class="border-border border-t pt-3">
      <p class="text-muted text-xs leading-5">
        Results update after a short pause. Press Enter to search immediately.
        Opening a title saves only its useful metadata snapshot; browsing alone
        does not add it to your library.
      </p>
    </div>
  </section>

  {#snippet mediaGrid(
    items: MediaSummary[],
    label: string,
    freshness: Freshness,
  )}
    <ul
      aria-label={label}
      class="grid min-w-0 grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 lg:gap-4 xl:grid-cols-6"
    >
      {#each items as media (media.anilistId)}
        {@const entry = entryFor(media.anilistId)}
        <li class="min-w-0">
          <button
            type="button"
            aria-haspopup="dialog"
            aria-label={`Open details for ${media.title}`}
            class="discovery-card glass-control glass-interactive control-rounded border-border group flex h-full w-full min-w-0 flex-col border p-2 text-left transition-[transform,background-color] hover:-translate-y-0.5 focus-visible:-translate-y-0.5"
            onclick={(event) => openDetails(media, freshness, event)}
          >
            <MediaCover title={media.title} coverUrl={media.coverUrl} />

            <span
              class="flex min-h-32 w-full min-w-0 flex-1 flex-col px-1 pt-3 pb-2"
            >
              <span
                class="text-muted text-[0.64rem] tracking-[0.08em] uppercase"
              >
                {formatLabel(media.mediaFormat)} // {episodeLabel(
                  media.episodes,
                )}
              </span>
              <span
                class="text-foreground mt-1.5 line-clamp-2 text-sm leading-5 font-light tracking-[0.02em]"
              >
                {media.title}
              </span>
              <span class="text-muted mt-2 line-clamp-1 text-xs">
                {genreLabel(media.genres)}
              </span>
              <span
                class="border-border mt-auto border-t pt-2 text-[0.62rem] tracking-[0.07em] uppercase"
                class:text-accent={entry !== null}
                class:text-muted={entry === null}
              >
                {#if libraryLoading}
                  checking local library
                {:else if libraryError}
                  tracking unavailable
                {:else if entry}
                  tracked // {entry.status.toLocaleLowerCase()}
                {:else}
                  not tracked
                {/if}
              </span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/snippet}

  {#snippet recommendationGrid(
    recommendations: Recommendation[],
    freshness: Freshness,
    fallback: boolean,
  )}
    <ul
      aria-label="Recommended anime"
      class="grid min-w-0 gap-3 sm:grid-cols-2 lg:grid-cols-3"
    >
      {#each recommendations.slice(0, 6) as recommendation, index (recommendation.media.anilistId)}
        {@const media = recommendation.media}
        {@const entry = entryFor(media.anilistId)}
        <li class="min-w-0">
          <button
            type="button"
            aria-haspopup="dialog"
            aria-label={`Open details for ${media.title}`}
            class="discovery-card glass-control glass-interactive control-rounded border-border group grid h-full w-full min-w-0 grid-cols-[5.25rem_minmax(0,1fr)] gap-3 border p-2 text-left transition-[transform,background-color] hover:-translate-y-0.5 focus-visible:-translate-y-0.5"
            onclick={(event) => openDetails(media, freshness, event)}
          >
            <MediaCover title={media.title} coverUrl={media.coverUrl} />
            <span class="flex min-w-0 flex-col py-1 pr-1">
              <span
                class="text-muted text-[0.62rem] tracking-[0.1em] uppercase"
              >
                signal {String(index + 1).padStart(2, "0")} // {formatLabel(
                  media.mediaFormat,
                )}
              </span>
              <span
                class="text-foreground mt-1.5 line-clamp-2 text-sm leading-5 font-light"
              >
                {media.title}
              </span>
              {#if fallback}
                <span class="text-muted mt-2 text-xs leading-5">
                  Trending while your local taste profile grows.
                </span>
              {:else if recommendation.explanationLabels.length > 0}
                <span class="text-muted mt-2 text-xs leading-5">
                  Matches {recommendation.explanationLabels.join(" · ")}
                </span>
              {:else}
                <span class="text-muted mt-2 text-xs leading-5">
                  Ranked from local genre and trending signals.
                </span>
              {/if}
              <span
                class="border-border mt-auto border-t pt-2 text-[0.62rem] tracking-[0.07em] uppercase"
                class:text-accent={entry !== null}
                class:text-muted={entry === null}
              >
                {entry
                  ? `tracked // ${entry.status.toLocaleLowerCase()}`
                  : "not tracked"}
              </span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/snippet}

  {#if searchTerm.length > 0}
    <section
      aria-labelledby="search-results-heading"
      class="glass-panel panel-large border-border grid min-w-0 gap-4 border p-4 sm:p-6"
    >
      <header class="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p class="text-accent text-[0.66rem] tracking-[0.14em] uppercase">
            query // {searchTerm}
          </p>
          <h2
            id="search-results-heading"
            class="text-foreground mt-1 text-xl font-light tracking-[0.03em]"
          >
            Search results
          </h2>
        </div>
        {#if searchResponse}
          <p class="text-muted text-xs" aria-live="polite">
            {searchResponse.items.length}
            {searchResponse.items.length === 1 ? "title" : "titles"} // cached
            {formatTimestamp(searchResponse.freshness.fetchedAt)}
          </p>
        {/if}
      </header>

      {#if searchLoading}
        <AsyncStateNotice
          status="loading"
          title={`Searching for "${searchTerm}"`}
          description="Animi is checking its cache and AniList when needed."
          compact
        />
      {/if}

      {#if searchError}
        <AsyncStateNotice
          status={searchResponse
            ? isOfflineError(searchError)
              ? "offline"
              : "stale"
            : isOfflineError(searchError)
              ? "offline"
              : "error"}
          title={searchResponse
            ? "Keeping the last search results"
            : "Search could not finish"}
          description={searchResponse
            ? "The latest search failed, so the previous results remain visible."
            : searchError.message}
          error={searchError}
          onRetry={searchError.retryable
            ? () => executeSearch(searchTerm)
            : undefined}
          retryLabel="Retry search"
          compact
        />
      {:else if searchResponse?.freshness.isStale}
        <AsyncStateNotice
          status={isOfflineError(searchResponse.freshness.refreshError)
            ? "offline"
            : "stale"}
          title="Showing cached search results"
          description={`Saved ${formatTimestamp(searchResponse.freshness.fetchedAt)}. The latest refresh did not replace them.`}
          error={searchResponse.freshness.refreshError}
          onRetry={searchResponse.freshness.refreshError?.retryable
            ? () => executeSearch(searchTerm)
            : undefined}
          retryLabel="Retry search"
          compact
        />
      {/if}

      {#if searchResponse}
        {#if searchResponse.items.length > 0}
          {@render mediaGrid(
            searchResponse.items,
            `Anime search results for ${searchTerm}`,
            searchResponse.freshness,
          )}
        {:else}
          <AsyncStateNotice
            status="empty"
            title={`No anime found for "${searchTerm}"`}
            description="Check the title spelling or try a shorter title."
            compact
          />
        {/if}
      {/if}
    </section>
  {:else}
    <section
      aria-labelledby="recommendations-heading"
      class="glass-panel panel-large border-border grid min-w-0 gap-4 border p-4 sm:p-6"
    >
      <header class="flex flex-wrap items-end justify-between gap-3">
        <div>
          <p class="text-accent text-[0.66rem] tracking-[0.14em] uppercase">
            local signal // explainable picks
          </p>
          <h2
            id="recommendations-heading"
            class="text-foreground mt-1 text-xl font-light tracking-[0.03em]"
          >
            For your next watch
          </h2>
        </div>
        {#if recommendationsResponse}
          <p class="text-muted max-w-sm text-xs leading-5 sm:text-right">
            {recommendationsResponse.isFallback
              ? `Trending fallback // ${recommendationsResponse.eligibleProfileTitles} eligible local titles`
              : `Personalized from ${recommendationsResponse.eligibleProfileTitles} local titles`}
          </p>
        {/if}
      </header>

      {#if recommendationsLoading}
        <AsyncStateNotice
          status="loading"
          title="Building local recommendations"
          description="Animi is comparing cached tags and genres on this device."
          compact
        />
      {/if}

      {#if recommendationsError}
        <AsyncStateNotice
          status={recommendationsResponse
            ? isOfflineError(recommendationsError)
              ? "offline"
              : "stale"
            : isOfflineError(recommendationsError)
              ? "offline"
              : "error"}
          title={recommendationsResponse
            ? "Keeping the saved recommendation set"
            : "Recommendations could not load"}
          description={recommendationsResponse
            ? "The previous explainable results remain visible."
            : "Search and the separate trending catalog remain available."}
          error={recommendationsError}
          onRetry={recommendationsError.retryable
            ? fetchRecommendations
            : undefined}
          retryLabel="Retry recommendations"
          compact
        />
      {:else if recommendationsResponse?.freshness.isStale}
        <AsyncStateNotice
          status={isOfflineError(recommendationsResponse.freshness.refreshError)
            ? "offline"
            : "stale"}
          title="Showing cached recommendations"
          description={`Candidate data saved ${formatTimestamp(recommendationsResponse.freshness.fetchedAt)}.`}
          error={recommendationsResponse.freshness.refreshError}
          onRetry={recommendationsResponse.freshness.refreshError?.retryable
            ? fetchRecommendations
            : undefined}
          retryLabel="Retry recommendations"
          compact
        />
      {/if}

      {#if recommendationsResponse}
        {#if recommendationsResponse.items.length > 0}
          {@render recommendationGrid(
            recommendationsResponse.items,
            recommendationsResponse.freshness,
            recommendationsResponse.isFallback,
          )}
        {:else}
          <AsyncStateNotice
            status="empty"
            title="No untracked candidates are available"
            description="Every cached candidate is already in your library. Check again when trending titles change."
            compact
          />
        {/if}
      {/if}
    </section>

    <section
      aria-labelledby="trending-heading"
      class="glass-panel panel-large border-border grid min-w-0 gap-4 border p-4 sm:p-6"
    >
      <header class="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p class="text-accent text-[0.66rem] tracking-[0.14em] uppercase">
            catalog // trending now
          </p>
          <h2
            id="trending-heading"
            class="text-foreground mt-1 text-xl font-light tracking-[0.03em]"
          >
            Trending anime
          </h2>
        </div>
        {#if trendingResponse}
          <p class="text-muted text-xs">
            {trendingResponse.items.length}
            {trendingResponse.items.length === 1 ? "title" : "titles"} // cached
            {formatTimestamp(trendingResponse.freshness.fetchedAt)}
          </p>
        {/if}
      </header>

      {#if trendingLoading}
        <AsyncStateNotice
          status="loading"
          title="Loading trending anime"
          description="Animi is checking its discovery cache and AniList when needed."
          compact
        />
      {/if}

      {#if trendingError}
        <AsyncStateNotice
          status={trendingResponse
            ? isOfflineError(trendingError)
              ? "offline"
              : "stale"
            : isOfflineError(trendingError)
              ? "offline"
              : "error"}
          title={trendingResponse
            ? "Keeping the saved trending list"
            : "Trending anime could not load"}
          description={trendingResponse
            ? "The latest refresh failed, so the previous trending list remains visible."
            : trendingError.message}
          error={trendingError}
          onRetry={trendingError.retryable ? fetchTrending : undefined}
          retryLabel="Retry trending"
          compact
        />
      {:else if trendingResponse?.freshness.isStale}
        <AsyncStateNotice
          status={isOfflineError(trendingResponse.freshness.refreshError)
            ? "offline"
            : "stale"}
          title="Showing cached trending anime"
          description={`Saved ${formatTimestamp(trendingResponse.freshness.fetchedAt)}. The latest refresh did not replace this list.`}
          error={trendingResponse.freshness.refreshError}
          onRetry={trendingResponse.freshness.refreshError?.retryable
            ? fetchTrending
            : undefined}
          retryLabel="Retry trending"
          compact
        />
      {/if}

      {#if trendingResponse}
        {#if trendingResponse.items.length > 0}
          {@render mediaGrid(
            trendingResponse.items,
            "Trending anime",
            trendingResponse.freshness,
          )}
        {:else}
          <AsyncStateNotice
            status="empty"
            title="No trending anime are available"
            description="Try again later or search for a title directly."
            compact
          />
        {/if}
      {/if}
    </section>
  {/if}

  {#if selectedSummary}
    <MediaDetailsDialog
      summary={selectedSummary}
      entry={entryFor(selectedSummary.anilistId)}
      fallbackFreshness={selectedFreshness}
      {loadDetails}
      {saveEntry}
      {deleteEntry}
      trackingError={trackingIssue}
      onRetryTracking={fetchLibrary}
      onLibraryChanged={handleLibraryChanged}
      onClose={closeDetails}
    />
  {/if}
</div>

<style>
  .discovery-card:active {
    transform: translateY(1px) scale(0.985);
  }

  @media (prefers-reduced-motion: reduce) {
    .discovery-card {
      transition: none;
    }
  }
</style>
