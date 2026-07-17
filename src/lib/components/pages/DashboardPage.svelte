<script lang="ts">
  import { onMount, tick } from "svelte";

  import {
    getDashboardInsights,
    getMediaDetails,
    normalizeCommandError,
    refreshLibraryMetadata,
    removeLibraryEntry,
    upsertLibraryEntry,
  } from "$lib/ipc/client";
  import type {
    CommandError,
    DashboardActivityBucket,
    DashboardInsights,
    Freshness,
    LibraryEntry,
    LibraryItem,
    LibraryStatus,
    MediaDetailsRequest,
    MediaDetailsResponse,
    MediaSummary,
    RefreshLibraryMetadataResponse,
    RemoveLibraryEntryRequest,
    RemoveLibraryEntryResponse,
    UpsertLibraryEntryRequest,
    WatchActivity,
  } from "$lib/ipc/types";

  import AsyncStateNotice from "../AsyncStateNotice.svelte";
  import GlassSelect from "../GlassSelect.svelte";
  import MediaCover from "../MediaCover.svelte";
  import MediaDetailsDialog from "../MediaDetailsDialog.svelte";

  interface Props {
    loadInsights?: () => Promise<DashboardInsights>;
    refreshMetadata?: () => Promise<RefreshLibraryMetadataResponse>;
    loadDetails?: (
      request: MediaDetailsRequest,
    ) => Promise<MediaDetailsResponse>;
    saveEntry?: (request: UpsertLibraryEntryRequest) => Promise<LibraryEntry>;
    deleteEntry?: (
      request: RemoveLibraryEntryRequest,
    ) => Promise<RemoveLibraryEntryResponse>;
    openDiscovery?: () => void;
  }

  type LibraryStatusFilter = "ALL" | LibraryStatus;
  type LibrarySort =
    | "RELEASE_DESC"
    | "UPDATED_DESC"
    | "TITLE_ASC"
    | "PROGRESS_DESC"
    | "SCORE_DESC";

  const libraryStatusOptions = [
    { value: "ALL", label: "All statuses" },
    { value: "WATCHING", label: "Watching" },
    { value: "COMPLETED", label: "Completed" },
    { value: "PLANNING", label: "Planning" },
  ] as const;
  const librarySortOptions = [
    { value: "RELEASE_DESC", label: "Newest release" },
    { value: "UPDATED_DESC", label: "Recently updated" },
    { value: "TITLE_ASC", label: "Title A–Z" },
    { value: "PROGRESS_DESC", label: "Most progress" },
    { value: "SCORE_DESC", label: "Highest score" },
  ] as const;

  let {
    loadInsights = getDashboardInsights,
    refreshMetadata = refreshLibraryMetadata,
    loadDetails = getMediaDetails,
    saveEntry = upsertLibraryEntry,
    deleteEntry = removeLibraryEntry,
    openDiscovery,
  }: Props = $props();

  let insights = $state<DashboardInsights | null>(null);
  let loading = $state(true);
  let loadError = $state<CommandError | null>(null);
  let refreshing = $state(false);
  let refreshResult = $state<RefreshLibraryMetadataResponse | null>(null);
  let refreshError = $state<CommandError | null>(null);
  let selectedItem = $state<LibraryItem | null>(null);
  let detailsOpener: HTMLButtonElement | null = null;
  let libraryIndexHeading = $state<HTMLElement | null>(null);
  let libraryQuery = $state("");
  let libraryStatusFilter = $state<LibraryStatusFilter>("ALL");
  let librarySort = $state<LibrarySort>("RELEASE_DESC");
  let libraryRail = $state<HTMLElement | null>(null);
  let releaseDateBackfillChecked = false;

  const selectedSummary = $derived.by<MediaSummary | null>(() => {
    if (selectedItem === null) {
      return null;
    }
    return summaryForItem(selectedItem);
  });

  const filteredLibraryItems = $derived.by<LibraryItem[]>(() => {
    const normalizedQuery = libraryQuery.trim().toLocaleLowerCase();
    const matchingItems = (insights?.libraryItems ?? []).filter((item) => {
      const matchesStatus =
        libraryStatusFilter === "ALL" ||
        item.entry.status === libraryStatusFilter;
      const matchesQuery =
        normalizedQuery.length === 0 ||
        titleForItem(item).toLocaleLowerCase().includes(normalizedQuery) ||
        item.entry.anilistId.toString().includes(normalizedQuery);
      return matchesStatus && matchesQuery;
    });
    return [...matchingItems].sort(compareLibraryItems);
  });

  const libraryFiltersActive = $derived(
    libraryQuery.trim().length > 0 || libraryStatusFilter !== "ALL",
  );

  onMount(() => {
    void loadDashboard();
  });

  async function loadDashboard(): Promise<void> {
    if (insights === null) {
      loading = true;
    }
    loadError = null;

    try {
      insights = await loadInsights();
      if (
        !releaseDateBackfillChecked &&
        insights.libraryItems.some(
          (item) =>
            item.media !== null &&
            item.media.releaseDate === null &&
            item.media.status !== "NOT_YET_RELEASED",
        )
      ) {
        releaseDateBackfillChecked = true;
        void checkTrackedMetadata();
      }
    } catch (reason) {
      loadError = normalizeCommandError(reason);
    } finally {
      loading = false;
    }
  }

  async function checkTrackedMetadata(): Promise<void> {
    refreshing = true;
    refreshResult = null;
    refreshError = null;

    try {
      refreshResult = await refreshMetadata();
      await loadDashboard();
    } catch (reason) {
      refreshError = normalizeCommandError(reason);
    } finally {
      refreshing = false;
    }
  }

  function titleForItem(item: LibraryItem): string {
    return item.media?.title ?? `AniList #${item.entry.anilistId}`;
  }

  function compareLibraryItems(left: LibraryItem, right: LibraryItem): number {
    if (librarySort === "RELEASE_DESC") {
      const leftRelease = left.media?.releaseDate ?? null;
      const rightRelease = right.media?.releaseDate ?? null;
      if (leftRelease !== rightRelease) {
        if (leftRelease === null) return 1;
        if (rightRelease === null) return -1;
        return (
          rightRelease.localeCompare(leftRelease) ||
          compareLibraryTitles(left, right)
        );
      }
      return compareLibraryTitles(left, right);
    }
    if (librarySort === "TITLE_ASC") {
      return compareLibraryTitles(left, right);
    }
    if (librarySort === "PROGRESS_DESC") {
      return (
        right.entry.progress - left.entry.progress ||
        compareLibraryTitles(left, right)
      );
    }
    if (librarySort === "SCORE_DESC") {
      return (
        (right.entry.score ?? -1) - (left.entry.score ?? -1) ||
        compareLibraryTitles(left, right)
      );
    }
    return (
      right.entry.updatedAt.localeCompare(left.entry.updatedAt) ||
      compareLibraryTitles(left, right)
    );
  }

  function compareLibraryTitles(left: LibraryItem, right: LibraryItem): number {
    return (
      titleForItem(left).localeCompare(titleForItem(right), undefined, {
        sensitivity: "base",
        numeric: true,
      }) || left.entry.anilistId - right.entry.anilistId
    );
  }

  function changeLibraryStatusFilter(value: string): void {
    const option = libraryStatusOptions.find(
      (candidate) => candidate.value === value,
    );
    if (!option) return;
    libraryStatusFilter = option.value;
    resetLibraryScroll();
  }

  function changeLibrarySort(value: string): void {
    const option = librarySortOptions.find(
      (candidate) => candidate.value === value,
    );
    if (!option) return;
    librarySort = option.value;
    resetLibraryScroll();
  }

  function resetLibraryFilters(): void {
    libraryQuery = "";
    libraryStatusFilter = "ALL";
    resetLibraryScroll();
  }

  function resetLibraryScroll(): void {
    libraryRail?.scrollTo?.({ left: 0, behavior: "smooth" });
  }

  function summaryForItem(item: LibraryItem): MediaSummary {
    return {
      anilistId: item.entry.anilistId,
      title: titleForItem(item),
      coverUrl: item.media?.coverUrl ?? null,
      mediaFormat: item.media?.mediaFormat ?? null,
      status: item.media?.status ?? null,
      episodes: item.media?.episodes ?? null,
      durationMinutes: item.media?.durationMinutes ?? null,
      releaseDate: item.media?.releaseDate ?? null,
      genres: item.media?.genres ?? [],
      tags: item.media?.tags ?? [],
      trending: 0,
    };
  }

  function freshnessForItem(item: LibraryItem): Freshness | null {
    if (item.media === null) {
      return null;
    }
    return {
      fetchedAt: item.media.cachedAt,
      expiresAt: item.media.expiresAt,
      isStale: item.media.isStale,
      refreshError: null,
    };
  }

  function openItemDetails(item: LibraryItem, event: MouseEvent): void {
    if (!(event.currentTarget instanceof HTMLButtonElement)) {
      return;
    }
    detailsOpener = event.currentTarget;
    selectedItem = item;
  }

  function closeItemDetails(): void {
    const anilistId = selectedItem?.entry.anilistId ?? null;
    selectedItem = null;
    const focusTarget = detailsOpener;
    detailsOpener = null;
    void tick().then(() => {
      if (focusTarget?.isConnected) {
        focusTarget.focus();
        return;
      }
      if (anilistId !== null) {
        const movedCard = document.querySelector<HTMLButtonElement>(
          `[data-library-anilist-id="${anilistId}"]`,
        );
        if (movedCard !== null) {
          movedCard.focus();
          return;
        }
      }
      libraryIndexHeading?.focus();
    });
  }

  async function handleLibraryChanged(
    entry: LibraryEntry | null,
  ): Promise<void> {
    if (selectedItem === null || insights === null) {
      await loadDashboard();
      return;
    }

    const previousEntry = selectedItem.entry;
    if (entry === null) {
      optimisticallyRemoveEntry(previousEntry);
    } else {
      selectedItem = { ...selectedItem, entry };
      optimisticallyUpdateEntry(previousEntry, entry);
    }
    await loadDashboard();
  }

  async function handleDetailsLoaded(
    response: MediaDetailsResponse,
  ): Promise<void> {
    if (selectedItem === null || insights === null) {
      return;
    }
    const refreshedMedia = {
      title: response.media.title,
      coverUrl: response.media.coverUrl,
      mediaFormat: response.media.mediaFormat,
      genres: response.media.genres,
      tags: response.media.tags,
      status: response.media.status,
      episodes: response.media.episodes,
      durationMinutes: response.media.durationMinutes,
      releaseDate: response.media.releaseDate,
      cachedAt: response.freshness.fetchedAt,
      expiresAt: response.freshness.expiresAt,
      isStale: response.freshness.isStale,
    };
    selectedItem = { ...selectedItem, media: refreshedMedia };
    insights = {
      ...insights,
      libraryItems: insights.libraryItems.map((item) =>
        item.entry.anilistId === selectedItem?.entry.anilistId
          ? { ...item, media: refreshedMedia }
          : item,
      ),
    };
    await loadDashboard();
  }

  function optimisticallyUpdateEntry(
    previousEntry: LibraryEntry,
    entry: LibraryEntry,
  ): void {
    if (insights === null) {
      return;
    }
    const totals = { ...insights.libraryTotals };
    if (previousEntry.status !== entry.status) {
      totals[statusTotalKey(previousEntry.status)] -= 1;
      totals[statusTotalKey(entry.status)] += 1;
    }
    totals.currentProgress += entry.progress - previousEntry.progress;
    insights = {
      ...insights,
      libraryTotals: totals,
      libraryItems: insights.libraryItems.map((item) =>
        item.entry.anilistId === entry.anilistId ? { ...item, entry } : item,
      ),
    };
  }

  function optimisticallyRemoveEntry(entry: LibraryEntry): void {
    if (insights === null) {
      return;
    }
    const totals = { ...insights.libraryTotals };
    totals.tracked -= 1;
    totals[statusTotalKey(entry.status)] -= 1;
    totals.currentProgress -= entry.progress;
    insights = {
      ...insights,
      libraryTotals: totals,
      libraryItems: insights.libraryItems.filter(
        (item) => item.entry.anilistId !== entry.anilistId,
      ),
    };
  }

  function statusTotalKey(
    status: LibraryStatus,
  ): "watching" | "completed" | "planning" {
    if (status === "WATCHING") {
      return "watching";
    }
    if (status === "COMPLETED") {
      return "completed";
    }
    return "planning";
  }

  function titleForActivity(activity: WatchActivity): string {
    return (
      insights?.libraryItems.find(
        (item) => item.entry.anilistId === activity.anilistId,
      )?.media?.title ?? `AniList #${activity.anilistId}`
    );
  }

  function formatMediaFormat(value: string | null): string {
    return value === null
      ? "format unknown"
      : value.replaceAll("_", " ").toLocaleLowerCase();
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

  function formatDuration(minutes: number): string {
    const hours = Math.floor(minutes / 60);
    const remainingMinutes = minutes % 60;
    if (hours === 0) {
      return `${remainingMinutes}m`;
    }
    return remainingMinutes === 0
      ? `${hours}h`
      : `${hours}h ${remainingMinutes.toString().padStart(2, "0")}m`;
  }

  function progressLabel(item: LibraryItem): string {
    return `${item.entry.progress} / ${item.media?.episodes ?? "??"} eps`;
  }

  function activityDuration(activity: WatchActivity): string {
    if (activity.episodeDurationMinutes === null) {
      return "duration unknown";
    }
    return formatDuration(
      activity.progressDelta * activity.episodeDurationMinutes,
    );
  }

  function padCount(value: number): string {
    return value.toString().padStart(2, "0");
  }

  function countLabel(
    value: number,
    singular: string,
    plural = `${singular}s`,
  ): string {
    return `${value} ${value === 1 ? singular : plural}`;
  }

  function activityBarHeight(
    bucket: DashboardActivityBucket,
    buckets: DashboardActivityBucket[],
  ): number {
    const maximum = Math.max(
      1,
      ...buckets.map((candidate) => candidate.recordedMinutes),
    );
    if (bucket.recordedMinutes === 0) {
      return bucket.recordedEpisodes > 0 ? 8 : 2;
    }
    return Math.max(12, (bucket.recordedMinutes / maximum) * 100);
  }

  function formatMonthPeriod(period: string): string {
    const date = new Date(`${period}-01T00:00:00.000Z`);
    if (Number.isNaN(date.getTime())) return period;
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      timeZone: "UTC",
    }).format(date);
  }

  function formatMonthName(period: string): string {
    const date = new Date(`${period}-01T00:00:00.000Z`);
    if (Number.isNaN(date.getTime())) return period;
    return new Intl.DateTimeFormat(undefined, {
      month: "long",
      year: "numeric",
      timeZone: "UTC",
    }).format(date);
  }

  function monthlyBucketLabel(bucket: DashboardActivityBucket): string {
    const known = formatDuration(bucket.recordedMinutes);
    const excluded = countLabel(
      bucket.unknownDurationEpisodes,
      "episode with unknown duration",
      "episodes with unknown duration",
    );
    return `${formatMonthName(bucket.period)}: ${known} of recorded watch time from ${countLabel(bucket.recordedEpisodes, "episode")}; ${excluded}.`;
  }

  function bucketLabel(bucket: DashboardActivityBucket): string {
    const known = formatDuration(bucket.recordedMinutes);
    const excluded = countLabel(
      bucket.unknownDurationEpisodes,
      "episode with unknown duration",
      "episodes with unknown duration",
    );
    return `${bucket.period}: ${known} recorded from ${countLabel(bucket.recordedEpisodes, "episode")}; ${excluded}.`;
  }
</script>

<div
  id="dashboard-panel"
  role="tabpanel"
  aria-labelledby="dashboard-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <h1 class="sr-only">Dashboard</h1>

  {#if loading && insights === null}
    <AsyncStateNotice
      status="loading"
      title="Loading your dashboard"
      description="Gathering your library and watch history."
    />
  {:else if loadError && insights === null}
    <AsyncStateNotice
      status="error"
      title="Your dashboard could not load"
      error={loadError}
      onRetry={loadError.retryable ? loadDashboard : undefined}
      retryLabel="Retry Dashboard"
    />
  {:else if insights}
    <section
      aria-labelledby="library-manifest-heading"
      class="glass-panel panel-large border-border grid min-w-0 overflow-hidden border lg:grid-cols-[minmax(0,1.25fr)_minmax(17rem,0.75fr)]"
    >
      <div class="min-w-0 p-4 sm:p-6 lg:p-7">
        <div class="flex flex-wrap items-end justify-between gap-3">
          <div>
            <h2
              id="library-manifest-heading"
              class="text-foreground text-xl font-light tracking-[0.03em] sm:text-2xl"
            >
              {insights.libraryTotals.tracked === 0
                ? "Your shelves are quiet."
                : `${insights.libraryTotals.tracked} ${insights.libraryTotals.tracked === 1 ? "title" : "titles"}, filed by what comes next.`}
            </h2>
          </div>
          <button
            type="button"
            class="glass-control glass-interactive control-rounded min-h-10 px-3 py-2 text-xs transition-colors disabled:cursor-wait disabled:opacity-60"
            disabled={refreshing || insights.libraryTotals.tracked === 0}
            onclick={() => void checkTrackedMetadata()}
          >
            {refreshing ? "refreshing…" : "refresh anime details"}
          </button>
        </div>

        <dl
          role="group"
          aria-label="Library totals"
          class="border-border mt-6 grid grid-cols-2 border-y"
        >
          <div class="border-border border-r p-3 sm:p-4">
            <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
              tracked
            </dt>
            <dd
              class="text-stat-tracked mt-2 text-3xl font-light tracking-[0.06em] tabular-nums sm:text-5xl"
              data-stat="tracked"
            >
              {padCount(insights.libraryTotals.tracked)}
            </dd>
          </div>
          <div class="border-border p-3 sm:p-4">
            <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
              watching
            </dt>
            <dd
              class="text-stat-watching mt-2 text-3xl font-light tracking-[0.06em] tabular-nums sm:text-5xl"
              data-stat="watching"
            >
              {padCount(insights.libraryTotals.watching)}
            </dd>
          </div>
          <div class="border-border border-t border-r p-3 sm:p-4">
            <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
              completed
            </dt>
            <dd
              class="text-stat-completed mt-2 text-3xl font-light tracking-[0.06em] tabular-nums sm:text-5xl"
              data-stat="completed"
            >
              {padCount(insights.libraryTotals.completed)}
            </dd>
          </div>
          <div class="border-border border-t p-3 sm:p-4">
            <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
              planning
            </dt>
            <dd
              class="text-stat-planning mt-2 text-3xl font-light tracking-[0.06em] tabular-nums sm:text-5xl"
              data-stat="planning"
            >
              {padCount(insights.libraryTotals.planning)}
            </dd>
          </div>
        </dl>

        <p class="text-muted mt-4 text-xs leading-5">
          <span class="text-foreground tabular-nums">
            {insights.libraryTotals.currentProgress} episodes
          </span>
          currently marked watched.
        </p>

        {#if loadError}
          <div class="mt-4">
            <AsyncStateNotice
              status="error"
              title="The dashboard could not be refreshed"
              description="Your previous dashboard is still shown."
              error={loadError}
              onRetry={loadError.retryable ? loadDashboard : undefined}
              retryLabel="Retry reload"
              compact
            />
          </div>
        {/if}

        {#if refreshError}
          <div class="mt-4">
            <AsyncStateNotice
              status={refreshError.code.toLocaleUpperCase().includes("OFFLINE")
                ? "offline"
                : "error"}
              title="Anime details could not be refreshed"
              description="Your library is still available."
              error={refreshError}
              onRetry={refreshError.retryable
                ? checkTrackedMetadata
                : undefined}
              retryLabel="Check again"
              compact
            />
          </div>
        {:else if refreshResult}
          <p
            role="status"
            class="control-rounded border-border bg-glass-control mt-4 border px-3 py-2 text-xs leading-5"
            class:text-warning={refreshResult.failures.length > 0}
            class:text-success={refreshResult.failures.length === 0}
          >
            {#if refreshResult.failures.length > 0}
              Updated {countLabel(refreshResult.updated, "title")}; {countLabel(
                refreshResult.failures.length,
                "title",
              )} could not be refreshed.
            {:else}
              Anime details refreshed.
            {/if}
          </p>
        {/if}
      </div>

      <div
        class="border-border min-w-0 border-t p-4 sm:p-6 lg:border-t-0 lg:border-l lg:p-7"
      >
        <h2 class="text-foreground text-xl font-light tracking-[0.03em]">
          Your favorite genres and tags
        </h2>

        {#if insights.preferenceSignals.genres.length > 0}
          <div class="mt-5">
            <h3 class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
              favorite genres
            </h3>
            <ul aria-label="Genre preference signals" class="mt-3 grid gap-3">
              {#each insights.preferenceSignals.genres.slice(0, 5) as signal (signal.label)}
                <li>
                  <div class="flex items-center justify-between gap-3 text-xs">
                    <span class="text-foreground">{signal.label}</span>
                    <span class="text-muted tabular-nums">
                      {signal.titleCount}
                      {signal.titleCount === 1 ? "title" : "titles"}
                    </span>
                  </div>
                  <div
                    aria-hidden="true"
                    class="bg-glass-control mt-1.5 h-1.5 overflow-hidden rounded-sm"
                  >
                    <span
                      class="bg-accent block h-full rounded-sm"
                      style:width={`${(signal.titleCount / (insights.preferenceSignals.genres[0]?.titleCount ?? 1)) * 100}%`}
                    ></span>
                  </div>
                </li>
              {/each}
            </ul>
          </div>
        {:else}
          <p class="text-muted mt-5 text-xs leading-5">
            Genres will appear as you watch and complete anime.
          </p>
        {/if}

        {#if insights.preferenceSignals.tags.length > 0}
          <div class="mt-5">
            <h3 class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
              favorite tags
            </h3>
            <ul
              aria-label="Tag preference signals"
              class="mt-3 flex flex-wrap gap-2"
            >
              {#each insights.preferenceSignals.tags.slice(0, 6) as tag (tag.label)}
                <li
                  class="control-rounded border-border bg-glass-control text-foreground border px-2.5 py-1.5 text-[0.68rem]"
                >
                  <span class="block">{tag.label}</span>
                  <span class="text-muted mt-0.5 block text-[0.58rem]">
                    {countLabel(tag.titleCount, "title")}
                  </span>
                </li>
              {/each}
            </ul>
          </div>
        {:else}
          <p class="text-muted mt-5 text-xs leading-5">
            Tags will appear as you watch and complete anime.
          </p>
        {/if}
      </div>
    </section>

    <section
      aria-labelledby="activity-ledger-heading"
      class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-4 sm:p-6 lg:p-7"
    >
      <header
        class="border-border flex flex-col gap-3 border-b pb-5 sm:flex-row sm:items-end sm:justify-between"
      >
        <div>
          <h2
            id="activity-ledger-heading"
            class="text-foreground text-xl font-light tracking-[0.03em] sm:text-2xl"
          >
            Your watch history
          </h2>
        </div>
      </header>

      {#if insights.activity.eventCount === 0}
        <div
          class="grid min-h-56 items-center gap-5 py-6 sm:grid-cols-[10rem_minmax(0,1fr)]"
        >
          <div
            aria-hidden="true"
            class="glass-control control-rounded border-border text-accent mx-auto grid min-h-32 w-full max-w-40 place-items-center border p-4 text-center"
          >
            <div>
              <p class="text-lg">(=^..^=)</p>
              <p
                class="text-muted mt-3 text-[0.62rem] tracking-[0.12em] uppercase"
              >
                no history yet
              </p>
            </div>
          </div>
          <div>
            <h3 class="text-foreground text-lg font-light">
              Your watch history is empty.
            </h3>
            <p class="text-muted mt-2 text-sm leading-6">
              Watch an episode and update its progress to start your history.
            </p>
          </div>
        </div>
      {:else}
        <dl
          role="group"
          aria-label="Recorded activity summary"
          class="mt-5 grid grid-cols-2 gap-3 lg:grid-cols-4"
        >
          <div class="border-border border-l-2 pl-3">
            <dt class="text-muted text-[0.62rem] tracking-[0.1em] uppercase">
              updates
            </dt>
            <dd class="text-foreground mt-1 text-lg tabular-nums">
              {insights.activity.eventCount}
            </dd>
          </div>
          <div class="border-border border-l-2 pl-3">
            <dt class="text-muted text-[0.62rem] tracking-[0.1em] uppercase">
              recorded episodes
            </dt>
            <dd class="text-foreground mt-1 text-lg tabular-nums">
              {insights.activity.recordedEpisodes}
            </dd>
          </div>
          <div class="border-border border-l-2 pl-3">
            <dt class="text-muted text-[0.62rem] tracking-[0.1em] uppercase">
              known watch time
            </dt>
            <dd class="text-foreground mt-1 text-lg tabular-nums">
              {formatDuration(insights.activity.recordedMinutes)}
            </dd>
          </div>
          <div class="border-warning border-l-2 pl-3">
            <dt class="text-muted text-[0.62rem] tracking-[0.1em] uppercase">
              time unknown
            </dt>
            <dd class="text-warning mt-1 text-sm tabular-nums">
              {countLabel(insights.activity.unknownDurationEpisodes, "episode")}
            </dd>
          </div>
        </dl>

        <div
          class="mt-6 grid min-w-0 gap-4 lg:grid-cols-[minmax(0,1.6fr)_minmax(12rem,0.4fr)]"
        >
          <section
            aria-labelledby="monthly-activity-heading"
            class="glass-control control-rounded border-border min-w-0 border p-3 sm:p-4"
          >
            <div class="flex flex-wrap items-end justify-between gap-2">
              <div>
                <h3
                  id="monthly-activity-heading"
                  class="text-foreground text-base font-light"
                >
                  Time spent watching anime each month
                </h3>
                <p id="monthly-activity-description" class="sr-only">
                  Each bar totals known watch time recorded during one calendar
                  month.
                </p>
              </div>
            </div>
            <div class="mt-4 min-w-0 overflow-x-auto pb-1">
              <ol
                aria-label="Monthly recorded watch activity"
                aria-describedby="monthly-activity-description"
                class="grid min-w-[36rem] grid-cols-12 gap-2"
              >
                {#each insights.activity.monthly as bucket (bucket.period)}
                  <li
                    aria-label={monthlyBucketLabel(bucket)}
                    class="grid min-w-0 gap-2"
                  >
                    <div
                      aria-hidden="true"
                      class="bg-glass-control flex h-28 items-end overflow-hidden rounded-sm"
                    >
                      <span
                        class="bg-accent block w-full rounded-t-sm opacity-80"
                        class:bg-warning={bucket.recordedMinutes === 0 &&
                          bucket.unknownDurationEpisodes > 0}
                        style:height={`${activityBarHeight(bucket, insights.activity.monthly)}%`}
                      ></span>
                    </div>
                    <div aria-hidden="true" class="text-center">
                      <p class="text-foreground text-[0.62rem] tabular-nums">
                        {bucket.recordedMinutes > 0
                          ? formatDuration(bucket.recordedMinutes)
                          : bucket.unknownDurationEpisodes > 0
                            ? "?"
                            : "—"}
                      </p>
                      <time
                        datetime={`${bucket.period}-01`}
                        aria-label={formatMonthName(bucket.period)}
                        class="text-muted mt-1 block text-[0.55rem] uppercase"
                      >
                        {formatMonthPeriod(bucket.period)}
                      </time>
                    </div>
                  </li>
                {/each}
              </ol>
            </div>
          </section>

          <section
            aria-labelledby="yearly-activity-heading"
            class="glass-control control-rounded border-border border p-3 sm:p-4"
          >
            <h3
              id="yearly-activity-heading"
              class="text-foreground text-base font-light"
            >
              Yearly totals
            </h3>
            <ul
              aria-label="Yearly recorded watch activity"
              class="border-border mt-4 grid divide-y border-y"
            >
              {#each insights.activity.yearly as bucket (bucket.period)}
                <li
                  aria-label={bucketLabel(bucket)}
                  class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-3 py-3"
                >
                  <span class="text-accent text-lg font-light tabular-nums"
                    >{bucket.period}</span
                  >
                  <span class="text-muted text-right text-[0.62rem] leading-4">
                    <span class="text-foreground block text-sm"
                      >{formatDuration(bucket.recordedMinutes)}</span
                    >
                    {countLabel(bucket.recordedEpisodes, "episode")}
                    {#if bucket.unknownDurationEpisodes > 0}
                      <span class="text-warning block"
                        >{bucket.unknownDurationEpisodes} duration excluded</span
                      >
                    {/if}
                  </span>
                </li>
              {/each}
            </ul>
          </section>
        </div>

        <ol
          aria-label="Recent recorded progress events"
          class="mt-5 grid gap-2"
        >
          {#each insights.activity.events.slice(0, 8) as activity, index (`${activity.occurredAt}:${activity.anilistId}:${index}`)}
            <li
              class="glass-control control-rounded border-border grid min-w-0 gap-2 border px-3 py-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center"
            >
              <div class="min-w-0">
                <p class="text-foreground truncate text-sm">
                  {titleForActivity(activity)}
                </p>
                <time
                  class="text-muted mt-1 block text-[0.65rem]"
                  datetime={activity.occurredAt}
                >
                  {formatTimestamp(activity.occurredAt)}
                </time>
              </div>
              <p class="text-accent text-xs tabular-nums sm:text-right">
                +{activity.progressDelta}
                {activity.progressDelta === 1 ? "episode" : "episodes"}
                <span class="text-muted block"
                  >{activityDuration(activity)}</span
                >
              </p>
            </li>
          {/each}
        </ol>
      {/if}
    </section>

    <section
      aria-labelledby="library-index-heading"
      class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-4 sm:p-6 lg:p-7"
    >
      <header
        class="border-border flex flex-col gap-3 border-b pb-5 sm:flex-row sm:items-end sm:justify-between"
      >
        <div>
          <h2
            bind:this={libraryIndexHeading}
            id="library-index-heading"
            tabindex="-1"
            class="text-foreground text-xl font-light tracking-[0.03em] sm:text-2xl"
          >
            Your library
          </h2>
        </div>
      </header>

      {#if insights.libraryTotals.tracked === 0}
        <div class="grid min-h-64 place-items-center py-8 text-center">
          <div class="max-w-xl">
            <p aria-hidden="true" class="text-accent text-xl">=^.^=</p>
            <h3 class="text-foreground mt-4 text-xl font-light">
              Your library has no anime yet.
            </h3>
            <p class="text-muted mt-2 text-sm leading-6">
              Open a title in Discover and add it to your library.
            </p>
            {#if openDiscovery}
              <button
                type="button"
                class="glass-accent control-rounded mt-5 min-h-10 px-4 py-2 text-xs tracking-[0.08em]"
                onclick={openDiscovery}
              >
                open Discover
              </button>
            {/if}
          </div>
        </div>
      {:else}
        <div class="mt-5 grid min-w-0 gap-4">
          <div
            role="search"
            aria-label="Filter library index"
            class="glass-control control-rounded border-border relative z-20 grid min-w-0 gap-3 border p-3 sm:grid-cols-2 sm:p-4 lg:grid-cols-[minmax(15rem,1fr)_minmax(10rem,0.45fr)_minmax(11rem,0.55fr)_auto] lg:items-end"
          >
            <label class="grid min-w-0 gap-1.5">
              <span
                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
              >
                find a title
              </span>
              <input
                type="search"
                placeholder="Title or AniList ID"
                bind:value={libraryQuery}
                oninput={resetLibraryScroll}
                class="glass-control control-rounded border-border text-foreground placeholder:text-muted min-h-10 w-full min-w-0 border px-3 py-2 text-sm outline-none"
              />
            </label>

            <div class="grid min-w-0 gap-1.5">
              <span
                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
              >
                status
              </span>
              <GlassSelect
                id="library-status-filter"
                label="status"
                value={libraryStatusFilter}
                options={libraryStatusOptions}
                onChange={changeLibraryStatusFilter}
              />
            </div>

            <div class="grid min-w-0 gap-1.5">
              <span
                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
              >
                sort by
              </span>
              <GlassSelect
                id="library-sort"
                label="sort by"
                value={librarySort}
                options={librarySortOptions}
                onChange={changeLibrarySort}
              />
            </div>

            <button
              type="button"
              class="glass-interactive control-rounded min-h-10 px-3 py-2 text-xs transition-colors disabled:cursor-not-allowed disabled:opacity-45"
              disabled={!libraryFiltersActive}
              onclick={resetLibraryFilters}
            >
              clear filters
            </button>
          </div>

          <div class="flex flex-wrap items-center justify-between gap-2">
            <p class="text-muted text-xs leading-5" aria-live="polite">
              <span class="text-foreground tabular-nums"
                >{filteredLibraryItems.length}</span
              >
              matching {filteredLibraryItems.length === 1 ? "title" : "titles"}
              shown.
            </p>
          </div>

          {#if filteredLibraryItems.length === 0}
            <div
              class="border-border grid min-h-48 place-items-center border-y py-8 text-center"
            >
              <div class="max-w-lg">
                <h3 class="text-foreground text-lg font-light">
                  No titles match these filters.
                </h3>
                <p class="text-muted mt-2 text-xs leading-5">
                  Try another title or status, or clear the filters to restore
                  your full library.
                </p>
                <button
                  type="button"
                  class="glass-interactive control-rounded mt-4 min-h-10 px-4 py-2 text-xs"
                  onclick={resetLibraryFilters}
                >
                  clear filters
                </button>
              </div>
            </div>
          {:else}
            <ul
              bind:this={libraryRail}
              aria-label="Library index results"
              class="flex min-w-0 touch-pan-x snap-x snap-mandatory gap-3 overflow-x-auto overscroll-x-contain scroll-smooth pb-3"
            >
              {#each filteredLibraryItems as item (item.entry.anilistId)}
                {@const itemTitle = titleForItem(item)}
                <li
                  class="w-[min(18rem,84vw)] min-w-0 shrink-0 snap-start sm:w-72"
                >
                  <button
                    type="button"
                    aria-haspopup="dialog"
                    aria-label={`Open library details for ${itemTitle}`}
                    aria-describedby={`library-card-summary-${item.entry.anilistId}`}
                    data-library-anilist-id={item.entry.anilistId}
                    class="glass-control glass-interactive control-rounded border-border grid h-full w-full min-w-0 grid-cols-[3.75rem_minmax(0,1fr)] gap-3 border p-2 text-left transition-colors"
                    onclick={(event) => openItemDetails(item, event)}
                  >
                    <span
                      id={`library-card-summary-${item.entry.anilistId}`}
                      class="sr-only"
                    >
                      {item.entry.status.toLocaleLowerCase()}.
                      {progressLabel(item)}.
                      {item.entry.score === null
                        ? "No personal score."
                        : `Personal score ${item.entry.score} out of 10.`}
                    </span>
                    <div
                      class="min-h-[5.625rem] w-[3.75rem] shrink-0 self-stretch"
                    >
                      <MediaCover
                        title={itemTitle}
                        coverUrl={item.media?.coverUrl ?? null}
                        fill
                      />
                    </div>

                    <span class="flex min-w-0 flex-col py-0.5">
                      <span
                        class="text-foreground line-clamp-2 text-sm leading-5 font-light"
                      >
                        {itemTitle}
                      </span>
                      <span
                        class="text-accent mt-1 text-[0.62rem] tracking-[0.08em] uppercase"
                      >
                        {item.entry.status.toLocaleLowerCase()}
                      </span>
                      <span
                        class="text-muted mt-1 text-[0.62rem] tracking-[0.08em] uppercase"
                      >
                        {formatMediaFormat(item.media?.mediaFormat ?? null)}
                      </span>
                      <span class="mt-3">
                        <span
                          class="text-foreground block text-xs tabular-nums"
                        >
                          {progressLabel(item)}
                        </span>
                        {#if item.media?.episodes === null || item.media === null}
                          <span class="text-warning mt-1 block text-[0.62rem]">
                            episode total unknown
                          </span>
                        {/if}
                        {#if item.entry.score !== null}
                          <span class="text-muted mt-1 block text-[0.62rem]">
                            score // {item.entry.score} / 10
                          </span>
                        {/if}
                      </span>
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    </section>
  {/if}

  {#if selectedItem && selectedSummary}
    <MediaDetailsDialog
      summary={selectedSummary}
      entry={selectedItem.entry}
      fallbackFreshness={freshnessForItem(selectedItem)}
      {loadDetails}
      {saveEntry}
      {deleteEntry}
      onDetailsLoaded={handleDetailsLoaded}
      onLibraryChanged={handleLibraryChanged}
      onClose={closeItemDetails}
    />
  {/if}
</div>
