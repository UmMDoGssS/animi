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
    score?: string;
    notes?: string;
  }

  interface ValidatedDrafts {
    progress: number;
    score: number | null;
  }

  type BusyOperation = "idle" | "saving" | "removing";
  type FailedOperation = "save" | "remove";

  const MAX_PROGRESS = 2_147_483_647;
  const MAX_NOTES_CHARACTERS = 10_000;
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
  let progressDraft = $state("0");
  let scoreDraft = $state("");
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
  let scoreHelpId = $derived(`${fieldIdPrefix}-score-help`);
  let scoreErrorId = $derived(`${fieldIdPrefix}-score-error`);
  let notesHelpId = $derived(`${fieldIdPrefix}-notes-help`);
  let notesErrorId = $derived(`${fieldIdPrefix}-notes-error`);
  let removalConfirmationId = $derived(`${fieldIdPrefix}-removal-confirmation`);
  let removalConfirmationHeadingId = $derived(
    `${fieldIdPrefix}-removal-heading`,
  );

  let notesCharacterCount = $derived(Array.from(notesDraft).length);
  let hasValidationErrors = $derived(
    fieldErrors.progress !== undefined ||
      fieldErrors.score !== undefined ||
      fieldErrors.notes !== undefined,
  );
  let isBusy = $derived(busyOperation !== "idle");

  $effect.pre(() => {
    const nextDraftSource = `${anilistId}:${entry?.updatedAt ?? "new"}`;
    if (initializedDraftSource === nextDraftSource) {
      return;
    }

    initializedDraftSource = nextDraftSource;
    statusDraft = entry?.status ?? "PLANNING";
    progressDraft = entry?.progress.toString() ?? "0";
    scoreDraft = entry?.score?.toString() ?? "";
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
    const trimmedProgress = progressDraft.trim();
    let progress = Number.NaN;

    if (!/^\d+$/.test(trimmedProgress)) {
      nextErrors.progress = "Enter progress as a non-negative whole number.";
    } else {
      progress = Number(trimmedProgress);
      if (!Number.isSafeInteger(progress) || progress > MAX_PROGRESS) {
        nextErrors.progress =
          "Enter a non-negative whole number within Animi's supported range.";
      } else if (knownTotal !== null && progress > knownTotal) {
        nextErrors.progress = `Progress cannot exceed the known total of ${knownTotal} episodes.`;
      }
    }

    const trimmedScore = scoreDraft.trim();
    let score: number | null = null;
    if (trimmedScore.length > 0) {
      score = Number(trimmedScore);
      if (!Number.isFinite(score) || score < 0 || score > 10) {
        nextErrors.score =
          "Enter a score from 0 to 10, or leave this field blank.";
      }
    }

    if (notesCharacterCount > MAX_NOTES_CHARACTERS) {
      nextErrors.notes = "Notes cannot exceed 10,000 characters.";
    }

    fieldErrors = nextErrors;
    operationError = null;
    failedOperation = null;
    successMessage = null;

    if (
      nextErrors.progress !== undefined ||
      nextErrors.score !== undefined ||
      nextErrors.notes !== undefined
    ) {
      return null;
    }

    return { progress, score };
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
          <label
            for={`${fieldIdPrefix}-progress`}
            class="text-foreground block text-sm font-medium"
          >
            Episodes watched
          </label>
          <input
            id={`${fieldIdPrefix}-progress`}
            type="text"
            inputmode="numeric"
            autocomplete="off"
            bind:value={progressDraft}
            aria-invalid={fieldErrors.progress ? "true" : undefined}
            aria-describedby={`${progressHelpId}${fieldErrors.progress ? ` ${progressErrorId}` : ""}`}
            class="glass-control control-rounded border-border text-foreground placeholder:text-muted mt-2 min-h-11 w-full min-w-0 border px-3 py-2 text-sm tabular-nums outline-none"
            class:border-danger={fieldErrors.progress !== undefined}
            oninput={() => clearFeedback("progress")}
          />
          <p id={progressHelpId} class="text-muted mt-2 text-xs leading-5">
            {#if knownTotal === null}
              Episode total is unknown. You can still record progress with a
              non-negative whole number.
            {:else}
              Known episode total: {knownTotal}. Progress must stay between 0
              and {knownTotal}.
            {/if}
          </p>
          {#if fieldErrors.progress}
            <p id={progressErrorId} class="text-danger mt-1 text-xs leading-5">
              {fieldErrors.progress}
            </p>
          {/if}
        </div>

        <div class="min-w-0">
          <label
            for={`${fieldIdPrefix}-score`}
            class="text-foreground block text-sm font-medium"
          >
            Personal score <span class="text-muted font-normal">(optional)</span
            >
          </label>
          <input
            id={`${fieldIdPrefix}-score`}
            type="text"
            inputmode="decimal"
            autocomplete="off"
            bind:value={scoreDraft}
            aria-invalid={fieldErrors.score ? "true" : undefined}
            aria-describedby={`${scoreHelpId}${fieldErrors.score ? ` ${scoreErrorId}` : ""}`}
            placeholder="0 to 10"
            class="glass-control control-rounded border-border text-foreground placeholder:text-muted mt-2 min-h-11 w-full min-w-0 border px-3 py-2 text-sm tabular-nums outline-none"
            class:border-danger={fieldErrors.score !== undefined}
            oninput={() => clearFeedback("score")}
          />
          <p id={scoreHelpId} class="text-muted mt-2 text-xs leading-5">
            Use any number from 0 to 10, or leave this blank.
          </p>
          {#if fieldErrors.score}
            <p id={scoreErrorId} class="text-danger mt-1 text-xs leading-5">
              {fieldErrors.score}
            </p>
          {/if}
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
</style>
