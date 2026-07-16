<script lang="ts">
  import type { CommandError, ThemePreference } from "$lib/ipc/types";

  import PageHeader from "../PageHeader.svelte";
  import ThemePreferenceControl from "../ThemePreferenceControl.svelte";

  interface Props {
    themePreference: ThemePreference;
    themeStatus: "loading" | "ready" | "saving" | "error";
    themeError: CommandError | null;
    selectThemePreference: (preference: ThemePreference) => Promise<void>;
    retryThemePreference: () => Promise<void>;
  }

  let {
    themePreference,
    themeStatus,
    themeError,
    selectThemePreference,
    retryThemePreference,
  }: Props = $props();
</script>

<div
  id="settings-panel"
  role="tabpanel"
  aria-labelledby="settings-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <PageHeader
    eyebrow="Local config // nothing leaves this device"
    title="Settings"
    description="Choose Animi's palette and preview the local cache, backup, recovery, and data-safety controls that will live here."
  />

  <aside
    role="note"
    aria-label="Settings preview disclosure"
    class="glass-control control-rounded border-border flex min-w-0 flex-col gap-3 border p-4 sm:flex-row sm:items-center sm:p-5"
  >
    <span
      class="control-rounded border-success/45 bg-success/10 text-success w-fit shrink-0 border px-2.5 py-1 text-[0.65rem] tracking-[0.14em] uppercase"
    >
      live now
    </span>
    <p class="text-muted text-xs leading-5 sm:text-sm sm:leading-6">
      Appearance changes are saved on this device. Cache inspection, backup,
      restore, and clear-data actions below are layout previews only; they
      cannot read, write, or delete anything yet.
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
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p class="text-accent text-xs tracking-[0.18em] uppercase">
            local-state manifest
          </p>
          <h2
            id="local-boundary-heading"
            class="text-foreground mt-2 text-xl font-light tracking-[0.03em]"
          >
            Local by design
          </h2>
        </div>
        <span
          aria-hidden="true"
          class="text-accent text-xs tracking-[0.08em] whitespace-nowrap"
        >
          (=^._.^=) // device only
        </span>
      </div>

      <p class="text-muted mt-3 text-xs leading-5">
        Your library and preferences stay in Animi's local database. There is no
        account or remote copy behind this screen.
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
    class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-5 sm:p-6"
  >
    <div
      class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between"
    >
      <div class="max-w-2xl">
        <p class="text-accent text-xs tracking-[0.18em] uppercase">
          refresh policy // static preview
        </p>
        <h2
          id="cache-policy-heading"
          class="text-foreground mt-2 text-xl font-light tracking-[0.03em]"
        >
          Cache refresh policy
        </h2>
        <p class="text-muted mt-2 text-xs leading-5 sm:text-sm sm:leading-6">
          These are Animi's fixed refresh windows for cached AniList metadata.
          This preview does not inspect cache records or report current
          freshness.
        </p>
      </div>
      <span
        class="control-rounded border-border bg-glass-control text-muted w-fit shrink-0 border px-2.5 py-1 text-[0.65rem] tracking-[0.12em] uppercase"
      >
        no live values
      </span>
    </div>

    <dl
      role="group"
      aria-label="AniList cache refresh policy"
      class="border-border mt-5 grid border-y sm:grid-cols-3"
    >
      <div class="border-border p-4 sm:border-r sm:px-5">
        <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
          airing data
        </dt>
        <dd class="text-foreground mt-2 text-lg font-light tabular-nums">
          30 minutes
        </dd>
      </div>
      <div class="border-border border-t p-4 sm:border-t-0 sm:border-r sm:px-5">
        <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
          discover + recommendations
        </dt>
        <dd class="text-foreground mt-2 text-lg font-light tabular-nums">
          6 hours
        </dd>
      </div>
      <div class="border-border border-t p-4 sm:border-t-0 sm:px-5">
        <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
          media details
        </dt>
        <dd class="text-foreground mt-2 text-lg font-light tabular-nums">
          24 hours
        </dd>
      </div>
    </dl>

    <p class="text-muted mt-4 text-xs leading-5">
      If a refresh fails, previously cached data may remain visible as a stale
      fallback with its age identified.
    </p>
  </section>

  <section
    aria-labelledby="backup-recovery-heading"
    class="glass-panel panel-large border-border grid min-w-0 overflow-hidden border lg:grid-cols-[minmax(0,0.9fr)_minmax(20rem,1.1fr)]"
  >
    <div class="min-w-0 p-5 sm:p-6 lg:p-7">
      <p class="text-accent text-xs tracking-[0.18em] uppercase">
        recovery desk // preview only
      </p>
      <h2
        id="backup-recovery-heading"
        class="text-foreground mt-2 text-xl font-light tracking-[0.03em]"
      >
        Backup &amp; recovery
      </h2>
      <p class="text-muted mt-3 text-xs leading-5 sm:text-sm sm:leading-6">
        File actions arrive with transactional restore support. The controls in
        this checkpoint cannot choose, read, or write a backup file.
      </p>

      <div class="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-1 xl:grid-cols-2">
        <button
          type="button"
          disabled
          class="glass-accent control-rounded min-h-11 px-4 py-2.5 text-xs tracking-[0.04em] disabled:cursor-not-allowed disabled:opacity-55"
        >
          Export backup // preview only
        </button>
        <button
          type="button"
          disabled
          class="glass-control control-rounded border-border min-h-11 border px-4 py-2.5 text-xs tracking-[0.04em] disabled:cursor-not-allowed disabled:opacity-55"
        >
          Restore backup // preview only
        </button>
      </div>
      <p class="text-muted mt-3 text-[0.68rem] leading-5">
        Both actions are disabled until the recovery workflow is implemented.
      </p>
    </div>

    <div
      class="border-border min-w-0 border-t p-5 sm:p-6 lg:border-t-0 lg:border-l lg:p-7"
    >
      <h3
        id="backup-manifest-heading"
        class="text-foreground text-sm font-medium tracking-[0.03em]"
      >
        Backup manifest
      </h3>
      <div
        aria-labelledby="backup-manifest-heading"
        class="mt-4 grid gap-5 sm:grid-cols-2"
      >
        <div>
          <p class="text-success text-[0.65rem] tracking-[0.12em] uppercase">
            included
          </p>
          <ul
            aria-label="Backup contents preview"
            class="text-muted mt-3 grid gap-2 text-xs leading-5"
          >
            <li class="flex gap-2">
              <span aria-hidden="true" class="text-success">+</span>
              <span>Theme and local settings</span>
            </li>
            <li class="flex gap-2">
              <span aria-hidden="true" class="text-success">+</span>
              <span>Library entries</span>
            </li>
            <li class="flex gap-2">
              <span aria-hidden="true" class="text-success">+</span>
              <span>Recorded watch activity</span>
            </li>
          </ul>
        </div>

        <div>
          <p class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            excluded
          </p>
          <ul
            aria-label="Backup exclusions preview"
            class="text-muted mt-3 grid gap-2 text-xs leading-5"
          >
            <li class="flex gap-2">
              <span aria-hidden="true">-</span>
              <span>Cached AniList responses</span>
            </li>
            <li class="flex gap-2">
              <span aria-hidden="true">-</span>
              <span>Downloaded images</span>
            </li>
          </ul>
        </div>
      </div>

      <p
        role="note"
        class="control-rounded border-border bg-glass-control text-muted mt-5 border p-3 text-xs leading-5"
      >
        Restore will validate the whole file and preview conflicts before any
        changes. Unrelated local library entries will stay in place.
      </p>
    </div>
  </section>

  <section
    aria-labelledby="clear-local-data-heading"
    class="glass-panel panel-large border-danger/60 grid min-w-0 gap-5 border p-5 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end sm:p-6"
  >
    <div class="min-w-0">
      <p class="text-danger text-xs tracking-[0.18em] uppercase">
        destructive action // preview only
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
        When implemented, this removes Animi's local library, preferences,
        recorded watch activity, and cached metadata from this device. It will
        require a specific confirmation and will never run silently.
      </p>
    </div>

    <div class="sm:text-right">
      <button
        type="button"
        disabled
        class="control-rounded border-danger/70 bg-danger/10 text-danger min-h-11 w-full border px-4 py-2.5 text-xs tracking-[0.04em] disabled:cursor-not-allowed disabled:opacity-60 sm:w-auto"
      >
        Clear local data // preview only
      </button>
      <p class="text-muted mt-2 text-[0.68rem] leading-5">
        unavailable in this checkpoint
      </p>
    </div>
  </section>
</div>
