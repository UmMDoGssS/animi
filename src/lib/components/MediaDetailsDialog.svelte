<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";

  import {
    getMediaDetails,
    normalizeCommandError,
    removeLibraryEntry,
    upsertLibraryEntry,
  } from "$lib/ipc/client";
  import type {
    CommandError,
    Freshness,
    LibraryEntry,
    MediaDetails,
    MediaDetailsRequest,
    MediaDetailsResponse,
    MediaSummary,
    RemoveLibraryEntryRequest,
    RemoveLibraryEntryResponse,
    UpsertLibraryEntryRequest,
  } from "$lib/ipc/types";

  import AsyncStateNotice from "./AsyncStateNotice.svelte";
  import LibraryEntryEditor from "./LibraryEntryEditor.svelte";
  import MediaCover from "./MediaCover.svelte";

  interface Props {
    summary: MediaSummary;
    entry: LibraryEntry | null;
    fallbackFreshness?: Freshness | null;
    loadDetails?: (
      request: MediaDetailsRequest,
    ) => Promise<MediaDetailsResponse>;
    saveEntry?: (request: UpsertLibraryEntryRequest) => Promise<LibraryEntry>;
    deleteEntry?: (
      request: RemoveLibraryEntryRequest,
    ) => Promise<RemoveLibraryEntryResponse>;
    trackingError?: CommandError | null;
    onRetryTracking?: () => void | Promise<void>;
    onLibraryChanged?: (entry: LibraryEntry | null) => void | Promise<void>;
    onClose: () => void;
  }

  let {
    summary,
    entry,
    fallbackFreshness = null,
    loadDetails = getMediaDetails,
    saveEntry = upsertLibraryEntry,
    deleteEntry = removeLibraryEntry,
    trackingError = null,
    onRetryTracking,
    onLibraryChanged,
    onClose,
  }: Props = $props();

  let dialog = $state<HTMLDialogElement | null>(null);
  let closeButton = $state<HTMLButtonElement | null>(null);
  let detailsResponse = $state<MediaDetailsResponse | null>(null);
  let detailsError = $state<CommandError | null>(null);
  let detailsLoading = $state(true);
  let currentEntry = $state<LibraryEntry | null>(null);
  let currentEntrySource = $state<string | null>(null);
  let libraryMessage = $state<string | null>(null);
  let hasFinishedClosing = false;

  const fallbackDetails = $derived<MediaDetails>({
    ...summary,
    sequels: [],
  });
  const media = $derived(detailsResponse?.media ?? fallbackDetails);
  const freshness = $derived(detailsResponse?.freshness ?? fallbackFreshness);
  const trackingLoading = $derived(trackingError?.code === "LIBRARY_LOADING");

  $effect.pre(() => {
    const nextSource = `${summary.anilistId}:${entry?.updatedAt ?? "new"}`;
    if (nextSource === currentEntrySource) {
      return;
    }
    currentEntrySource = nextSource;
    currentEntry = entry;
  });

  onMount(() => {
    document.documentElement.classList.add("has-modal");
    const currentDialog = dialog;
    if (currentDialog !== null) {
      try {
        currentDialog.showModal();
      } catch {
        currentDialog.setAttribute("open", "");
      }
    }
    void tick().then(() => closeButton?.focus());
    void fetchDetails();
  });

  onDestroy(() => {
    document.documentElement.classList.remove("has-modal");
  });

  async function fetchDetails(): Promise<void> {
    detailsLoading = true;
    detailsError = null;

    try {
      detailsResponse = await loadDetails({ anilistId: summary.anilistId });
    } catch (reason) {
      detailsError = normalizeCommandError(reason);
    } finally {
      detailsLoading = false;
    }
  }

  function requestClose(): void {
    if (dialog?.open && typeof dialog.close === "function") {
      try {
        dialog.close();
        return;
      } catch {
        // jsdom and older WebViews may not implement native dialog closing.
      }
    }

    dialog?.removeAttribute("open");
    finishClosing();
  }

  function finishClosing(): void {
    if (hasFinishedClosing) {
      return;
    }
    hasFinishedClosing = true;
    document.documentElement.classList.remove("has-modal");
    onClose();
  }

  function handleBackdrop(event: MouseEvent): void {
    if (event.target === event.currentTarget) {
      requestClose();
    }
  }

  function handleCancel(event: Event): void {
    event.preventDefault();
    requestClose();
  }

  async function handleSaved(savedEntry: LibraryEntry): Promise<void> {
    currentEntry = savedEntry;
    libraryMessage =
      currentEntry === null ? null : "Saved to your local library.";
    await onLibraryChanged?.(savedEntry);
  }

  async function handleRemoved(): Promise<void> {
    currentEntry = null;
    libraryMessage =
      "Removed from your local library. Recorded activity was kept.";
    await onLibraryChanged?.(null);
  }

  function formatLabel(value: string | null): string {
    return value === null
      ? "unknown"
      : value.toLocaleLowerCase().replaceAll("_", " ");
  }

  function episodeLabel(episodes: number | null): string {
    if (episodes === null) {
      return "Episode total unknown";
    }
    return `${episodes} ${episodes === 1 ? "episode" : "episodes"}`;
  }

  function durationLabel(minutes: number | null): string {
    return minutes === null ? "Duration unknown" : `${minutes} min per episode`;
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

  function formatCalendarDate(value: string): string {
    return new Intl.DateTimeFormat(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      timeZone: "UTC",
    }).format(new Date(`${value}T00:00:00Z`));
  }

  function isOfflineError(error: CommandError): boolean {
    return error.code.toLocaleUpperCase().includes("OFFLINE");
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="media-details-title"
  aria-describedby="media-details-description"
  aria-modal="true"
  class="media-details-dialog glass-panel panel-large border-border text-foreground border"
  onclose={finishClosing}
  oncancel={handleCancel}
  onclick={handleBackdrop}
>
  <div class="min-h-full">
    <header
      class="bg-glass border-border sticky top-0 z-20 flex min-h-14 items-center justify-between gap-3 border-b px-4 py-2 backdrop-blur-xl"
    >
      <div class="min-w-0">
        <p
          class="text-accent truncate text-[0.66rem] tracking-[0.12em] uppercase"
        >
          media dossier // anilist {summary.anilistId}
        </p>
        {#if freshness}
          <p class="text-muted mt-0.5 truncate text-[0.62rem]">
            Cached {formatTimestamp(freshness.fetchedAt)}
          </p>
        {/if}
      </div>
      <button
        bind:this={closeButton}
        type="button"
        aria-label="Close details"
        class="glass-control glass-interactive control-rounded grid size-10 shrink-0 place-items-center transition-colors"
        onclick={requestClose}
      >
        <span aria-hidden="true">&times;</span>
      </button>
    </header>

    <div
      class="grid min-w-0 gap-5 p-4 sm:grid-cols-[11rem_minmax(0,1fr)] sm:p-6"
    >
      <div class="mx-auto w-full max-w-44">
        <MediaCover title={media.title} coverUrl={media.coverUrl} />
      </div>

      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <p class="text-accent text-[0.66rem] tracking-[0.12em] uppercase">
            {formatLabel(media.mediaFormat)} // {formatLabel(media.status)}
          </p>
          {#if freshness?.isStale}
            <span
              class="control-rounded border-warning/45 bg-warning/10 text-warning border px-2 py-1 text-[0.6rem] tracking-[0.1em] uppercase"
            >
              cached // stale
            </span>
          {:else if freshness}
            <span
              class="control-rounded border-success/45 bg-success/10 text-success border px-2 py-1 text-[0.6rem] tracking-[0.1em] uppercase"
            >
              cached // current
            </span>
          {/if}
        </div>
        <h2
          id="media-details-title"
          class="text-foreground mt-2 text-2xl leading-tight font-light tracking-[0.025em] sm:text-3xl"
        >
          {media.title}
        </h2>
        <p
          id="media-details-description"
          class="text-muted mt-2 text-xs leading-5"
        >
          AniList metadata with your private, device-local tracking controls.
        </p>

        <dl class="border-border mt-5 grid grid-cols-2 gap-4 border-y py-4">
          <div>
            <dt class="text-muted text-[0.62rem] tracking-[0.12em] uppercase">
              episodes
            </dt>
            <dd class="text-foreground mt-1 text-sm">
              {episodeLabel(media.episodes)}
            </dd>
          </div>
          <div>
            <dt class="text-muted text-[0.62rem] tracking-[0.12em] uppercase">
              duration
            </dt>
            <dd class="text-foreground mt-1 text-sm">
              {durationLabel(media.durationMinutes)}
            </dd>
          </div>
        </dl>
      </div>
    </div>

    <div class="border-border grid gap-5 border-t px-4 py-5 sm:px-6">
      {#if detailsLoading}
        <AsyncStateNotice
          status="loading"
          title="Opening the full dossier"
          description="Animi is checking the details cache and AniList when needed."
          compact
        />
      {:else if detailsError}
        <AsyncStateNotice
          status={fallbackFreshness && isOfflineError(detailsError)
            ? "offline"
            : "error"}
          title={fallbackFreshness
            ? "Showing the saved snapshot"
            : "Full details could not load"}
          description={fallbackFreshness
            ? "The latest refresh failed, so this dossier is using metadata already saved on this device."
            : detailsError.message}
          error={detailsError}
          onRetry={detailsError.retryable ? fetchDetails : undefined}
          retryLabel="Retry details"
          compact
        />
      {:else if freshness?.isStale}
        <AsyncStateNotice
          status={freshness.refreshError &&
          isOfflineError(freshness.refreshError)
            ? "offline"
            : "stale"}
          title="Showing cached details"
          description={`Saved ${formatTimestamp(freshness.fetchedAt)}. The latest refresh did not replace this snapshot.`}
          error={freshness.refreshError}
          onRetry={freshness.refreshError?.retryable ? fetchDetails : undefined}
          retryLabel="Retry details"
          compact
        />
      {/if}

      <div class="grid min-w-0 gap-5 lg:grid-cols-2">
        <section aria-labelledby="media-genres-heading">
          <h3
            id="media-genres-heading"
            class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
          >
            genres
          </h3>
          {#if media.genres.length > 0}
            <ul class="mt-2 flex flex-wrap gap-2">
              {#each media.genres as genre (genre)}
                <li
                  class="control-rounded border-border bg-glass-control border px-2 py-1 text-xs"
                >
                  {genre}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="text-muted mt-2 text-xs">
              Genre metadata is unavailable.
            </p>
          {/if}
        </section>

        <section aria-labelledby="media-tags-heading">
          <h3
            id="media-tags-heading"
            class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
          >
            ranked tags
          </h3>
          {#if media.tags.length > 0}
            <ul class="mt-2 grid gap-2">
              {#each media.tags as tag (tag.name)}
                <li
                  class="glass-control control-rounded border-border flex items-center justify-between gap-3 border px-3 py-2 text-xs"
                >
                  <span>{tag.name}</span>
                  <span class="text-accent">rank {tag.rank}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="text-muted mt-2 text-xs">Ranked tags are unavailable.</p>
          {/if}
        </section>
      </div>

      <section aria-labelledby="media-sequels-heading">
        <h3
          id="media-sequels-heading"
          class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
        >
          official sequel relations
        </h3>
        {#if media.sequels.length > 0}
          <ul class="mt-2 grid gap-2 sm:grid-cols-2">
            {#each media.sequels as sequel (sequel.anilistId)}
              <li
                class="glass-control control-rounded border-border border px-3 py-2 text-xs leading-5"
              >
                <span class="text-foreground block">{sequel.title}</span>
                {#if sequel.releaseDate}
                  <time class="text-muted block" datetime={sequel.releaseDate}>
                    Releases {formatCalendarDate(sequel.releaseDate)}
                  </time>
                {:else}
                  <span class="text-muted block">Release date unknown</span>
                {/if}
              </li>
            {/each}
          </ul>
        {:else if detailsResponse}
          <p class="text-muted mt-2 text-xs leading-5">
            No official sequel relation is available.
          </p>
        {:else}
          <p class="text-muted mt-2 text-xs leading-5">
            Sequel relations require the full details response.
          </p>
        {/if}
      </section>

      <section aria-labelledby="local-tracking-heading">
        <div class="flex flex-wrap items-end justify-between gap-2">
          <div>
            <p class="text-accent text-[0.65rem] tracking-[0.12em] uppercase">
              private tracking
            </p>
            <h3
              id="local-tracking-heading"
              class="text-foreground mt-1 text-lg font-light tracking-[0.03em]"
            >
              {currentEntry
                ? "Update your local entry"
                : "Add to your local library"}
            </h3>
          </div>
          {#if currentEntry}
            <span class="text-muted text-xs">
              Last edited {formatTimestamp(currentEntry.updatedAt)}
            </span>
          {/if}
        </div>

        {#if libraryMessage && trackingError === null}
          <p
            role="status"
            class="control-rounded border-success/45 bg-success/10 text-success mt-3 border px-3 py-2 text-xs leading-5"
          >
            {libraryMessage}
          </p>
        {/if}

        {#if trackingError}
          <div class="mt-3">
            <AsyncStateNotice
              status={trackingLoading ? "loading" : "error"}
              title={trackingLoading
                ? "Reading your local library"
                : "Tracking controls are unavailable"}
              description={trackingLoading
                ? "Editing will become available after Animi confirms the current local entry."
                : "Animi could not safely determine the current local entry, so editing is paused."}
              error={trackingLoading ? null : trackingError}
              onRetry={!trackingLoading && trackingError.retryable
                ? onRetryTracking
                : undefined}
              retryLabel="Retry library"
              compact
            />
          </div>
        {:else}
          {#key `${summary.anilistId}:${currentEntry?.updatedAt ?? "new"}`}
            <LibraryEntryEditor
              anilistId={summary.anilistId}
              knownTotal={media.episodes}
              entry={currentEntry}
              {saveEntry}
              {deleteEntry}
              onSaved={handleSaved}
              onRemoved={handleRemoved}
            />
          {/key}
        {/if}
      </section>
    </div>
  </div>
</dialog>

<style>
  .media-details-dialog {
    position: fixed;
    inset: 50% auto auto 50%;
    width: min(58rem, calc(100vw - 2rem));
    height: min(52rem, calc(100dvh - 2rem));
    max-width: none;
    max-height: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    transform: translate(-50%, -50%);
    transform-origin: center;
    animation: dossier-materialize 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .media-details-dialog::backdrop {
    background: var(--scrim);
    -webkit-backdrop-filter: blur(5px);
    backdrop-filter: blur(5px);
    animation: dossier-scrim-enter 180ms ease-out;
  }

  @keyframes dossier-materialize {
    from {
      opacity: 0;
      filter: blur(6px);
      transform: translate(-50%, calc(-50% + 0.75rem)) scale(0.965);
    }

    to {
      opacity: 1;
      filter: blur(0);
      transform: translate(-50%, -50%) scale(1);
    }
  }

  @keyframes dossier-scrim-enter {
    from {
      background: transparent;
      -webkit-backdrop-filter: blur(0);
      backdrop-filter: blur(0);
    }
  }

  @media (max-width: 639px) {
    .media-details-dialog {
      width: calc(100vw - 1rem);
      height: calc(100dvh - 1rem);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .media-details-dialog,
    .media-details-dialog::backdrop {
      animation: none;
    }
  }
</style>
