<script lang="ts">
  import { onDestroy, onMount } from "svelte";

  import {
    advanceLibraryProgress,
    getHomeAiring,
    normalizeCommandError,
  } from "$lib/ipc/client";
  import type {
    AdvanceLibraryProgressRequest,
    CommandError,
    HomeAiringItem,
    HomeAiringResponse,
    LibraryEntry,
  } from "$lib/ipc/types";

  import AsyncStateNotice from "../AsyncStateNotice.svelte";
  import MediaCover from "../MediaCover.svelte";
  import PageHeader from "../PageHeader.svelte";

  interface Props {
    loadAiring?: () => Promise<HomeAiringResponse>;
    advanceProgress?: (
      request: AdvanceLibraryProgressRequest,
    ) => Promise<LibraryEntry>;
    openDiscovery?: () => void;
    refreshIntervalMs?: number;
  }

  let {
    loadAiring = getHomeAiring,
    advanceProgress = advanceLibraryProgress,
    openDiscovery = () => undefined,
    refreshIntervalMs = 30 * 60 * 1000,
  }: Props = $props();

  let response = $state<HomeAiringResponse | null>(null);
  let loading = $state(true);
  let loadError = $state<CommandError | null>(null);
  let advancingError = $state<CommandError | null>(null);
  let advancingId = $state<number | null>(null);
  let selectedIndex = $state(0);
  let now = $state(Date.now());
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let clockTimer: ReturnType<typeof setInterval> | null = null;

  const items = $derived(response?.items ?? []);
  const waitingCount = $derived(
    items.reduce((total, item) => total + catchUpCount(item), 0),
  );
  const unknownCount = $derived(
    items.filter((item) => item.nextAiring === null).length,
  );
  const staleCount = $derived(
    items.filter((item) => item.scheduleFreshness?.isStale).length,
  );

  onMount(() => {
    void refreshAiring();
    refreshTimer = setInterval(() => {
      void refreshAiring();
    }, refreshIntervalMs);
    clockTimer = setInterval(() => {
      now = Date.now();
    }, 60_000);
  });

  onDestroy(() => {
    if (refreshTimer !== null) clearInterval(refreshTimer);
    if (clockTimer !== null) clearInterval(clockTimer);
  });

  async function refreshAiring(): Promise<void> {
    if (response === null) loading = true;
    loadError = null;
    try {
      response = await loadAiring();
      if (selectedIndex >= response.items.length) selectedIndex = 0;
    } catch (reason) {
      loadError = normalizeCommandError(reason);
    } finally {
      loading = false;
    }
  }

  async function markNextWatched(item: HomeAiringItem): Promise<void> {
    if (!canAdvance(item) || advancingId !== null) return;
    advancingId = item.library.entry.anilistId;
    advancingError = null;
    try {
      const entry = await advanceProgress({
        anilistId: item.library.entry.anilistId,
      });
      if (response !== null) {
        response = {
          ...response,
          items: response.items.map((candidate) =>
            candidate.library.entry.anilistId === entry.anilistId
              ? {
                  ...candidate,
                  library: { ...candidate.library, entry },
                }
              : candidate,
          ),
        };
      }
    } catch (reason) {
      advancingError = normalizeCommandError(reason);
    } finally {
      advancingId = null;
    }
  }

  function selectedItem(): HomeAiringItem | null {
    return items[selectedIndex] ?? null;
  }

  function selectSlide(index: number): void {
    if (items.length === 0) return;
    selectedIndex = (index + items.length) % items.length;
    advancingError = null;
  }

  function handleCarouselKeydown(event: KeyboardEvent): void {
    if (event.key === "ArrowRight") {
      event.preventDefault();
      selectSlide(selectedIndex + 1);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      selectSlide(selectedIndex - 1);
    } else if (event.key === "Home") {
      event.preventDefault();
      selectSlide(0);
    } else if (event.key === "End") {
      event.preventDefault();
      selectSlide(items.length - 1);
    }
  }

  function catchUpCount(item: HomeAiringItem): number {
    if (item.nextAiring === null) return 0;
    return Math.max(
      0,
      item.nextAiring.episode - 1 - item.library.entry.progress,
    );
  }

  function canAdvance(item: HomeAiringItem): boolean {
    return catchUpCount(item) > 0;
  }

  function title(item: HomeAiringItem): string {
    return (
      item.library.media?.title ?? `AniList #${item.library.entry.anilistId}`
    );
  }

  function progressLabel(item: HomeAiringItem): string {
    return `${item.library.entry.progress.toString().padStart(2, "0")} / ${
      item.library.media?.episodes?.toString().padStart(2, "0") ?? "??"
    }`;
  }

  function progressPercent(item: HomeAiringItem): number {
    const total = item.library.media?.episodes;
    if (total === null || total === undefined || total === 0) {
      return Math.min(item.library.entry.progress * 8, 88);
    }
    return Math.min(100, (item.library.entry.progress / total) * 100);
  }

  function countdown(item: HomeAiringItem): string {
    if (item.nextAiring === null) return "unknown";
    const remaining = new Date(item.nextAiring.airingAt).getTime() - now;
    if (!Number.isFinite(remaining)) return "unknown";
    if (remaining <= 0) return "airing now";
    const minutes = Math.floor(remaining / 60_000);
    const days = Math.floor(minutes / 1_440);
    const hours = Math.floor((minutes % 1_440) / 60);
    const remainder = minutes % 60;
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${remainder}m`;
    return `${Math.max(1, remainder)}m`;
  }

  function formatTimestamp(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(date);
  }

  function scheduleState(
    item: HomeAiringItem,
  ): "catch-up" | "on-time" | "unknown" {
    if (item.nextAiring === null) return "unknown";
    return catchUpCount(item) > 0 ? "catch-up" : "on-time";
  }
</script>

<div
  id="home-panel"
  role="tabpanel"
  aria-labelledby="home-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <PageHeader
    eyebrow="Today // your airing desk"
    title="Home"
    description="Keep the week in view, catch the episodes that slipped past, and log one more without leaving your queue."
  />

  <section
    aria-labelledby="airing-queue-heading"
    class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-4 sm:p-6 lg:p-7"
  >
    <header
      class="border-border flex flex-col gap-4 border-b pb-5 sm:flex-row sm:items-end sm:justify-between"
    >
      <div>
        <p class="text-accent text-xs tracking-[0.18em] uppercase">
          airing queue // local library
        </p>
        <h2
          id="airing-queue-heading"
          class="text-foreground mt-2 text-xl font-light tracking-[0.03em] sm:text-2xl"
        >
          Your week, one episode at a time.
        </h2>
        {#if response}
          <p class="text-muted mt-2 text-xs leading-5">
            Schedule check {formatTimestamp(response.checkedAt)}. Cached data is
            reused for 30 minutes.
          </p>
        {/if}
      </div>
      <button
        type="button"
        disabled={loading}
        class="glass-control glass-interactive control-rounded min-h-10 px-4 py-2 text-xs tracking-[0.08em] disabled:opacity-55"
        onclick={refreshAiring}
      >
        {loading ? "checking schedules…" : "check schedules"}
      </button>
    </header>

    {#if loading && response === null}
      <div class="py-6">
        <AsyncStateNotice
          status="loading"
          title="Checking your airing desk"
          description="Animi is joining Watching entries to locally cached schedules."
        />
      </div>
    {:else if loadError && response === null}
      <div class="py-6">
        <AsyncStateNotice
          status={loadError.code.includes("OFFLINE") ? "offline" : "error"}
          title="The airing desk is unavailable"
          description="No library progress was changed."
          error={loadError}
          onRetry={loadError.retryable ? refreshAiring : undefined}
          retryLabel="Check again"
        />
      </div>
    {:else if items.length === 0}
      <div
        class="grid min-h-80 items-center gap-6 py-8 sm:grid-cols-[minmax(13rem,0.7fr)_minmax(0,1fr)] sm:py-12"
      >
        <div
          aria-hidden="true"
          class="glass-control control-rounded border-border text-accent mx-auto grid min-h-48 w-full max-w-xs place-items-center border p-6 text-center"
        >
          <div>
            <p class="text-4xl tracking-[0.12em]">／ᐠ｡ꞈ｡ᐟ＼</p>
            <p class="text-muted mt-4 text-xs tracking-[0.14em] uppercase">
              queue asleep
            </p>
          </div>
        </div>
        <div class="max-w-lg">
          <p class="text-accent text-xs tracking-[0.16em] uppercase">
            no active anime
          </p>
          <h3 class="text-foreground mt-2 text-2xl font-light">
            Nothing is in your Watching shelf.
          </h3>
          <p class="text-muted mt-3 text-sm leading-6">
            Add an anime from Discover or move a planned title into Watching to
            begin an airing queue.
          </p>
          <button
            type="button"
            class="glass-accent control-rounded mt-5 px-4 py-2.5 text-xs tracking-[0.08em]"
            onclick={openDiscovery}
          >
            visit Discover →
          </button>
        </div>
      </div>
    {:else}
      <div
        class="flex flex-col gap-4 py-5 sm:flex-row sm:items-center sm:justify-between"
      >
        <div class="flex flex-wrap items-center gap-x-5 gap-y-2">
          <p class="text-muted text-xs tracking-[0.1em] uppercase">
            <span class="text-foreground text-base">{items.length}</span> watching
          </p>
          <p class="text-muted text-xs tracking-[0.1em] uppercase">
            <span class="text-warning text-base">{waitingCount}</span> waiting
          </p>
          <p class="text-muted text-xs tracking-[0.1em] uppercase">
            <span class="text-foreground text-base">{unknownCount}</span> unknown
          </p>
          {#if staleCount > 0}
            <p class="text-warning text-xs tracking-[0.1em] uppercase">
              {staleCount} stale
            </p>
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <span
            aria-live="polite"
            class="text-muted min-w-16 text-center text-xs tracking-[0.14em]"
          >
            {(selectedIndex + 1).toString().padStart(2, "0")} // {items.length
              .toString()
              .padStart(2, "0")}
          </span>
          <button
            type="button"
            aria-label="Previous anime"
            class="glass-control glass-interactive control-rounded grid size-10 place-items-center"
            onclick={() => selectSlide(selectedIndex - 1)}
            onkeydown={handleCarouselKeydown}>←</button
          >
          <button
            type="button"
            aria-label="Next anime"
            class="glass-control glass-interactive control-rounded grid size-10 place-items-center"
            onclick={() => selectSlide(selectedIndex + 1)}
            onkeydown={handleCarouselKeydown}>→</button
          >
        </div>
      </div>

      {@const item = selectedItem()}
      {#if item}
        <div
          role="region"
          aria-roledescription="carousel"
          aria-label="Airing anime"
        >
          <article
            role="group"
            aria-roledescription="slide"
            aria-label={`${selectedIndex + 1} of ${items.length}: ${title(item)}`}
            class="glass-control control-rounded border-border grid min-w-0 gap-5 border p-4 sm:grid-cols-[11rem_minmax(0,1fr)] sm:p-5 lg:grid-cols-[13rem_minmax(0,1fr)] lg:gap-7 lg:p-6"
          >
            <div class="mx-auto w-36 sm:w-full">
              <MediaCover
                title={title(item)}
                coverUrl={item.library.media?.coverUrl ?? null}
              />
            </div>
            <div class="flex min-w-0 flex-col sm:py-1">
              <div class="flex flex-wrap items-center gap-2">
                <span
                  class:text-warning={scheduleState(item) === "catch-up"}
                  class:text-success={scheduleState(item) === "on-time"}
                  class="control-rounded border-border bg-glass-control border px-2 py-1 text-[0.65rem] tracking-[0.12em] uppercase"
                >
                  {scheduleState(item) === "catch-up"
                    ? "catch up"
                    : scheduleState(item) === "on-time"
                      ? "on time"
                      : "schedule unknown"}
                </span>
                <span
                  class="text-muted text-[0.68rem] tracking-[0.08em] uppercase"
                >
                  {item.library.media?.mediaFormat
                    ?.replaceAll("_", " ")
                    .toLocaleLowerCase() ?? "format unknown"}
                </span>
              </div>
              <h3
                class="text-foreground mt-4 text-xl leading-tight font-light sm:text-3xl"
              >
                {title(item)}
              </h3>
              <p
                class="text-muted mt-2 text-[0.68rem] tracking-[0.08em] uppercase"
              >
                {item.scheduleFreshness
                  ? `${item.scheduleFreshness.isStale ? "stale schedule" : "schedule cached"} // ${formatTimestamp(item.scheduleFreshness.fetchedAt)}`
                  : (item.scheduleError?.message ?? "schedule not available")}
              </p>

              <div class="border-border mt-6 border-l-2 pl-4">
                {#if scheduleState(item) === "catch-up"}
                  <p class="text-warning text-2xl font-light sm:text-3xl">
                    {catchUpCount(item)}
                    {catchUpCount(item) === 1 ? "episode" : "episodes"} waiting
                  </p>
                  <p class="text-muted mt-2 text-xs leading-5">
                    Episode {item.nextAiring?.episode} airs in {countdown(
                      item,
                    )}.
                  </p>
                {:else if scheduleState(item) === "on-time"}
                  <p class="text-foreground text-3xl font-light">
                    {countdown(item)}
                  </p>
                  <p class="text-muted mt-2 text-xs leading-5">
                    until episode {item.nextAiring?.episode}; you are right on
                    time.
                  </p>
                {:else}
                  <p class="text-foreground text-2xl font-light">
                    schedule unknown
                  </p>
                  <p class="text-muted mt-2 text-xs leading-5">
                    Animi will not estimate the next episode or change progress
                    from Home.
                  </p>
                {/if}
              </div>

              <dl class="mt-6 grid grid-cols-2 gap-4">
                <div>
                  <dt
                    class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                  >
                    progress
                  </dt>
                  <dd
                    aria-live="polite"
                    class="text-foreground mt-1 text-base tracking-[0.08em]"
                  >
                    {progressLabel(item)}
                  </dd>
                </div>
                <div>
                  <dt
                    class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                  >
                    next episode
                  </dt>
                  <dd class="text-foreground mt-1 text-base tracking-[0.08em]">
                    {item.nextAiring?.episode.toString().padStart(2, "0") ??
                      "—"}
                  </dd>
                </div>
              </dl>
              <div
                aria-hidden="true"
                class="bg-glass-control mt-3 h-1.5 overflow-hidden rounded-sm"
              >
                <span
                  class="bg-accent block h-full rounded-sm transition-[width]"
                  style:width={`${progressPercent(item)}%`}
                ></span>
              </div>
              <div class="mt-5 sm:mt-auto sm:pt-5">
                <button
                  type="button"
                  disabled={!canAdvance(item) || advancingId !== null}
                  class="glass-accent control-rounded w-full px-4 py-3 text-xs tracking-[0.06em] disabled:cursor-default disabled:opacity-55 sm:w-fit sm:min-w-60"
                  onclick={() => markNextWatched(item)}
                >
                  {advancingId === item.library.entry.anilistId
                    ? "saving progress…"
                    : canAdvance(item)
                      ? `mark episode ${item.library.entry.progress + 1} watched`
                      : scheduleState(item) === "unknown"
                        ? "schedule unavailable"
                        : "all caught up"}
                </button>
              </div>
            </div>
          </article>
          {#if advancingError}
            <div class="mt-3">
              <AsyncStateNotice
                status="error"
                title="Progress was not changed"
                error={advancingError}
                compact
              />
            </div>
          {/if}
          <div class="mt-3 flex items-center justify-center gap-1">
            {#each items as carouselItem, index (carouselItem.library.entry.anilistId)}
              <button
                type="button"
                aria-label={`Show ${title(carouselItem)}`}
                aria-current={selectedIndex === index ? "true" : undefined}
                class="control-rounded grid size-10 place-items-center"
                onclick={() => selectSlide(index)}
                onkeydown={handleCarouselKeydown}
              >
                <span
                  aria-hidden="true"
                  class="control-rounded border-border h-1.5 border transition-[width,background-color]"
                  class:bg-accent={selectedIndex === index}
                  class:bg-glass-control={selectedIndex !== index}
                  class:w-8={selectedIndex === index}
                  class:w-3={selectedIndex !== index}
                ></span>
              </button>
            {/each}
          </div>
        </div>
      {/if}
      {#if loadError}
        <div class="mt-4">
          <AsyncStateNotice
            status={loadError.code.includes("OFFLINE") ? "offline" : "error"}
            title="The latest schedule check failed"
            description="The prior local airing desk remains visible."
            error={loadError}
            onRetry={loadError.retryable ? refreshAiring : undefined}
            compact
          />
        </div>
      {/if}
    {/if}
  </section>
</div>
