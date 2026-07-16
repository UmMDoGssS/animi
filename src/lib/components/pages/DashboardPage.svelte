<script lang="ts">
  import { tick } from "svelte";

  import type { LibraryStatus } from "$lib/ipc/types";

  import {
    buildGenreSignals,
    buildTagSampler,
    dashboardPreviewLibrary,
    libraryEntriesForStatus,
    monthlyActivityDays,
    monthlyActivityPreview,
    summarizeDashboardLibrary,
    yearlyActivityPreview,
    type DashboardPreviewActivityBucket,
    type DashboardPreviewActivityDay,
    type DashboardPreviewLibraryEntry,
  } from "../dashboardPreview";
  import DiscoveryPreviewCover from "../DiscoveryPreviewCover.svelte";
  import PageHeader from "../PageHeader.svelte";

  type ActivityRange = "month" | "year";
  type HistoryPreview = "sample" | "empty";

  interface LibrarySection {
    status: LibraryStatus;
    title: string;
    description: string;
    entries: DashboardPreviewLibraryEntry[];
  }

  const manifest = summarizeDashboardLibrary(dashboardPreviewLibrary);
  const genreSignals = buildGenreSignals(dashboardPreviewLibrary);
  const tagSampler = buildTagSampler(dashboardPreviewLibrary);
  const heatmapWeekLabels = ["W1", "W2", "W3", "W4", "W5"] as const;
  const heatmapWeekdays = [
    { shortLabel: "M", label: "Monday" },
    { shortLabel: "T", label: "Tuesday" },
    { shortLabel: "W", label: "Wednesday" },
    { shortLabel: "T", label: "Thursday" },
    { shortLabel: "F", label: "Friday" },
    { shortLabel: "S", label: "Saturday" },
    { shortLabel: "S", label: "Sunday" },
  ] as const;
  const heatmapLegend = [
    { level: 0, label: "0m" },
    { level: 1, label: "1–24m" },
    { level: 2, label: "25–48m" },
    { level: 3, label: "49–72m" },
    { level: 4, label: "73m+" },
  ] as const;
  const librarySections: readonly LibrarySection[] = [
    {
      status: "WATCHING",
      title: "Watching",
      description: "In progress now",
      entries: libraryEntriesForStatus(dashboardPreviewLibrary, "WATCHING"),
    },
    {
      status: "COMPLETED",
      title: "Completed",
      description: "Finished entries",
      entries: libraryEntriesForStatus(dashboardPreviewLibrary, "COMPLETED"),
    },
    {
      status: "PLANNING",
      title: "Planning",
      description: "Saved for later",
      entries: libraryEntriesForStatus(dashboardPreviewLibrary, "PLANNING"),
    },
  ];

  let activityRange = $state<ActivityRange>("month");
  let historyPreview = $state<HistoryPreview>("sample");
  let selectedActivityDay = $state(29);
  let heatmapTable = $state<HTMLTableElement | undefined>(undefined);

  const selectedActivity = $derived(
    activityRange === "month" ? monthlyActivityPreview : yearlyActivityPreview,
  );
  const inspectedActivityDay = $derived(
    monthlyActivityDays.find((day) => day.dayOfMonth === selectedActivityDay) ??
      null,
  );
  const activityTotalMinutes = $derived(
    selectedActivity.buckets.reduce(
      (total, bucket) => total + bucket.minutes,
      0,
    ),
  );
  const activityTotalEpisodes = $derived(
    selectedActivity.buckets.reduce(
      (total, bucket) => total + bucket.episodes,
      0,
    ),
  );
  const peakBucket = $derived(
    selectedActivity.buckets.reduce((peak, bucket) =>
      bucket.minutes > peak.minutes ? bucket : peak,
    ),
  );

  function padCount(value: number): string {
    return value.toString().padStart(2, "0");
  }

  function formatDuration(minutes: number): string {
    if (minutes === 0) {
      return "0m";
    }

    const hours = Math.floor(minutes / 60);
    const remainingMinutes = minutes % 60;
    if (hours === 0) {
      return `${remainingMinutes}m`;
    }

    return remainingMinutes === 0
      ? `${hours}h`
      : `${hours}h ${remainingMinutes.toString().padStart(2, "0")}m`;
  }

  function bucketAccessibleLabel(
    bucket: DashboardPreviewActivityBucket,
  ): string {
    if (bucket.minutes === 0) {
      return `${bucket.label}: no recorded sample watch time.`;
    }

    return `${bucket.label}: ${formatDuration(bucket.minutes)} across ${bucket.episodes} sample ${bucket.episodes === 1 ? "episode" : "episodes"}.`;
  }

  function activityDayAt(
    weekIndex: number,
    weekdayIndex: number,
  ): DashboardPreviewActivityDay | null {
    return (
      monthlyActivityDays.find(
        (day) =>
          day.weekIndex === weekIndex && day.weekdayIndex === weekdayIndex,
      ) ?? null
    );
  }

  function activityDayAccessibleLabel(
    day: DashboardPreviewActivityDay,
  ): string {
    if (day.minutes === 0) {
      return `${day.label}: no recorded sample watch time.`;
    }

    return `${day.label}: ${formatDuration(day.minutes)} across ${day.episodes} sample ${day.episodes === 1 ? "episode" : "episodes"}.`;
  }

  function contributionLevel(minutes: number): number {
    if (minutes === 0) {
      return 0;
    }

    if (minutes <= 24) {
      return 1;
    }

    if (minutes <= 48) {
      return 2;
    }

    if (minutes <= 72) {
      return 3;
    }

    return 4;
  }

  async function selectActivityDay(
    dayOfMonth: number,
    moveFocus = false,
  ): Promise<void> {
    selectedActivityDay = Math.min(
      monthlyActivityDays.length,
      Math.max(1, dayOfMonth),
    );

    if (!moveFocus) {
      return;
    }

    await tick();
    heatmapTable
      ?.querySelector<HTMLButtonElement>(`[data-day="${selectedActivityDay}"]`)
      ?.focus();
  }

  function handleActivityDayKeydown(
    event: KeyboardEvent,
    day: DashboardPreviewActivityDay,
  ): void {
    let nextDay: number | null = null;

    if (event.key === "ArrowDown") {
      nextDay = day.dayOfMonth + 1;
    } else if (event.key === "ArrowUp") {
      nextDay = day.dayOfMonth - 1;
    } else if (event.key === "ArrowRight") {
      nextDay = day.dayOfMonth + 7;
    } else if (event.key === "ArrowLeft") {
      nextDay = day.dayOfMonth - 7;
    } else if (event.key === "Home") {
      nextDay = 1;
    } else if (event.key === "End") {
      nextDay = monthlyActivityDays.length;
    }

    if (nextDay === null) {
      return;
    }

    event.preventDefault();
    void selectActivityDay(nextDay, true);
  }

  function barHeight(minutes: number): number {
    if (minutes === 0) {
      return 0;
    }

    return Math.max(8, Math.round((minutes / peakBucket.minutes) * 100));
  }

  function formatMediaFormat(value: string | null): string {
    if (value === null) {
      return "format unknown";
    }

    return value.replaceAll("_", " ").toLocaleLowerCase();
  }

  function progressLabel(entry: DashboardPreviewLibraryEntry): string {
    const progress = entry.progress.toString().padStart(2, "0");
    const total =
      entry.media.media.episodes?.toString().padStart(2, "0") ?? "??";
    return `${progress} / ${total} eps`;
  }
</script>

<div
  id="dashboard-panel"
  role="tabpanel"
  aria-labelledby="dashboard-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <PageHeader
    eyebrow="Local ledger // what Animi remembers"
    title="Dashboard"
    description="Read your library at a glance, trace recorded watch time, and revisit every shelf without sending it anywhere."
  />

  <aside
    role="note"
    aria-label="Dashboard sample data disclosure"
    class="glass-control control-rounded border-border flex min-w-0 flex-col gap-3 border p-4 sm:flex-row sm:items-center sm:p-5"
  >
    <span
      class="control-rounded border-accent/45 bg-accent/10 text-accent w-fit shrink-0 border px-2.5 py-1 text-[0.65rem] tracking-[0.14em] uppercase"
    >
      sample ledger
    </span>
    <p class="text-muted text-xs leading-5 sm:text-sm sm:leading-6">
      Layout preview only — the library, preference signals, and watch-time
      values below are fictional. Nothing here reads or changes your local
      library yet.
    </p>
  </aside>

  <section
    aria-labelledby="library-manifest-heading"
    class="glass-panel panel-large border-border grid min-w-0 overflow-hidden border lg:grid-cols-[minmax(0,1.25fr)_minmax(17rem,0.75fr)]"
  >
    <div class="min-w-0 p-4 sm:p-6 lg:p-7">
      <div class="flex flex-wrap items-end justify-between gap-3">
        <div>
          <p class="text-accent text-xs tracking-[0.18em] uppercase">
            library manifest
          </p>
          <h2
            id="library-manifest-heading"
            class="text-foreground mt-2 text-xl font-light tracking-[0.03em] sm:text-2xl"
          >
            Six titles, filed by what comes next.
          </h2>
        </div>
        <p class="text-muted text-[0.65rem] tracking-[0.1em] uppercase">
          bundled sample // current state
        </p>
      </div>

      <dl
        role="group"
        aria-label="Fictional sample library totals"
        class="border-border mt-6 grid grid-cols-2 border-y sm:grid-cols-4"
      >
        <div class="border-border border-r p-3 sm:p-4">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            tracked
          </dt>
          <dd
            class="text-foreground mt-2 text-2xl font-light tracking-[0.08em] tabular-nums sm:text-3xl"
          >
            {padCount(manifest.tracked)}
          </dd>
        </div>
        <div class="border-border p-3 sm:border-r sm:p-4">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            watching
          </dt>
          <dd
            class="text-accent mt-2 text-2xl font-light tracking-[0.08em] tabular-nums sm:text-3xl"
          >
            {padCount(manifest.watching)}
          </dd>
        </div>
        <div class="border-border border-t border-r p-3 sm:border-t-0 sm:p-4">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            completed
          </dt>
          <dd
            class="text-success mt-2 text-2xl font-light tracking-[0.08em] tabular-nums sm:text-3xl"
          >
            {padCount(manifest.completed)}
          </dd>
        </div>
        <div class="border-border border-t p-3 sm:border-t-0 sm:p-4">
          <dt class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
            planning
          </dt>
          <dd
            class="text-foreground mt-2 text-2xl font-light tracking-[0.08em] tabular-nums sm:text-3xl"
          >
            {padCount(manifest.planning)}
          </dd>
        </div>
      </dl>

      <p class="text-muted mt-4 text-xs leading-5">
        <span class="text-foreground tabular-nums"
          >{manifest.currentProgress} episodes</span
        >
        are marked in the current sample entries. This is library progress, not lifetime
        watch history.
      </p>
    </div>

    <div
      class="border-border min-w-0 border-t p-4 sm:p-6 lg:border-t-0 lg:border-l lg:p-7"
    >
      <p class="text-accent text-xs tracking-[0.18em] uppercase">
        preference signals
      </p>
      <h2 class="text-foreground mt-2 text-xl font-light tracking-[0.03em]">
        Metadata in the active shelves.
      </h2>
      <p class="text-muted mt-2 text-xs leading-5">
        Future signals use Watching and Completed metadata only. These labels
        come from the fictional sample and are not personalized rankings.
      </p>

      <div class="mt-5">
        <h3 class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
          genre frequency
        </h3>
        <ul aria-label="Sample genre frequency" class="mt-3 grid gap-3">
          {#each genreSignals as signal (signal.label)}
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
                  style:width={`${(signal.titleCount / genreSignals[0].titleCount) * 100}%`}
                ></span>
              </div>
            </li>
          {/each}
        </ul>
      </div>

      <div class="mt-5">
        <h3 class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
          title tag sampler
        </h3>
        <ul aria-label="Sample media tags" class="mt-3 flex flex-wrap gap-2">
          {#each tagSampler as tag (tag)}
            <li
              class="control-rounded border-border bg-glass-control text-foreground border px-2.5 py-1.5 text-[0.68rem]"
            >
              {tag}
            </li>
          {/each}
        </ul>
        <p class="text-muted mt-3 text-[0.68rem] leading-5">
          Tag order is illustrative; AniList tag relevance is not a personal
          affinity percentage.
        </p>
      </div>
    </div>
  </section>

  <section
    aria-labelledby="watch-ledger-heading"
    class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-4 sm:p-6 lg:p-7"
  >
    <header
      class="border-border flex min-w-0 flex-col gap-5 border-b pb-5 lg:flex-row lg:items-end lg:justify-between"
    >
      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <p class="text-accent text-xs tracking-[0.18em] uppercase">
            watch ledger
          </p>
          <span
            class="control-rounded border-warning/45 bg-warning/10 text-warning border px-2 py-1 text-[0.62rem] tracking-[0.12em] uppercase"
          >
            fictional activity
          </span>
        </div>
        <h2
          id="watch-ledger-heading"
          class="text-foreground mt-2 text-xl font-light tracking-[0.03em] sm:text-2xl"
        >
          A watch log, not a guess.
        </h2>
        <p class="text-muted mt-2 max-w-2xl text-xs leading-5 sm:text-sm">
          Watch time can come only from recorded progress events with known
          episode duration — never from library edit dates.
        </p>
      </div>

      <div class="grid w-full gap-2 sm:w-fit sm:grid-cols-2 lg:flex">
        <div
          role="group"
          aria-label="Preview watch history state"
          class="glass-control control-group border-border grid grid-cols-2 gap-1 border p-1"
        >
          <button
            type="button"
            aria-pressed={historyPreview === "sample"}
            class="glass-interactive control-rounded min-h-10 px-3 py-2 text-xs transition-colors"
            class:glass-accent={historyPreview === "sample"}
            class:text-muted={historyPreview !== "sample"}
            onclick={() => (historyPreview = "sample")}
          >
            sample history
          </button>
          <button
            type="button"
            aria-pressed={historyPreview === "empty"}
            class="glass-interactive control-rounded min-h-10 px-3 py-2 text-xs transition-colors"
            class:glass-accent={historyPreview === "empty"}
            class:text-muted={historyPreview !== "empty"}
            onclick={() => (historyPreview = "empty")}
          >
            no history
          </button>
        </div>

        {#if historyPreview === "sample"}
          <div
            role="group"
            aria-label="Activity range"
            class="glass-control control-group border-border grid grid-cols-2 gap-1 border p-1"
          >
            <button
              type="button"
              aria-pressed={activityRange === "month"}
              class="glass-interactive control-rounded min-h-10 px-4 py-2 text-xs transition-colors"
              class:glass-accent={activityRange === "month"}
              class:text-muted={activityRange !== "month"}
              onclick={() => (activityRange = "month")}
            >
              month
            </button>
            <button
              type="button"
              aria-pressed={activityRange === "year"}
              class="glass-interactive control-rounded min-h-10 px-4 py-2 text-xs transition-colors"
              class:glass-accent={activityRange === "year"}
              class:text-muted={activityRange !== "year"}
              onclick={() => (activityRange = "year")}
            >
              year
            </button>
          </div>
        {/if}
      </div>
    </header>

    {#if historyPreview === "sample"}
      <div aria-live="polite" class="sr-only">
        Showing fictional sample activity for {selectedActivity.label}.
      </div>

      {#key activityRange}
        <div class="ledger-materialize pt-6">
          <div class="mb-4 flex flex-wrap items-end justify-between gap-3">
            <div>
              <p class="text-muted text-[0.65rem] tracking-[0.12em] uppercase">
                covered range
              </p>
              <p class="text-foreground mt-1 text-sm">
                {selectedActivity.coveredRange}
              </p>
            </div>
            <p class="text-muted text-xs">
              peak // <span class="text-foreground">{peakBucket.label}</span>
              // {formatDuration(peakBucket.minutes)}
            </p>
          </div>

          {#if activityRange === "month"}
            <div
              class="heatmap-frame control-rounded border-border bg-glass-control flex min-w-0 flex-col gap-5 border p-3 sm:flex-row sm:items-center sm:justify-center sm:gap-8 sm:p-5"
            >
              <div class="min-w-0 overflow-hidden">
                <table
                  bind:this={heatmapTable}
                  aria-label="Fictional daily sample activity for July 2026"
                  aria-describedby="monthly-heatmap-help monthly-heatmap-readout"
                  class="heatmap-table mx-auto"
                >
                  <caption class="sr-only">
                    Daily fictional watch-time contributions for July 2026,
                    arranged by week and weekday.
                  </caption>
                  <thead>
                    <tr>
                      <th scope="col">
                        <span class="sr-only">Weekday</span>
                      </th>
                      {#each heatmapWeekLabels as week (week)}
                        <th
                          scope="col"
                          class="text-muted pb-1 text-center text-[0.58rem] font-light tracking-[0.08em] sm:text-[0.64rem]"
                        >
                          {week}
                        </th>
                      {/each}
                    </tr>
                  </thead>
                  <tbody>
                    {#each heatmapWeekdays as weekday, weekdayIndex (weekday.label)}
                      <tr>
                        <th
                          scope="row"
                          title={weekday.label}
                          class="text-muted pr-1 text-center text-[0.58rem] font-light sm:text-[0.64rem]"
                        >
                          <span aria-hidden="true">{weekday.shortLabel}</span>
                          <span class="sr-only">{weekday.label}</span>
                        </th>
                        {#each heatmapWeekLabels as week, weekIndex (week)}
                          {@const day = activityDayAt(weekIndex, weekdayIndex)}
                          <td>
                            {#if day}
                              <button
                                type="button"
                                aria-label={activityDayAccessibleLabel(day)}
                                aria-pressed={selectedActivityDay ===
                                  day.dayOfMonth}
                                title={activityDayAccessibleLabel(day)}
                                tabindex={selectedActivityDay === day.dayOfMonth
                                  ? 0
                                  : -1}
                                data-day={day.dayOfMonth}
                                data-level={contributionLevel(day.minutes)}
                                class="heatmap-cell"
                                onclick={() =>
                                  selectActivityDay(day.dayOfMonth)}
                                onfocus={() =>
                                  selectActivityDay(day.dayOfMonth)}
                                onmouseenter={() =>
                                  selectActivityDay(day.dayOfMonth)}
                                onkeydown={(event) =>
                                  handleActivityDayKeydown(event, day)}
                              ></button>
                            {:else}
                              <span
                                aria-hidden="true"
                                class="heatmap-cell heatmap-placeholder"
                              ></span>
                            {/if}
                          </td>
                        {/each}
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>

              <aside
                aria-label="Monthly contribution graph legend"
                class="border-border min-w-0 border-t pt-4 sm:max-w-52 sm:border-t-0 sm:border-l sm:pt-0 sm:pl-6"
              >
                <p
                  class="text-accent text-[0.65rem] tracking-[0.12em] uppercase"
                >
                  daily contribution map
                </p>
                <p class="text-muted mt-2 text-xs leading-5">
                  Each square is one calendar day. Deeper mauve means more
                  fictional watch time was recorded.
                </p>
                <p
                  id="monthly-heatmap-help"
                  class="text-muted mt-2 text-[0.65rem] leading-5"
                >
                  Focus a day, then use the arrow keys to inspect the ledger.
                </p>
                <p
                  id="monthly-heatmap-readout"
                  aria-live="polite"
                  class="border-border text-foreground mt-4 border-l-2 pl-3 text-xs leading-5 tabular-nums"
                >
                  {#if inspectedActivityDay}
                    <span class="text-accent">
                      JUL {padCount(inspectedActivityDay.dayOfMonth)}
                    </span>
                    // {formatDuration(inspectedActivityDay.minutes)} //
                    {inspectedActivityDay.episodes}
                    {inspectedActivityDay.episodes === 1 ? "ep" : "eps"}
                  {/if}
                </p>
                <ul
                  aria-label="Monthly activity intensity legend"
                  class="mt-4 grid grid-cols-5 gap-2 sm:grid-cols-1"
                >
                  {#each heatmapLegend as item (item.level)}
                    <li class="flex min-w-0 items-center gap-2">
                      <span
                        aria-hidden="true"
                        data-level={item.level}
                        class="heatmap-cell heatmap-legend-cell shrink-0"
                      ></span>
                      <span
                        class="text-muted overflow-hidden text-[0.58rem] text-ellipsis whitespace-nowrap sm:text-[0.64rem]"
                      >
                        {item.label}
                      </span>
                    </li>
                  {/each}
                </ul>
              </aside>
            </div>
          {:else}
            <div
              class="ledger-chart control-rounded border-border bg-glass-control grid min-w-0 grid-cols-[2.25rem_minmax(0,1fr)] gap-2 border p-3 sm:grid-cols-[3rem_minmax(0,1fr)] sm:gap-3 sm:p-4"
            >
              <div
                aria-hidden="true"
                class="text-muted flex min-h-48 flex-col justify-between py-1 text-[0.58rem] tabular-nums sm:min-h-56 sm:text-[0.65rem]"
              >
                <span>{formatDuration(peakBucket.minutes)}</span>
                <span>0m</span>
              </div>

              <ol
                aria-label={`Fictional sample activity for ${selectedActivity.label}`}
                class="ledger-bars ledger-bars-year min-h-48 min-w-0 gap-1.5 sm:min-h-56 sm:gap-2"
              >
                {#each selectedActivity.buckets as bucket (bucket.label)}
                  <li
                    aria-label={bucketAccessibleLabel(bucket)}
                    title={bucketAccessibleLabel(bucket)}
                    class="ledger-bucket min-w-0"
                  >
                    <span aria-hidden="true" class="ledger-bar-track">
                      <span
                        class="ledger-bar"
                        class:ledger-bar-empty={bucket.minutes === 0}
                        style:height={`${barHeight(bucket.minutes)}%`}
                      ></span>
                    </span>
                    <span
                      aria-hidden="true"
                      class="text-muted overflow-hidden text-center text-[0.56rem] tracking-[0.05em] text-ellipsis whitespace-nowrap sm:text-[0.64rem]"
                    >
                      {bucket.shortLabel}
                    </span>
                  </li>
                {/each}
              </ol>
            </div>
          {/if}

          <dl
            role="group"
            aria-label="Fictional activity summary"
            class="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4"
          >
            <div class="border-border border-l-2 pl-3">
              <dt class="text-muted text-[0.62rem] tracking-[0.1em] uppercase">
                illustrative total
              </dt>
              <dd class="text-foreground mt-1 text-base tabular-nums">
                {formatDuration(activityTotalMinutes)}
              </dd>
            </div>
            <div class="border-border border-l-2 pl-3">
              <dt class="text-muted text-[0.62rem] tracking-[0.1em] uppercase">
                sample episodes
              </dt>
              <dd class="text-foreground mt-1 text-base tabular-nums">
                {activityTotalEpisodes}
              </dd>
            </div>
            <div class="border-border border-l-2 pl-3">
              <dt class="text-muted text-[0.62rem] tracking-[0.1em] uppercase">
                peak bucket
              </dt>
              <dd class="text-foreground mt-1 truncate text-sm">
                {peakBucket.shortLabel} // {formatDuration(peakBucket.minutes)}
              </dd>
            </div>
            <div class="border-warning border-l-2 pl-3">
              <dt class="text-muted text-[0.62rem] tracking-[0.1em] uppercase">
                excluded
              </dt>
              <dd class="text-warning mt-1 text-sm">
                {selectedActivity.excludedEpisodes}
                episodes // duration unknown
              </dd>
            </div>
          </dl>
        </div>
      {/key}
    {:else}
      <div
        class="grid min-h-72 items-center gap-5 py-8 sm:grid-cols-[11rem_minmax(0,1fr)] sm:py-10"
      >
        <div
          aria-hidden="true"
          class="glass-control control-rounded border-border text-accent mx-auto grid min-h-36 w-full max-w-44 place-items-center overflow-hidden border p-4"
        >
          <div class="text-center">
            <p class="text-xl tracking-[0.04em] whitespace-nowrap">(=^･ω･^=)</p>
            <p
              class="text-muted mt-3 text-[0.62rem] tracking-[0.12em] uppercase"
            >
              ledger quiet
            </p>
          </div>
        </div>

        <div class="max-w-2xl">
          <p class="text-accent text-xs tracking-[0.16em] uppercase">
            no recorded activity
          </p>
          <h3
            class="text-foreground mt-2 text-2xl font-light tracking-[0.03em]"
          >
            There is no watch history to chart yet.
          </h3>
          <p class="text-muted mt-3 text-sm leading-6">
            Live monthly and yearly charts will begin with recorded progress
            events. Animi will not turn existing library edit dates into watch
            history.
          </p>
          <p class="text-warning mt-3 text-xs leading-5">
            Progress events without a known episode duration will remain
            excluded and visibly counted.
          </p>
        </div>
      </div>
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
        <p class="text-accent text-xs tracking-[0.18em] uppercase">
          library index
        </p>
        <h2
          id="library-index-heading"
          class="text-foreground mt-2 text-xl font-light tracking-[0.03em] sm:text-2xl"
        >
          Every shelf stays in view.
        </h2>
      </div>
      <p class="text-muted max-w-md text-xs leading-5 sm:text-right">
        Static sample rows only. Opening and editing entries arrives with the
        real library workflow.
      </p>
    </header>

    <div class="mt-5 grid min-w-0 gap-4 lg:grid-cols-3">
      {#each librarySections as section (section.status)}
        <section
          aria-labelledby={`dashboard-${section.status.toLocaleLowerCase()}-heading`}
          class="glass-control control-rounded border-border min-w-0 overflow-hidden border p-3 sm:p-4"
        >
          <header class="flex items-end justify-between gap-3">
            <div>
              <h3
                id={`dashboard-${section.status.toLocaleLowerCase()}-heading`}
                class="text-foreground text-base font-light tracking-[0.04em]"
              >
                {section.title}
              </h3>
              <p
                class="text-muted mt-1 text-[0.65rem] tracking-[0.08em] uppercase"
              >
                {section.description}
              </p>
            </div>
            <span
              class="text-accent text-sm tracking-[0.1em] tabular-nums"
              aria-label={`${section.entries.length} entries`}
            >
              {padCount(section.entries.length)}
            </span>
          </header>

          <ul
            aria-label={`${section.title} sample anime`}
            class="border-border mt-4 grid divide-y border-y"
          >
            {#each section.entries as entry (entry.media.media.anilistId)}
              <li
                class="grid min-w-0 grid-cols-[3.5rem_minmax(0,1fr)] gap-3 py-3"
              >
                <div class="w-14 shrink-0">
                  <DiscoveryPreviewCover
                    title={entry.media.media.title}
                    motif={entry.media.motif}
                    code={entry.media.catalogCode}
                  />
                </div>

                <div class="flex min-w-0 flex-col py-0.5">
                  <h4
                    class="text-foreground line-clamp-2 text-sm leading-5 font-light"
                  >
                    {entry.media.media.title}
                  </h4>
                  <p
                    class="text-muted mt-1 text-[0.62rem] tracking-[0.08em] uppercase"
                  >
                    {formatMediaFormat(entry.media.media.mediaFormat)} // sample snapshot
                  </p>
                  <div class="mt-auto pt-3">
                    <p class="text-foreground text-xs tabular-nums">
                      {progressLabel(entry)}
                    </p>
                    {#if entry.media.media.episodes === null}
                      <p class="text-warning mt-1 text-[0.62rem]">
                        episode total unknown
                      </p>
                    {:else if entry.score !== null}
                      <p class="text-muted mt-1 text-[0.62rem]">
                        sample score // {entry.score} / 10
                      </p>
                    {:else}
                      <p class="text-muted mt-1 text-[0.62rem]">
                        no sample score
                      </p>
                    {/if}
                  </div>
                </div>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  </section>
</div>

<style>
  .ledger-chart {
    background-image: repeating-linear-gradient(
      to top,
      transparent 0,
      transparent calc(25% - 1px),
      var(--border) 25%
    );
  }

  .ledger-bars {
    display: grid;
    align-items: stretch;
  }

  .ledger-bars-year {
    grid-template-columns: repeat(12, minmax(0, 1fr));
  }

  .heatmap-table {
    border-spacing: 0.35rem;
    border-collapse: separate;
    table-layout: fixed;
  }

  .heatmap-cell {
    position: relative;
    display: block;
    width: 1.75rem;
    aspect-ratio: 1;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 0.25rem;
    background: var(--glass-control);
    box-shadow: inset 0 1px 0 var(--glass-highlight);
    color: inherit;
    padding: 0;
    appearance: none;
    transition:
      transform 160ms ease-out,
      border-color 160ms ease-out;
  }

  .heatmap-cell::before {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--accent);
    content: "";
    opacity: var(--heat-opacity, 0);
  }

  .heatmap-cell[data-level="0"] {
    --heat-opacity: 0;
  }

  .heatmap-cell[data-level="1"] {
    --heat-opacity: 0.22;
  }

  .heatmap-cell[data-level="2"] {
    --heat-opacity: 0.42;
  }

  .heatmap-cell[data-level="3"] {
    --heat-opacity: 0.65;
  }

  .heatmap-cell[data-level="4"] {
    --heat-opacity: 0.9;
  }

  .heatmap-cell:not(.heatmap-placeholder):hover {
    transform: translateY(-1px) scale(1.04);
    border-color: var(--accent);
  }

  button.heatmap-cell {
    cursor: pointer;
  }

  button.heatmap-cell[aria-pressed="true"] {
    border-color: var(--accent);
    box-shadow:
      0 0 0 1px var(--accent),
      inset 0 1px 0 var(--glass-highlight);
  }

  .heatmap-placeholder {
    visibility: hidden;
  }

  .heatmap-legend-cell {
    width: 0.8rem;
    border-radius: 0.14rem;
  }

  .ledger-bucket {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 0.45rem;
  }

  .ledger-bar-track {
    display: flex;
    min-height: 0;
    align-items: flex-end;
  }

  .ledger-bar {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 0.2rem 0.2rem 0 0;
    background: var(--accent-glass);
    box-shadow: inset 0 1px 0 var(--glass-highlight);
    transition:
      height 180ms ease-out,
      opacity 180ms ease-out;
  }

  .ledger-bar-empty {
    border-width: 0;
    opacity: 0;
  }

  .ledger-materialize {
    animation: ledger-materialize 180ms ease-out both;
  }

  @keyframes ledger-materialize {
    from {
      opacity: 0.45;
      transform: translateY(0.25rem);
    }

    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (min-width: 640px) {
    .heatmap-cell {
      width: 2.25rem;
    }

    .heatmap-legend-cell {
      width: 0.85rem;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ledger-materialize {
      animation: none;
    }
  }
</style>
