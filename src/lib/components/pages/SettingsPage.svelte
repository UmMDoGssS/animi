<script lang="ts">
  import { onMount } from "svelte";

  import { normalizeCommandError } from "$lib/ipc/client";
  import type {
    ClearLocalDataRequest,
    ClearLocalDataResponse,
    CommandError,
    ExportBackupResponse,
    ImportBackupRequest,
    ImportBackupResponse,
    ImportPreviewResponse,
    SettingsOverview,
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
    loadOverview: () => Promise<SettingsOverview>;
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
    loadOverview,
    saveBackup,
    chooseImport,
    restoreBackup,
    clearData,
    localStateChanged,
  }: Props = $props();

  let overview = $state<SettingsOverview | null>(null);
  let overviewStatus = $state<"loading" | "ready" | "error">("loading");
  let overviewError = $state<CommandError | null>(null);
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

  onMount(() => {
    void refreshOverview();
  });

  async function refreshOverview() {
    overviewStatus = "loading";
    overviewError = null;
    try {
      overview = await loadOverview();
      overviewStatus = "ready";
    } catch (reason) {
      overviewError = normalizeCommandError(reason);
      overviewStatus = "error";
    }
  }

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
      await Promise.all([refreshOverview(), localStateChanged()]);
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
      await Promise.all([refreshOverview(), localStateChanged()]);
    } catch (reason) {
      clearError = normalizeCommandError(reason);
      clearStatus = "error";
    }
  }

  function formatTimestamp(value: string | null): string {
    if (value === null) return "No cached data yet";
    const timestamp = new Date(value);
    return Number.isNaN(timestamp.getTime())
      ? "Unknown cache time"
      : timestamp.toLocaleString(undefined, {
          dateStyle: "medium",
          timeStyle: "short",
        });
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

  <aside
    role="note"
    aria-label="Local settings disclosure"
    class="glass-control control-rounded border-border flex min-w-0 flex-col gap-3 border p-4 sm:flex-row sm:items-center sm:p-5"
  >
    <span
      class="control-rounded border-success/45 bg-success/10 text-success w-fit shrink-0 border px-2.5 py-1 text-[0.65rem] tracking-[0.14em] uppercase"
    >
      device only
    </span>
    <p class="text-muted text-xs leading-5 sm:text-sm sm:leading-6">
      Settings, backups, and recovery run through Animi's local Rust core. No
      account, cloud copy, or analytics service receives this data.
    </p>
  </aside>

  <div
    class="grid min-w-0 gap-4 lg:grid-cols-[minmax(0,1.35fr)_minmax(17rem,0.65fr)] lg:gap-5"
  >
    <ThemePreferenceControl
      preference={themePreference}
      status={themeStatus}
      error={themeError}
      selectPreference={selectThemePreference}
      retryPreference={retryThemePreference}
    />

    <section
      aria-labelledby="local-boundary-heading"
      class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-5 sm:p-6"
    >
      <p class="text-accent text-xs tracking-[0.18em] uppercase">
        local-state manifest
      </p>
      <h2
        id="local-boundary-heading"
        class="text-foreground mt-2 text-xl font-light tracking-[0.03em]"
      >
        Local by design
      </h2>
      <p class="text-muted mt-3 text-xs leading-5">
        Your library is authoritative on this device. A backup is the only
        portable copy Animi creates.
      </p>
      <dl
        role="group"
        aria-label="Local-only manifest"
        class="border-border mt-5 divide-y border-y"
      >
        <div class="flex items-baseline justify-between gap-4 py-3">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            storage
          </dt>
          <dd class="text-foreground text-right text-xs">
            this Windows device
          </dd>
        </div>
        <div class="flex items-baseline justify-between gap-4 py-3">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            account
          </dt>
          <dd class="text-foreground text-right text-xs">none</dd>
        </div>
        <div class="flex items-baseline justify-between gap-4 py-3">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            cloud sync
          </dt>
          <dd class="text-foreground text-right text-xs">none</dd>
        </div>
        <div class="flex items-baseline justify-between gap-4 py-3">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            analytics
          </dt>
          <dd class="text-foreground text-right text-xs">none</dd>
        </div>
      </dl>
    </section>
  </div>

  <section
    aria-labelledby="cache-policy-heading"
    aria-busy={overviewStatus === "loading"}
    class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-5 sm:p-6"
  >
    <div
      class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"
    >
      <div class="max-w-2xl">
        <p class="text-accent text-xs tracking-[0.18em] uppercase">
          cache observatory // local metadata
        </p>
        <h2
          id="cache-policy-heading"
          class="text-foreground mt-2 text-xl font-light tracking-[0.03em]"
        >
          Cache freshness
        </h2>
        <p class="text-muted mt-2 text-xs leading-5 sm:text-sm sm:leading-6">
          Cached AniList data keeps tracked titles useful offline. Stale records
          remain visible and are refreshed on an allowed request.
        </p>
      </div>
      <button
        type="button"
        onclick={refreshOverview}
        disabled={overviewStatus === "loading"}
        class="glass-control glass-interactive control-rounded border-border min-h-10 border px-3 py-2 text-xs transition disabled:cursor-wait disabled:opacity-60"
      >
        {overviewStatus === "loading"
          ? "Inspecting cache…"
          : "Refresh cache view"}
      </button>
    </div>

    {#if overviewStatus === "error" && overviewError}
      <div
        role="alert"
        class="control-rounded border-danger/60 bg-danger/10 text-danger mt-5 border p-3 text-xs leading-5"
      >
        {overviewError.message}
      </div>
    {:else if overview}
      <dl
        role="group"
        aria-label="Local cache freshness"
        class="border-border mt-5 grid border-y sm:grid-cols-3"
      >
        <div class="border-border p-4 sm:border-r sm:px-5">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            media details // 24h
          </dt>
          <dd class="text-foreground mt-2 text-lg font-light tabular-nums">
            {overview.cachedMediaCount} cached
          </dd>
          <dd
            class:text-warning={overview.staleMediaCount > 0}
            class="text-muted mt-1 text-xs"
          >
            {overview.staleMediaCount} stale
          </dd>
        </div>
        <div
          class="border-border border-t p-4 sm:border-t-0 sm:border-r sm:px-5"
        >
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            airing // 30m
          </dt>
          <dd class="text-foreground mt-2 text-lg font-light tabular-nums">
            {overview.airingRecordCount} cached
          </dd>
          <dd
            class:text-warning={overview.staleAiringCount > 0}
            class="text-muted mt-1 text-xs"
          >
            {overview.staleAiringCount} stale
          </dd>
        </div>
        <div class="border-border border-t p-4 sm:border-t-0 sm:px-5">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            discover // 6h
          </dt>
          <dd class="text-foreground mt-2 text-lg font-light tabular-nums">
            {overview.responseCacheCount} cached
          </dd>
          <dd
            class:text-warning={overview.staleResponseCacheCount > 0}
            class="text-muted mt-1 text-xs"
          >
            {overview.staleResponseCacheCount} stale
          </dd>
        </div>
      </dl>
      <p class="text-muted mt-4 text-xs leading-5">
        Latest cache update: <span class="text-foreground"
          >{formatTimestamp(overview.latestCacheUpdateAt)}</span
        >
      </p>
    {/if}
  </section>

  <section
    aria-labelledby="backup-recovery-heading"
    class="glass-panel panel-large border-border grid min-w-0 overflow-hidden border lg:grid-cols-[minmax(0,0.78fr)_minmax(20rem,1.22fr)]"
  >
    <div class="min-w-0 p-5 sm:p-6 lg:p-7">
      <p class="text-accent text-xs tracking-[0.18em] uppercase">
        recovery desk // schema v1
      </p>
      <h2
        id="backup-recovery-heading"
        class="text-foreground mt-2 text-xl font-light tracking-[0.03em]"
      >
        Backup &amp; recovery
      </h2>
      <p class="text-muted mt-3 text-xs leading-5 sm:text-sm sm:leading-6">
        Export settings, library entries, and watch activity. Restore first
        validates the entire file, then shows exactly how local state will
        merge.
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
          Backup exported: {exportResult.libraryEntryCount} library entries and {exportResult.watchActivityCount}
          activity events.
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
          Recovery complete. Added {importResult.changes.libraryAdded} and merged
          {importResult.changes.libraryMerged} library entries.
        </p>
      {/if}
    </div>

    <div
      class="border-border min-w-0 border-t p-5 sm:p-6 lg:border-t-0 lg:border-l lg:p-7"
    >
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-accent text-[0.65rem] tracking-[0.14em] uppercase">
            recovery ledger
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
            {importPreview.settingsCount} settings · {importPreview.libraryEntryCount}
            library · {importPreview.watchActivityCount} activity
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
                  <li>
                    <span class="font-medium">{issue.code}</span> — {issue.message}{issue.path
                      ? ` (${issue.path})`
                      : ""}
                  </li>
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
                <dt class="text-muted text-[0.62rem] uppercase">
                  library added
                </dt>
                <dd class="text-success mt-1 text-xl font-light tabular-nums">
                  {importPreview.changes.libraryAdded}
                </dd>
              </div>
              <div class="bg-surface/85 p-3">
                <dt class="text-muted text-[0.62rem] uppercase">
                  library merged
                </dt>
                <dd class="text-accent mt-1 text-xl font-light tabular-nums">
                  {importPreview.changes.libraryMerged}
                </dd>
              </div>
              <div class="bg-surface/85 p-3">
                <dt class="text-muted text-[0.62rem] uppercase">kept local</dt>
                <dd
                  class="text-foreground mt-1 text-xl font-light tabular-nums"
                >
                  {importPreview.changes.libraryKeptLocal}
                </dd>
              </div>
              <div class="bg-surface/85 p-3">
                <dt class="text-muted text-[0.62rem] uppercase">
                  activity added
                </dt>
                <dd class="text-success mt-1 text-xl font-light tabular-nums">
                  {importPreview.changes.activityAdded}
                </dd>
              </div>
              <div class="bg-surface/85 p-3">
                <dt class="text-muted text-[0.62rem] uppercase">
                  activity unchanged
                </dt>
                <dd
                  class="text-foreground mt-1 text-xl font-light tabular-nums"
                >
                  {importPreview.changes.activityUnchanged}
                </dd>
              </div>
              <div class="bg-surface/85 p-3">
                <dt class="text-muted text-[0.62rem] uppercase">
                  unrelated retained
                </dt>
                <dd
                  class="text-foreground mt-1 text-xl font-light tabular-nums"
                >
                  {importPreview.changes.unrelatedLocalLibraryEntriesRetained}
                </dd>
              </div>
            </dl>
            <p
              role="note"
              aria-label="Recovery safety guarantee"
              class="control-rounded border-border bg-glass-control text-muted mt-4 border p-3 text-xs leading-5"
            >
              Unrelated local library entries remain in place. Equal timestamps
              keep local data. Restore commits all changes together or keeps
              none.
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
          Choose a backup to populate this ledger. Cached AniList responses and
          images are never included.
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
        <p class="text-danger text-xs tracking-[0.18em] uppercase">
          destructive action // irreversible
        </p>
        <h2
          id="clear-local-data-heading"
          class="text-foreground mt-2 text-xl font-light tracking-[0.03em]"
        >
          Clear local data
        </h2>
        <p
          class="text-muted mt-3 max-w-3xl text-xs leading-5 sm:text-sm sm:leading-6"
        >
          Removes the library, preferences, watch activity, and cached metadata
          from this device. Export a backup first if you may want this state
          again.
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
        Local data cleared: {clearResult.libraryEntriesRemoved} library entries and
        {clearResult.watchActivityRemoved} activity events removed.
      </p>
    {/if}
  </section>
</div>
