<script lang="ts">
  import { normalizeCommandError } from "$lib/ipc/client";
  import type {
    ClearLocalDataRequest,
    ClearLocalDataResponse,
    CommandError,
    ExportBackupResponse,
    ImportBackupRequest,
    ImportBackupResponse,
    ImportPreviewResponse,
    ThemePreference,
  } from "$lib/ipc/types";

  import ThemePreferenceControl from "../ThemePreferenceControl.svelte";

  const CLEAR_CONFIRMATION = "CLEAR MY LOCAL DATA";

  interface Props {
    themePreference: ThemePreference;
    themeStatus: "loading" | "ready" | "saving" | "error";
    themeError: CommandError | null;
    selectThemePreference: (preference: ThemePreference) => Promise<void>;
    retryThemePreference: () => Promise<void>;
    saveBackup: () => Promise<ExportBackupResponse | null>;
    chooseImport: () => Promise<ImportPreviewResponse | null>;
    restoreBackup: (
      request: ImportBackupRequest,
    ) => Promise<ImportBackupResponse>;
    clearData: (
      request: ClearLocalDataRequest,
    ) => Promise<ClearLocalDataResponse>;
    localStateChanged: () => Promise<void>;
  }

  let {
    themePreference,
    themeStatus,
    themeError,
    selectThemePreference,
    retryThemePreference,
    saveBackup,
    chooseImport,
    restoreBackup,
    clearData,
    localStateChanged,
  }: Props = $props();

  let exportStatus = $state<"idle" | "working" | "success" | "error">("idle");
  let exportResult = $state<ExportBackupResponse | null>(null);
  let exportError = $state<CommandError | null>(null);
  let importStatus = $state<
    "idle" | "previewing" | "applying" | "success" | "error"
  >("idle");
  let importPreview = $state<ImportPreviewResponse | null>(null);
  let importResult = $state<ImportBackupResponse | null>(null);
  let importError = $state<CommandError | null>(null);
  let clearExpanded = $state(false);
  let clearConfirmation = $state("");
  let clearStatus = $state<"idle" | "working" | "success" | "error">("idle");
  let clearResult = $state<ClearLocalDataResponse | null>(null);
  let clearError = $state<CommandError | null>(null);

  const clearIsConfirmed = $derived(clearConfirmation === CLEAR_CONFIRMATION);

  async function exportNow() {
    exportStatus = "working";
    exportResult = null;
    exportError = null;
    try {
      const result = await saveBackup();
      if (result === null) {
        exportStatus = "idle";
        return;
      }
      exportResult = result;
      exportStatus = "success";
    } catch (reason) {
      exportError = normalizeCommandError(reason);
      exportStatus = "error";
    }
  }

  async function previewRestore() {
    importStatus = "previewing";
    importPreview = null;
    importResult = null;
    importError = null;
    try {
      const preview = await chooseImport();
      if (preview === null) {
        importStatus = "idle";
        return;
      }
      importPreview = preview;
      importStatus = "idle";
    } catch (reason) {
      importError = normalizeCommandError(reason);
      importStatus = "error";
    }
  }

  async function applyRestore() {
    const token = importPreview?.importToken;
    if (!importPreview?.canApply || token === null || token === undefined)
      return;

    importStatus = "applying";
    importError = null;
    try {
      importResult = await restoreBackup({ importToken: token });
      importStatus = "success";
      importPreview = null;
      await localStateChanged();
    } catch (reason) {
      importError = normalizeCommandError(reason);
      importStatus = "error";
    }
  }

  async function clearEverything() {
    if (!clearIsConfirmed) return;
    clearStatus = "working";
    clearResult = null;
    clearError = null;
    try {
      clearResult = await clearData({ confirmation: clearConfirmation });
      clearStatus = "success";
      clearExpanded = false;
      clearConfirmation = "";
      importPreview = null;
      await localStateChanged();
    } catch (reason) {
      clearError = normalizeCommandError(reason);
      clearStatus = "error";
    }
  }
</script>

<div
  id="settings-panel"
  role="tabpanel"
  aria-labelledby="settings-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <h1 class="sr-only">Settings</h1>

  <ThemePreferenceControl
    preference={themePreference}
    status={themeStatus}
    error={themeError}
    selectPreference={selectThemePreference}
    retryPreference={retryThemePreference}
  />

  <section
    aria-labelledby="backup-recovery-heading"
    class="glass-panel panel-large border-border grid min-w-0 overflow-hidden border lg:grid-cols-[minmax(0,0.78fr)_minmax(20rem,1.22fr)]"
  >
    <div class="min-w-0 p-5 sm:p-6 lg:p-7">
      <h2
        id="backup-recovery-heading"
        class="text-foreground text-xl font-light tracking-[0.03em]"
      >
        Backup &amp; recovery
      </h2>
      <p class="text-muted mt-3 text-xs leading-5 sm:text-sm sm:leading-6">
        Export a backup or restore one. You'll review the changes before
        anything is updated.
      </p>
      <div class="mt-5 grid gap-3">
        <button
          type="button"
          onclick={exportNow}
          disabled={exportStatus === "working" || importStatus === "applying"}
          class="glass-accent control-rounded min-h-11 px-4 py-2.5 text-xs tracking-[0.04em] disabled:cursor-wait disabled:opacity-60"
        >
          {exportStatus === "working" ? "Exporting backup…" : "Export backup"}
        </button>
        <button
          type="button"
          onclick={previewRestore}
          disabled={importStatus === "previewing" ||
            importStatus === "applying"}
          class="glass-control glass-interactive control-rounded border-border min-h-11 border px-4 py-2.5 text-xs tracking-[0.04em] disabled:cursor-wait disabled:opacity-60"
        >
          {importStatus === "previewing"
            ? "Validating backup…"
            : "Choose backup to restore"}
        </button>
      </div>
      {#if exportStatus === "success" && exportResult}
        <p
          role="status"
          class="control-rounded border-success/50 bg-success/10 text-success mt-4 border p-3 text-xs leading-5"
        >
          Backup exported: {exportResult.libraryEntryCount} library titles and {exportResult.watchActivityCount}
          watch history updates.
        </p>
      {:else if exportStatus === "error" && exportError}
        <p
          role="alert"
          class="control-rounded border-danger/60 bg-danger/10 text-danger mt-4 border p-3 text-xs leading-5"
        >
          {exportError.message}
        </p>
      {/if}
      {#if importStatus === "error" && importError}
        <p
          role="alert"
          class="control-rounded border-danger/60 bg-danger/10 text-danger mt-4 border p-3 text-xs leading-5"
        >
          {importError.message}
        </p>
      {:else if importStatus === "success" && importResult}
        <p
          role="status"
          class="control-rounded border-success/50 bg-success/10 text-success mt-4 border p-3 text-xs leading-5"
        >
          Recovery complete. Added {importResult.changes.libraryAdded} and updated
          {importResult.changes.libraryMerged} library titles.
        </p>
      {/if}
    </div>

    <div
      class="border-border min-w-0 border-t p-5 sm:p-6 lg:border-t-0 lg:border-l lg:p-7"
    >
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-accent text-[0.65rem] tracking-[0.14em] uppercase">
            restore preview
          </p>
          <h3
            class="text-foreground mt-2 text-sm font-medium tracking-[0.03em]"
          >
            What restore will do
          </h3>
        </div>
        <span aria-hidden="true" class="text-accent text-xs">ฅ^•ﻌ•^ฅ</span>
      </div>

      {#if importPreview}
        <div class="mt-4" aria-live="polite">
          <p class="text-foreground truncate text-sm">
            {importPreview.fileName}
          </p>
          <p class="text-muted mt-1 text-xs">
            {importPreview.libraryEntryCount} library titles · {importPreview.watchActivityCount}
            watch history updates
          </p>
          {#if importPreview.validationErrors.length > 0}
            <div
              role="alert"
              class="control-rounded border-danger/60 bg-danger/10 mt-4 border p-4"
            >
              <p class="text-danger text-xs font-medium">
                This backup cannot be restored.
              </p>
              <ul class="text-danger mt-2 grid gap-2 text-xs leading-5">
                {#each importPreview.validationErrors as issue (issue.code + (issue.path ?? issue.message))}
                  <li>{issue.message}</li>
                {/each}
              </ul>
            </div>
          {:else}
            <dl
              role="group"
              aria-label="Import change preview"
              class="mt-4 grid grid-cols-2 gap-px overflow-hidden rounded-lg bg-[var(--border)] sm:grid-cols-3"
            >
              <div class="bg-surface/85 p-3">
                <dt class="text-muted text-[0.62rem] uppercase">new titles</dt>
                <dd class="text-success mt-1 text-xl font-light tabular-nums">
                  {importPreview.changes.libraryAdded}
                </dd>
              </div>
              <div class="bg-surface/85 p-3">
                <dt class="text-muted text-[0.62rem] uppercase">
                  updated titles
                </dt>
                <dd class="text-accent mt-1 text-xl font-light tabular-nums">
                  {importPreview.changes.libraryMerged}
                </dd>
              </div>
              <div class="bg-surface/85 p-3">
                <dt class="text-muted text-[0.62rem] uppercase">
                  history updates
                </dt>
                <dd class="text-success mt-1 text-xl font-light tabular-nums">
                  {importPreview.changes.activityAdded}
                </dd>
              </div>
            </dl>
            <p
              role="note"
              aria-label="Recovery safety guarantee"
              class="control-rounded border-border bg-glass-control text-muted mt-4 border p-3 text-xs leading-5"
            >
              Titles that are not in this backup will stay in your library.
            </p>
            <button
              type="button"
              onclick={applyRestore}
              disabled={!importPreview.canApply || importStatus === "applying"}
              class="glass-accent control-rounded mt-4 min-h-11 w-full px-4 py-2.5 text-xs font-medium disabled:cursor-wait disabled:opacity-60"
            >
              {importStatus === "applying"
                ? "Restoring backup…"
                : "Restore this backup"}
            </button>
          {/if}
        </div>
      {:else}
        <div
          class="control-rounded border-border bg-glass-control text-muted mt-4 border p-4 text-xs leading-5"
        >
          Choose a backup to preview its anime library and watch history.
        </div>
      {/if}
    </div>
  </section>

  <section
    aria-labelledby="clear-local-data-heading"
    class="glass-panel panel-large border-danger/60 min-w-0 border p-5 sm:p-6"
  >
    <div class="grid gap-5 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
      <div class="min-w-0">
        <h2
          id="clear-local-data-heading"
          class="text-foreground text-xl font-light tracking-[0.03em]"
        >
          Clear local data
        </h2>
        <p
          class="text-muted mt-3 max-w-3xl text-xs leading-5 sm:text-sm sm:leading-6"
        >
          Permanently removes your library, preferences, and watch history.
          Export a backup first if you may want them again.
        </p>
      </div>
      <button
        type="button"
        aria-expanded={clearExpanded}
        aria-controls="clear-confirmation-panel"
        onclick={() => {
          clearExpanded = !clearExpanded;
          clearError = null;
        }}
        class="control-rounded border-danger/70 bg-danger/10 text-danger min-h-11 w-full border px-4 py-2.5 text-xs tracking-[0.04em] sm:w-auto"
      >
        {clearExpanded ? "Cancel clear" : "Clear local data"}
      </button>
    </div>

    {#if clearExpanded}
      <div
        id="clear-confirmation-panel"
        class="control-rounded border-danger/70 bg-danger/10 mt-5 border p-4 sm:p-5"
      >
        <label
          for="clear-confirmation"
          class="text-foreground text-sm font-medium"
          >Type <span class="text-danger">{CLEAR_CONFIRMATION}</span> to confirm</label
        >
        <input
          id="clear-confirmation"
          bind:value={clearConfirmation}
          autocomplete="off"
          spellcheck="false"
          class="glass-control control-rounded border-danger/60 text-foreground placeholder:text-muted mt-3 min-h-11 w-full border px-3 py-2 text-sm outline-none"
          placeholder={CLEAR_CONFIRMATION}
        />
        <div class="mt-4 flex flex-col gap-3 sm:flex-row sm:justify-end">
          <button
            type="button"
            onclick={() => {
              clearExpanded = false;
              clearConfirmation = "";
            }}
            class="glass-control glass-interactive control-rounded border-border min-h-11 border px-4 py-2.5 text-xs"
            >Keep my data</button
          >
          <button
            type="button"
            onclick={clearEverything}
            disabled={!clearIsConfirmed || clearStatus === "working"}
            class="control-rounded bg-danger text-surface min-h-11 px-4 py-2.5 text-xs font-medium disabled:cursor-not-allowed disabled:opacity-50"
            >{clearStatus === "working"
              ? "Clearing local data…"
              : "Permanently clear local data"}</button
          >
        </div>
      </div>
    {/if}
    {#if clearStatus === "error" && clearError}
      <p role="alert" class="text-danger mt-4 text-xs leading-5">
        {clearError.message}
      </p>
    {:else if clearStatus === "success" && clearResult}
      <p role="status" class="text-success mt-4 text-xs leading-5">
        Removed {clearResult.libraryEntriesRemoved} library titles and {clearResult.watchActivityRemoved}
        watch history updates.
      </p>
    {/if}
  </section>
</div>
