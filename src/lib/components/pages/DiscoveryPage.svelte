<script lang="ts">
  import { onDestroy, tick } from "svelte";

  import {
    discoveryPreviewAnime,
    matchesDiscoveryQuery,
    type DiscoveryPreviewAnime,
  } from "../discoveryPreview";
  import DiscoveryPreviewCover from "../DiscoveryPreviewCover.svelte";
  import PageHeader from "../PageHeader.svelte";

  const trendingPreview = discoveryPreviewAnime.slice(0, 4);
  const recommendationPreview = discoveryPreviewAnime.slice(4);

  let query = $state("");
  let submittedQuery = $state("");
  let selectedAnime = $state<DiscoveryPreviewAnime | null>(null);

  let searchInput: HTMLInputElement;
  let detailsDialog = $state<HTMLDialogElement | null>(null);
  let detailsCloseButton = $state<HTMLButtonElement | null>(null);
  let detailsOpener: HTMLButtonElement | null = null;
  let pendingFocusTarget: HTMLElement | null = null;

  const searchResults = $derived(
    discoveryPreviewAnime.filter((anime) =>
      matchesDiscoveryQuery(anime, submittedQuery),
    ),
  );

  const resultStatus = $derived(
    submittedQuery.length === 0
      ? `${discoveryPreviewAnime.length} bundled sample titles ready to browse.`
      : `${searchResults.length} bundled sample ${
          searchResults.length === 1 ? "match" : "matches"
        } for ${submittedQuery}.`,
  );

  onDestroy(() => {
    document.documentElement.classList.remove("has-modal");
  });

  function submitSearch(event: SubmitEvent) {
    event.preventDefault();
    closeDetails(searchInput);
    submittedQuery = query.trim();
  }

  function clearSearch() {
    closeDetails(searchInput);
    query = "";
    submittedQuery = "";
    void tick().then(() => searchInput?.focus());
  }

  async function openDetails(anime: DiscoveryPreviewAnime, event: MouseEvent) {
    if (!(event.currentTarget instanceof HTMLButtonElement)) {
      return;
    }

    detailsOpener = event.currentTarget;
    pendingFocusTarget = detailsOpener;
    selectedAnime = anime;
    await tick();

    const dialog = detailsDialog;
    if (dialog === null) {
      return;
    }

    document.documentElement.classList.add("has-modal");
    try {
      dialog.showModal();
    } catch {
      dialog.setAttribute("open", "");
    }
    detailsCloseButton?.focus();
  }

  function closeDetails(focusTarget: HTMLElement | null = detailsOpener) {
    if (selectedAnime === null) {
      return;
    }

    pendingFocusTarget = focusTarget;
    if (detailsDialog?.open && typeof detailsDialog.close === "function") {
      try {
        detailsDialog.close();
        return;
      } catch {
        // The local test environment does not implement native dialog closing.
      }
    }

    detailsDialog?.removeAttribute("open");
    finishClosingDetails();
  }

  function finishClosingDetails() {
    document.documentElement.classList.remove("has-modal");
    selectedAnime = null;
    const focusTarget =
      pendingFocusTarget?.isConnected === true
        ? pendingFocusTarget
        : searchInput;
    pendingFocusTarget = null;
    void tick().then(() => focusTarget?.focus());
  }

  function handleDialogBackdrop(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      closeDetails();
    }
  }

  function handleDialogCancel(event: Event) {
    event.preventDefault();
    closeDetails();
  }

  function formatLabel(value: string | null): string {
    if (value === null) {
      return "unknown";
    }

    return value.toLocaleLowerCase().replaceAll("_", " ");
  }

  function episodeLabel(episodes: number | null): string {
    if (episodes === null) {
      return "episode count unknown";
    }

    return `${episodes} ${episodes === 1 ? "episode" : "episodes"}`;
  }

  function formatCalendarDate(value: string): string {
    return new Intl.DateTimeFormat(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      timeZone: "UTC",
    }).format(new Date(`${value}T00:00:00Z`));
  }
</script>

<div
  id="discovery-panel"
  role="tabpanel"
  aria-labelledby="discovery-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <div class="grid min-w-0 gap-4 sm:gap-5">
    <PageHeader
      eyebrow="catalog // find your next signal"
      title="Discover"
      description="Search the anime catalog, scan cover collections, and inspect a title without leaving your quiet corner."
    />

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
            Scan the bundled catalog
          </h2>
        </div>
        <span
          class="control-rounded border-border bg-glass-control text-muted border px-2 py-1 text-[0.62rem] tracking-[0.12em] uppercase"
        >
          sample catalog
        </span>
      </div>

      <form
        role="search"
        aria-label="Discover sample anime"
        class="grid min-w-0 gap-2 sm:grid-cols-[minmax(0,1fr)_auto]"
        onsubmit={submitSearch}
      >
        <div class="min-w-0">
          <label
            for="discovery-search"
            class="text-muted mb-2 block text-xs tracking-[0.08em]"
          >
            Title, genre, or tag
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
              placeholder="Try mystery, cats, or movie"
              class="glass-control control-rounded border-border text-foreground placeholder:text-muted min-h-11 w-full min-w-0 border py-2 pr-11 pl-10 text-sm outline-none"
              oninput={(event) => (query = event.currentTarget.value)}
            />
            {#if query.length > 0 || submittedQuery.length > 0}
              <button
                type="button"
                aria-label="Clear search"
                class="glass-interactive control-rounded text-muted absolute top-1/2 right-0.5 grid size-10 -translate-y-1/2 place-items-center transition-colors"
                onclick={clearSearch}
              >
                <span aria-hidden="true">×</span>
              </button>
            {/if}
          </div>
        </div>

        <button
          type="submit"
          class="glass-accent control-rounded min-h-11 self-end px-5 py-2.5 text-xs tracking-[0.08em]"
        >
          search samples
        </button>
      </form>

      <div
        class="border-border grid gap-2 border-t pt-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-start"
      >
        <p class="text-muted text-xs leading-5">
          Layout preview only — this searches bundled fictional titles; no
          network request or library change occurs.
        </p>
        <p
          role="status"
          aria-live="polite"
          class="text-foreground text-xs leading-5 sm:text-right"
        >
          {resultStatus}
        </p>
      </div>
    </section>

    {#snippet previewCard(anime: DiscoveryPreviewAnime, showSignals: boolean)}
      <li class="min-w-0">
        <button
          type="button"
          aria-haspopup="dialog"
          aria-label={`Open sample details for ${anime.media.title}`}
          class="glass-control glass-interactive control-rounded border-border group flex h-full w-full min-w-0 flex-col border p-2 text-left transition-[transform,background-color] hover:-translate-y-0.5 focus-visible:-translate-y-0.5"
          onclick={(event) => openDetails(anime, event)}
        >
          <DiscoveryPreviewCover
            title={anime.media.title}
            motif={anime.motif}
            code={anime.catalogCode}
          />

          <span
            class="flex min-h-30 w-full min-w-0 flex-1 flex-col px-1 pt-3 pb-2"
          >
            <span class="text-muted text-[0.66rem] tracking-[0.09em] uppercase">
              {formatLabel(anime.media.mediaFormat)} //
              {anime.media.episodes ?? "??"}
            </span>
            <span
              class="text-foreground mt-1.5 line-clamp-2 text-sm leading-5 font-light tracking-[0.02em]"
            >
              {anime.media.title}
            </span>
            <span class="text-muted mt-2 line-clamp-1 text-xs">
              {anime.media.genres.join(" · ")}
            </span>
            {#if showSignals}
              <span
                class="border-border text-accent mt-auto border-t pt-2 text-[0.62rem] leading-4 tracking-[0.06em]"
              >
                sample signals // {anime.matchSignals.join(" · ")}
              </span>
            {:else}
              <span
                class="text-accent mt-auto pt-2 text-[0.62rem] tracking-[0.08em] uppercase"
              >
                inspect dossier →
              </span>
            {/if}
          </span>
        </button>
      </li>
    {/snippet}

    {#if submittedQuery.length === 0}
      <section
        aria-labelledby="trending-preview-heading"
        class="glass-panel panel-large border-border min-w-0 border p-4 sm:p-6"
      >
        <div class="mb-4 flex flex-wrap items-end justify-between gap-2">
          <div>
            <p class="text-accent text-[0.66rem] tracking-[0.14em] uppercase">
              01 // cover collection
            </p>
            <h2
              id="trending-preview-heading"
              class="text-foreground mt-1 text-xl font-light tracking-[0.03em]"
            >
              Sample trending layout
            </h2>
          </div>
          <p class="text-muted text-xs">fictional snapshot // 04 titles</p>
        </div>

        <ul
          aria-label="Sample trending anime"
          class="grid min-w-0 grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 lg:gap-4 xl:grid-cols-6"
        >
          {#each trendingPreview as anime (anime.media.anilistId)}
            {@render previewCard(anime, false)}
          {/each}
        </ul>
      </section>

      <section
        aria-labelledby="recommendation-preview-heading"
        class="glass-panel panel-large border-border min-w-0 border p-4 sm:p-6"
      >
        <div class="mb-4 max-w-2xl">
          <p class="text-accent text-[0.66rem] tracking-[0.14em] uppercase">
            02 // explanation treatment
          </p>
          <h2
            id="recommendation-preview-heading"
            class="text-foreground mt-1 text-xl font-light tracking-[0.03em]"
          >
            Recommendation layout preview
          </h2>
          <p class="text-muted mt-2 text-xs leading-5">
            These labels demonstrate how future local match reasons can read;
            they are not personalized rankings.
          </p>
        </div>

        <ul
          aria-label="Sample recommendation layout"
          class="grid min-w-0 grid-cols-2 gap-3 sm:max-w-[24rem] lg:gap-4"
        >
          {#each recommendationPreview as anime (anime.media.anilistId)}
            {@render previewCard(anime, true)}
          {/each}
        </ul>
      </section>
    {:else if searchResults.length > 0}
      <section
        aria-labelledby="search-results-heading"
        class="glass-panel panel-large border-border min-w-0 border p-4 sm:p-6"
      >
        <div class="mb-4">
          <p class="text-accent text-[0.66rem] tracking-[0.14em] uppercase">
            query // {submittedQuery}
          </p>
          <h2
            id="search-results-heading"
            class="text-foreground mt-1 text-xl font-light tracking-[0.03em]"
          >
            Sample catalog results
          </h2>
        </div>

        <ul
          aria-label="Bundled sample search results"
          class="grid min-w-0 grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 lg:gap-4 xl:grid-cols-6"
        >
          {#each searchResults as anime (anime.media.anilistId)}
            {@render previewCard(anime, false)}
          {/each}
        </ul>
      </section>
    {:else}
      <section
        aria-labelledby="empty-results-heading"
        class="glass-panel panel-large border-border grid min-h-72 items-center gap-5 border p-5 sm:grid-cols-[10rem_minmax(0,1fr)] sm:p-7"
      >
        <div
          aria-hidden="true"
          class="glass-control control-rounded border-border text-accent grid min-h-36 place-items-center border p-4"
        >
          <div class="text-center">
            <p class="text-3xl">ฅ(≈චᆽච≈)ฅ</p>
            <p
              class="text-muted mt-3 text-[0.62rem] tracking-[0.12em] uppercase"
            >
              no signal
            </p>
          </div>
        </div>

        <div class="max-w-xl">
          <p class="text-accent text-xs tracking-[0.14em] uppercase">
            sample catalog // no match
          </p>
          <h2
            id="empty-results-heading"
            class="text-foreground mt-2 text-xl font-light tracking-[0.03em] sm:text-2xl"
          >
            No bundled sample titles match “{submittedQuery}”.
          </h2>
          <p class="text-muted mt-3 text-sm leading-6">
            Try a broader genre or tag. This only describes the six fictional
            entries in the layout preview, not AniList results.
          </p>
          <button
            type="button"
            class="glass-accent control-rounded mt-5 min-h-10 px-4 py-2 text-xs tracking-[0.08em]"
            onclick={clearSearch}
          >
            clear search
          </button>
        </div>
      </section>
    {/if}
  </div>

  {#if selectedAnime}
    <dialog
      bind:this={detailsDialog}
      aria-labelledby="preview-details-title"
      aria-describedby="preview-details-description"
      aria-modal="true"
      class="discovery-dialog glass-panel panel-large border-border text-foreground border"
      onclose={finishClosingDetails}
      oncancel={handleDialogCancel}
      onclick={handleDialogBackdrop}
    >
      <div class="min-h-full">
        <div
          class="bg-glass border-border sticky top-0 z-10 flex min-h-14 items-center justify-between gap-3 border-b px-4 py-2 backdrop-blur-xl"
        >
          <p
            class="text-accent min-w-0 truncate text-[0.66rem] tracking-[0.12em] uppercase"
          >
            dossier // {selectedAnime.catalogCode}
          </p>
          <button
            bind:this={detailsCloseButton}
            type="button"
            aria-label="Close details"
            class="glass-control glass-interactive control-rounded grid size-10 shrink-0 place-items-center transition-colors"
            onclick={() => closeDetails()}
          >
            <span aria-hidden="true">×</span>
          </button>
        </div>

        <div
          class="grid min-w-0 gap-5 p-4 sm:grid-cols-[10rem_minmax(0,1fr)] sm:p-6"
        >
          <div class="mx-auto w-full max-w-40">
            <DiscoveryPreviewCover
              title={selectedAnime.media.title}
              motif={selectedAnime.motif}
              code={selectedAnime.catalogCode}
            />
          </div>

          <div class="min-w-0">
            <p class="text-accent text-[0.66rem] tracking-[0.12em] uppercase">
              sample details // not fetched
            </p>
            <h2
              id="preview-details-title"
              class="text-foreground mt-2 text-2xl leading-tight font-light tracking-[0.025em]"
            >
              {selectedAnime.media.title}
            </h2>
            <p
              id="preview-details-description"
              class="text-muted mt-2 text-xs leading-5"
            >
              Secondary details layout preview. No library changes are available
              in this checkpoint.
            </p>

            <dl class="mt-5 grid grid-cols-2 gap-x-4 gap-y-4">
              <div>
                <dt
                  class="text-muted text-[0.62rem] tracking-[0.12em] uppercase"
                >
                  format
                </dt>
                <dd class="text-foreground mt-1 text-sm">
                  {formatLabel(selectedAnime.media.mediaFormat)}
                </dd>
              </div>
              <div>
                <dt
                  class="text-muted text-[0.62rem] tracking-[0.12em] uppercase"
                >
                  status
                </dt>
                <dd class="text-foreground mt-1 text-sm">
                  {formatLabel(selectedAnime.media.status)}
                </dd>
              </div>
              <div class="col-span-2">
                <dt
                  class="text-muted text-[0.62rem] tracking-[0.12em] uppercase"
                >
                  episodes
                </dt>
                <dd class="text-foreground mt-1 text-sm">
                  {episodeLabel(selectedAnime.media.episodes)}
                </dd>
              </div>
            </dl>
          </div>
        </div>

        <div class="border-border grid gap-5 border-t px-4 py-5 sm:px-6">
          <section aria-labelledby="preview-genres-heading">
            <h3
              id="preview-genres-heading"
              class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
            >
              genres
            </h3>
            <ul class="mt-2 flex flex-wrap gap-2">
              {#each selectedAnime.media.genres as genre (genre)}
                <li
                  class="control-rounded border-border bg-glass-control border px-2 py-1 text-xs"
                >
                  {genre}
                </li>
              {/each}
            </ul>
          </section>

          <section aria-labelledby="preview-tags-heading">
            <h3
              id="preview-tags-heading"
              class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
            >
              ranked tags
            </h3>
            <ul class="mt-2 grid gap-2">
              {#each selectedAnime.media.tags as tag (tag.name)}
                <li
                  class="glass-control control-rounded border-border flex items-center justify-between gap-3 border px-3 py-2 text-xs"
                >
                  <span>{tag.name}</span>
                  <span class="text-accent">rank {tag.rank}</span>
                </li>
              {/each}
            </ul>
          </section>

          <section aria-labelledby="preview-sequel-heading">
            <h3
              id="preview-sequel-heading"
              class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
            >
              official sequel relations
            </h3>
            {#if selectedAnime.media.sequels.length > 0}
              <ul class="mt-2 grid gap-2">
                {#each selectedAnime.media.sequels as sequel (sequel.anilistId)}
                  <li
                    class="glass-control control-rounded border-border border px-3 py-2 text-xs leading-5"
                  >
                    <span class="text-foreground block">{sequel.title}</span>
                    {#if sequel.releaseDate}
                      <time
                        class="text-muted block"
                        datetime={sequel.releaseDate}
                      >
                        releases {formatCalendarDate(sequel.releaseDate)}
                      </time>
                    {:else}
                      <span class="text-muted block">release date unknown</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="text-muted mt-2 text-xs leading-5">
                No official sequel relation in this sample.
              </p>
            {/if}
          </section>

          <button
            type="button"
            disabled
            class="glass-accent control-rounded min-h-11 w-full px-4 py-2.5 text-xs tracking-[0.06em] disabled:cursor-default disabled:opacity-55"
          >
            add to planning // preview only
          </button>
        </div>
      </div>
    </dialog>
  {/if}
</div>

<style>
  .discovery-dialog {
    position: fixed;
    inset: 1rem 1rem 1rem auto;
    width: min(34rem, calc(100vw - 2rem));
    height: calc(100dvh - 2rem);
    max-width: none;
    max-height: none;
    margin: 0 0 0 auto;
    padding: 0;
    overflow-y: auto;
  }

  .discovery-dialog::backdrop {
    background: var(--scrim);
    -webkit-backdrop-filter: blur(5px);
    backdrop-filter: blur(5px);
  }

  @media (max-width: 639px) {
    .discovery-dialog {
      inset: 0.5rem;
      width: calc(100vw - 1rem);
      height: calc(100dvh - 1rem);
      margin: 0;
    }
  }
</style>
