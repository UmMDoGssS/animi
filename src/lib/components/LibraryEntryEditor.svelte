<script lang="ts">
  import { tick } from "svelte";

  import {
    normalizeCommandError,
    removeLibraryEntry,
    upsertLibraryEntry,
  } from "$lib/ipc/client";
  import type {
    CommandError,
    LibraryEntry,
    LibraryStatus,
    RemoveLibraryEntryRequest,
    RemoveLibraryEntryResponse,
    UpsertLibraryEntryRequest,
  } from "$lib/ipc/types";

  interface Props {
    anilistId: number;
    knownTotal: number | null;
    entry: LibraryEntry | null;
    saveEntry?: (request: UpsertLibraryEntryRequest) => Promise<LibraryEntry>;
    deleteEntry?: (
      request: RemoveLibraryEntryRequest,
    ) => Promise<RemoveLibraryEntryResponse>;
    onSaved?: (savedEntry: LibraryEntry) => void;
    onRemoved?: () => void;
  }

  interface FieldErrors {
    progress?: string;
    notes?: string;
  }

  interface ValidatedDrafts {
    progress: number;
    score: number | null;
  }

  type BusyOperation = "idle" | "saving" | "removing";
  type FailedOperation = "save" | "remove";

  const MAX_PROGRESS = 2_147_483_647;
  const DEFAULT_UNKNOWN_PROGRESS_MAXIMUM = 2_000;
  const MAX_NOTES_CHARACTERS = 10_000;
  const scoreOptions = Array.from({ length: 10 }, (_, index) => index + 1);
  const statusOptions: readonly {
    value: LibraryStatus;
    label: string;
    description: string;
  }[] = [
    {
      value: "WATCHING",
      label: "Watching",
      description: "In progress now.",
    },
    {
      value: "COMPLETED",
      label: "Completed",
      description: "All episodes finished.",
    },
    {
      value: "PLANNING",
      label: "Planning",
      description: "Saved for later.",
    },
  ];

  let {
    anilistId,
    knownTotal,
    entry,
    saveEntry = upsertLibraryEntry,
    deleteEntry = removeLibraryEntry,
    onSaved,
    onRemoved,
  }: Props = $props();

  let statusDraft = $state<LibraryStatus>("PLANNING");
  let progressDraft = $state(0);
  let scoreDraft = $state<number | null>(null);
  let unknownProgressMaximum = $state(DEFAULT_UNKNOWN_PROGRESS_MAXIMUM);
  let notesDraft = $state("");
  let fieldErrors = $state<FieldErrors>({});
  let operationError = $state<CommandError | null>(null);
  let failedOperation = $state<FailedOperation | null>(null);
  let successMessage = $state<string | null>(null);
  let busyOperation = $state<BusyOperation>("idle");
  let removalConfirmationVisible = $state(false);
  let removeButton = $state<HTMLButtonElement | null>(null);
  let confirmRemoveButton = $state<HTMLButtonElement | null>(null);
  let initializedDraftSource = $state<string | null>(null);

  let fieldIdPrefix = $derived(`library-entry-${anilistId}`);
  let headingId = $derived(`${fieldIdPrefix}-heading`);
  let progressHelpId = $derived(`${fieldIdPrefix}-progress-help`);
  let progressErrorId = $derived(`${fieldIdPrefix}-progress-error`);
  let notesHelpId = $derived(`${fieldIdPrefix}-notes-help`);
  let notesErrorId = $derived(`${fieldIdPrefix}-notes-error`);
  let removalConfirmationId = $derived(`${fieldIdPrefix}-removal-confirmation`);
  let removalConfirmationHeadingId = $derived(
    `${fieldIdPrefix}-removal-heading`,
  );

  let notesCharacterCount = $derived(Array.from(notesDraft).length);
  let hasValidationErrors = $derived(
    fieldErrors.progress !== undefined || fieldErrors.notes !== undefined,
  );
  let isBusy = $derived(busyOperation !== "idle");
  let progressMaximum = $derived(knownTotal ?? unknownProgressMaximum);
  let progressFill = $derived(
    progressMaximum === 0 ? 0 : (progressDraft / progressMaximum) * 100,
  );

  $effect.pre(() => {
    const nextDraftSource = `${anilistId}:${entry?.updatedAt ?? "new"}`;
    if (initializedDraftSource === nextDraftSource) {
      return;
    }

    initializedDraftSource = nextDraftSource;
    statusDraft = entry?.status ?? "PLANNING";
    progressDraft = entry?.progress ?? 0;
    scoreDraft = entry?.score ?? null;
    unknownProgressMaximum = Math.max(
      DEFAULT_UNKNOWN_PROGRESS_MAXIMUM,
      entry?.progress ?? 0,
    );
    notesDraft = entry?.notes ?? "";
    fieldErrors = {};
    operationError = null;
    failedOperation = null;
    successMessage = null;
    busyOperation = "idle";
    removalConfirmationVisible = false;
  });

  function clearFeedback(field?: keyof FieldErrors): void {
    if (field !== undefined && fieldErrors[field] !== undefined) {
      fieldErrors = { ...fieldErrors, [field]: undefined };
    }
    operationError = null;
    failedOperation = null;
    successMessage = null;
  }

  function validateDrafts(): ValidatedDrafts | null {
    const nextErrors: FieldErrors = {};
    const progress = Number(progressDraft);

    if (!Number.isSafeInteger(progress) || progress < 0) {
      nextErrors.progress = "Enter progress as a non-negative whole number.";
    } else if (progress > MAX_PROGRESS) {
      nextErrors.progress =
        "Enter a non-negative whole number within Animi's supported range.";
    } else if (knownTotal !== null && progress > knownTotal) {
      nextErrors.progress = `Progress cannot exceed the known total of ${knownTotal} episodes.`;
    }

    if (notesCharacterCount > MAX_NOTES_CHARACTERS) {
      nextErrors.notes = "Notes cannot exceed 10,000 characters.";
    }

    fieldErrors = nextErrors;
    operationError = null;
    failedOperation = null;
    successMessage = null;

    if (nextErrors.progress !== undefined || nextErrors.notes !== undefined) {
      return null;
    }

    return { progress, score: scoreDraft };
  }

  function updateProgress(value: string): void {
    progressDraft = Number(value);
    clearFeedback("progress");
  }

  function adjustProgress(delta: number): void {
    const nextProgress = Math.max(
      0,
      Math.min(knownTotal ?? MAX_PROGRESS, progressDraft + delta),
    );
    if (knownTotal === null && nextProgress > unknownProgressMaximum) {
      unknownProgressMaximum = nextProgress;
    }
    progressDraft = nextProgress;
    clearFeedback("progress");
  }

  function setScore(score: number | null): void {
    scoreDraft = score;
    clearFeedback();
  }

  function starFill(star: number): number {
    return Math.max(0, Math.min(1, (scoreDraft ?? 0) - (star - 1)));
  }

  async function submitEntry(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (isBusy) {
      return;
    }

    const validatedDrafts = validateDrafts();
    if (validatedDrafts === null) {
      return;
    }

    const wasEditing = entry !== null;
    busyOperation = "saving";
    try {
      const savedEntry = await saveEntry({
        anilistId,
        mediaKind: entry?.mediaKind ?? "ANIME",
        status: statusDraft,
        progress: validatedDrafts.progress,
        score: validatedDrafts.score,
        notes: notesDraft,
      });
      busyOperation = "idle";
      successMessage = wasEditing ? "Changes saved." : "Added to your library.";
      onSaved?.(savedEntry);
    } catch (reason) {
      busyOperation = "idle";
      operationError = normalizeCommandError(reason);
      failedOperation = "save";
    }
  }

  async function showRemovalConfirmation(): Promise<void> {
    clearFeedback();
    removalConfirmationVisible = true;
    await tick();
    confirmRemoveButton?.focus();
  }

  async function cancelRemoval(): Promise<void> {
    removalConfirmationVisible = false;
    operationError = null;
    failedOperation = null;
    await tick();
    removeButton?.focus();
  }

  async function confirmRemoval(): Promise<void> {
    if (entry === null || isBusy) {
      return;
    }

    operationError = null;
    failedOperation = null;
    successMessage = null;
    busyOperation = "removing";
    try {
      const response = await deleteEntry({
        anilistId,
        mediaKind: entry.mediaKind,
        confirmed: true,
      });
      busyOperation = "idle";

      if (!response.removed) {
        operationError = {
          code: "LIBRARY_ENTRY_NOT_FOUND",
          message: "This title is no longer in your library.",
          retryable: false,
        };
        failedOperation = "remove";
        return;
      }

      removalConfirmationVisible = false;
      successMessage = "Removed from your library.";
      onRemoved?.();
    } catch (reason) {
      busyOperation = "idle";
      operationError = normalizeCommandError(reason);
      failedOperation = "remove";
    }
  }
</script>

<section
  aria-labelledby={headingId}
  aria-busy={isBusy}
  class="glass-panel panel-large border-border min-w-0 border p-4 sm:p-6"
>
  <header class="border-border border-b pb-4">
    <h2
      id={headingId}
      class="text-foreground text-lg font-light tracking-[0.03em] sm:text-xl"
    >
      {entry === null ? "Add to your library" : "Edit library entry"}
    </h2>
  </header>

  <form class="mt-5 grid min-w-0 gap-5" novalidate onsubmit={submitEntry}>
    <fieldset disabled={isBusy} class="grid min-w-0 gap-5">
      <legend class="text-foreground text-sm font-medium">Library status</legend
      >
      <div class="mt-3 grid gap-2 sm:grid-cols-3">
        {#each statusOptions as option (option.value)}
          <label
            class="status-option glass-control glass-interactive control-rounded border-border has-checked:border-accent has-checked:ring-accent/30 flex min-w-0 cursor-pointer gap-3 border p-3 transition has-checked:ring-2"
          >
            <input
              type="radio"
              name={`${fieldIdPrefix}-status`}
              value={option.value}
              bind:group={statusDraft}
              class="accent-accent mt-0.5"
              onchange={() => clearFeedback()}
            />
            <span class="min-w-0">
              <span class="text-foreground block text-sm font-medium">
                {option.label}
              </span>
              <span class="text-muted mt-0.5 block text-xs leading-5">
                {option.description}
              </span>
            </span>
          </label>
        {/each}
      </div>

      <div class="grid min-w-0 gap-4 sm:grid-cols-2">
        <div class="min-w-0">
          <div class="flex items-center justify-between gap-3">
            <label
              for={`${fieldIdPrefix}-progress`}
              class="text-foreground block text-sm font-medium"
            >
              Episodes watched
            </label>
            <span
              class="glass-control control-rounded border-border text-foreground border px-2.5 py-1 text-xs tabular-nums"
            >
              {progressDraft}{knownTotal === null ? "" : ` / ${knownTotal}`}
            </span>
          </div>
          <div
            class="mt-3 grid grid-cols-[2.75rem_minmax(0,1fr)_2.75rem] items-center gap-2"
          >
            <button
              type="button"
              aria-label="Decrease episodes watched"
              class="glass-control glass-interactive control-rounded border-border text-foreground min-h-11 border text-lg disabled:cursor-not-allowed disabled:opacity-40"
              disabled={progressDraft === 0}
              onclick={() => adjustProgress(-1)}
            >
              −
            </button>
            <input
              id={`${fieldIdPrefix}-progress`}
              type="range"
              min="0"
              max={progressMaximum}
              step="1"
              value={progressDraft}
              aria-invalid={fieldErrors.progress ? "true" : undefined}
              aria-describedby={`${progressHelpId}${fieldErrors.progress ? ` ${progressErrorId}` : ""}`}
              class="episode-slider w-full min-w-0 cursor-pointer"
              style={`--progress-fill: ${progressFill}%`}
              oninput={(event) => updateProgress(event.currentTarget.value)}
            />
            <button
              type="button"
              aria-label="Increase episodes watched"
              class="glass-control glass-interactive control-rounded border-border text-foreground min-h-11 border text-lg disabled:cursor-not-allowed disabled:opacity-40"
              disabled={knownTotal !== null && progressDraft === knownTotal}
              onclick={() => adjustProgress(1)}
            >
              +
            </button>
          </div>
          <p id={progressHelpId} class="text-muted mt-2 text-xs leading-5">
            {#if knownTotal === null}
              Episode total is unknown. Use the slider, arrow keys, or step
              buttons to record progress. Saving nonzero progress adds those
              episodes to your watch history.
            {:else}
              Known episode total: {knownTotal}. Saving nonzero progress adds
              those episodes to your watch history.
            {/if}
          </p>
          {#if fieldErrors.progress}
            <p id={progressErrorId} class="text-danger mt-1 text-xs leading-5">
              {fieldErrors.progress}
            </p>
          {/if}
        </div>

        <div class="min-w-0">
          <div class="flex items-center justify-between gap-3">
            <span class="text-foreground text-sm font-medium">
              Personal score
              <span class="text-muted font-normal">(optional)</span>
            </span>
            <span class="text-warning text-xs tabular-nums" aria-live="polite">
              {scoreDraft === null ? "Not rated" : `${scoreDraft} / 10`}
            </span>
          </div>
          <div
            role="group"
            aria-label="Personal score"
            class="score-stars mt-3 grid grid-cols-10 gap-0.5"
          >
            {#each scoreOptions as score (score)}
              <button
                type="button"
                aria-label={`Set personal score to ${score} out of 10`}
                aria-pressed={scoreDraft === score}
                class="star-button control-rounded relative grid min-h-10 min-w-0 place-items-center"
                onclick={() => setScore(score)}
              >
                <span aria-hidden="true" class="star-shell">
                  <span class="star-empty">☆</span>
                  <span
                    class="star-fill"
                    style={`width: ${starFill(score) * 100}%`}>★</span
                  >
                </span>
              </button>
            {/each}
          </div>
          <button
            type="button"
            class="text-muted hover:text-warning mt-2 text-xs underline decoration-dotted underline-offset-4 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={scoreDraft === null}
            onclick={() => setScore(null)}
          >
            Clear rating
          </button>
        </div>
      </div>

      <div class="min-w-0">
        <div class="flex flex-wrap items-end justify-between gap-2">
          <label
            for={`${fieldIdPrefix}-notes`}
            class="text-foreground text-sm font-medium"
          >
            Private notes <span class="text-muted font-normal">(optional)</span>
          </label>
          <span
            class:text-danger={notesCharacterCount > MAX_NOTES_CHARACTERS}
            class="text-muted text-xs tabular-nums"
          >
            {notesCharacterCount.toLocaleString()} / 10,000
          </span>
        </div>
        <textarea
          id={`${fieldIdPrefix}-notes`}
          rows="4"
          bind:value={notesDraft}
          aria-invalid={fieldErrors.notes ? "true" : undefined}
          aria-describedby={`${notesHelpId}${fieldErrors.notes ? ` ${notesErrorId}` : ""}`}
          placeholder="A thought to keep with this title"
          class="glass-control control-rounded border-border text-foreground placeholder:text-muted mt-2 min-h-28 w-full min-w-0 resize-y border px-3 py-2 text-sm leading-6 outline-none"
          class:border-danger={fieldErrors.notes !== undefined}
          oninput={() => clearFeedback("notes")}></textarea>
        <p id={notesHelpId} class="text-muted mt-2 text-xs leading-5">
          Up to 10,000 characters.
        </p>
        {#if fieldErrors.notes}
          <p id={notesErrorId} class="text-danger mt-1 text-xs leading-5">
            {fieldErrors.notes}
          </p>
        {/if}
      </div>
    </fieldset>

    {#if hasValidationErrors}
      <p
        role="alert"
        class="control-rounded border-danger/60 bg-danger/10 text-danger border p-3 text-xs leading-5"
      >
        Check the highlighted fields. Your other edits are still here.
      </p>
    {/if}

    {#if operationError && failedOperation}
      <section
        role="alert"
        data-error-code={operationError.code}
        class="control-rounded border-danger/60 bg-danger/10 min-w-0 border p-3"
      >
        <p class="text-danger text-[0.65rem] tracking-[0.12em] uppercase">
          {failedOperation === "save"
            ? "changes could not be saved"
            : "anime could not be removed"}
        </p>
        <p class="text-foreground mt-1 text-sm leading-6">
          {operationError.message}
        </p>
        <p class="text-muted mt-1 text-xs leading-5">
          Your status, progress, score, and notes remain unchanged in this form.
        </p>
      </section>
    {/if}

    {#if successMessage}
      <p
        role="status"
        class="control-rounded border-success/50 bg-success/10 text-success border p-3 text-xs leading-5"
      >
        {successMessage}
      </p>
    {/if}

    <div
      class="border-border flex flex-col gap-3 border-t pt-4 sm:flex-row sm:items-center sm:justify-between"
    >
      <button
        type="submit"
        class="glass-accent control-rounded min-h-11 px-5 py-2.5 text-sm font-medium transition disabled:cursor-wait disabled:opacity-60"
        disabled={isBusy}
      >
        {#if busyOperation === "saving"}
          {entry === null ? "Adding…" : "Saving…"}
        {:else}
          {entry === null ? "Add to library" : "Save changes"}
        {/if}
      </button>

      {#if entry !== null && !removalConfirmationVisible}
        <button
          bind:this={removeButton}
          type="button"
          aria-expanded="false"
          aria-controls={removalConfirmationId}
          class="control-rounded border-danger/60 bg-danger/10 text-danger hover:bg-danger/15 min-h-11 px-4 py-2.5 text-sm transition disabled:cursor-wait disabled:opacity-60"
          disabled={isBusy}
          onclick={() => void showRemovalConfirmation()}
        >
          Remove from library
        </button>
      {/if}
    </div>

    {#if entry !== null && removalConfirmationVisible}
      <section
        id={removalConfirmationId}
        aria-labelledby={removalConfirmationHeadingId}
        class="control-rounded border-danger/70 bg-danger/10 min-w-0 border p-4"
      >
        <p class="text-danger text-[0.65rem] tracking-[0.14em] uppercase">
          confirm removal
        </p>
        <h3
          id={removalConfirmationHeadingId}
          class="text-foreground mt-1 text-base font-medium"
        >
          Remove this library entry?
        </h3>
        <p class="text-muted mt-2 text-sm leading-6">
          This removes the title from your library. Your watch history remains.
        </p>
        <div class="mt-4 grid gap-2 sm:flex sm:flex-wrap">
          <button
            bind:this={confirmRemoveButton}
            type="button"
            class="control-rounded border-danger bg-danger text-surface min-h-11 px-4 py-2.5 text-sm font-medium transition hover:opacity-90 disabled:cursor-wait disabled:opacity-60"
            disabled={isBusy}
            onclick={() => void confirmRemoval()}
          >
            {busyOperation === "removing" ? "Removing…" : "Remove from library"}
          </button>
          <button
            type="button"
            class="glass-control glass-interactive control-rounded border-border text-foreground min-h-11 border px-4 py-2.5 text-sm transition disabled:cursor-wait disabled:opacity-60"
            disabled={isBusy}
            onclick={() => void cancelRemoval()}
          >
            Keep entry
          </button>
        </div>
      </section>
    {/if}
  </form>
</section>

<style>
  .status-option:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }

  .episode-slider {
    height: 0.5rem;
    appearance: none;
    border-radius: 999px;
    background: linear-gradient(
      90deg,
      var(--accent) 0 var(--progress-fill),
      var(--surface-sunken) var(--progress-fill) 100%
    );
    box-shadow: inset 0 0 0 1px var(--control-border);
  }

  .episode-slider::-webkit-slider-thumb {
    width: 1.25rem;
    height: 1.25rem;
    appearance: none;
    border: 2px solid var(--surface-raised);
    border-radius: 999px;
    background: var(--accent);
    box-shadow:
      0 0 0 1px var(--control-border),
      var(--control-shadow);
  }

  .episode-slider::-moz-range-thumb {
    width: 1.1rem;
    height: 1.1rem;
    border: 2px solid var(--surface-raised);
    border-radius: 999px;
    background: var(--accent);
    box-shadow:
      0 0 0 1px var(--control-border),
      var(--control-shadow);
  }

  .star-button:hover .star-empty,
  .star-button:focus-visible .star-empty {
    color: var(--warning);
  }

  .star-shell {
    position: relative;
    display: inline-block;
    font-size: clamp(1.1rem, 4vw, 1.45rem);
    line-height: 1;
  }

  .star-empty {
    color: var(--text-muted);
    transition: color 120ms ease;
  }

  .star-fill {
    position: absolute;
    inset: 0 auto 0 0;
    overflow: hidden;
    color: var(--warning);
    white-space: nowrap;
  }
</style>
