<script lang="ts">
  import HomePreviewCover, {
    type HomeCoverMotif,
  } from "../HomePreviewCover.svelte";
  import PageHeader from "../PageHeader.svelte";

  type PreviewMode = "watching" | "empty";
  type AiringState = "on-time" | "catch-up" | "unknown";

  interface PreviewAnime {
    id: number;
    title: string;
    format: string;
    progress: number;
    totalEpisodes: number | null;
    nextEpisode: number | null;
    countdown: string | null;
    freshness: string;
    motif: HomeCoverMotif;
    coverCode: string;
  }

  interface Props {
    openDiscovery?: () => void;
  }

  let { openDiscovery = () => undefined }: Props = $props();

  const previewAnime: PreviewAnime[] = [
    {
      id: 701,
      title: "Moonlit Post Office",
      format: "TV · 24 min",
      progress: 8,
      totalEpisodes: 12,
      nextEpisode: 9,
      countdown: "02d 11h",
      freshness: "sample snapshot // 12m old",
      motif: "moon",
      coverCode: "01 // 03",
    },
    {
      id: 702,
      title: "Last Train to Aomori",
      format: "TV · 23 min",
      progress: 5,
      totalEpisodes: 12,
      nextEpisode: 8,
      countdown: "05h 42m",
      freshness: "sample snapshot // 18m old",
      motif: "rail",
      coverCode: "02 // 03",
    },
    {
      id: 703,
      title: "Signal Lost at 2AM",
      format: "ONA · 18 min",
      progress: 3,
      totalEpisodes: 10,
      nextEpisode: null,
      countdown: null,
      freshness: "sample snapshot // 26m old",
      motif: "signal",
      coverCode: "03 // 03",
    },
  ];

  let mode = $state<PreviewMode>("watching");
  let shows = $state<PreviewAnime[]>(
    previewAnime.map((anime) => ({ ...anime })),
  );
  let selectedIndex = $state(0);

  const waitingCount = $derived(
    shows.reduce((total, show) => total + unwatchedCount(show), 0),
  );
  const unknownScheduleCount = $derived(
    shows.filter((show) => effectiveState(show) === "unknown").length,
  );

  function unwatchedCount(show: PreviewAnime): number {
    if (show.nextEpisode === null) {
      return 0;
    }

    return Math.max(0, show.nextEpisode - 1 - show.progress);
  }

  function effectiveState(show: PreviewAnime): AiringState {
    if (show.nextEpisode === null) {
      return "unknown";
    }

    return unwatchedCount(show) > 0 ? "catch-up" : "on-time";
  }

  function canMarkWatched(show: PreviewAnime): boolean {
    if (show.totalEpisodes !== null && show.progress >= show.totalEpisodes) {
      return false;
    }

    return effectiveState(show) === "catch-up";
  }

  function progressLabel(show: PreviewAnime): string {
    return `${show.progress.toString().padStart(2, "0")} / ${
      show.totalEpisodes?.toString().padStart(2, "0") ?? "??"
    }`;
  }

  function progressPercent(show: PreviewAnime): number {
    if (show.totalEpisodes === null) {
      return Math.min(show.progress * 8, 88);
    }

    return (show.progress / show.totalEpisodes) * 100;
  }

  function actionLabel(show: PreviewAnime): string {
    if (effectiveState(show) === "unknown") {
      return "schedule unavailable";
    }

    if (effectiveState(show) === "on-time") {
      return "all caught up";
    }

    return `preview · mark ep. ${(show.progress + 1).toString().padStart(2, "0")} watched`;
  }

  function markWatched(index: number) {
    const show = shows[index];
    if (!show || !canMarkWatched(show)) {
      return;
    }

    show.progress += 1;
  }

  function selectSlide(index: number) {
    selectedIndex = (index + shows.length) % shows.length;
  }

  function selectedShow(): PreviewAnime {
    return shows[selectedIndex] ?? previewAnime[0];
  }

  function handleCarouselKeydown(event: KeyboardEvent) {
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
      selectSlide(shows.length - 1);
    }
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
      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <p class="text-accent text-xs tracking-[0.18em] uppercase">
            airing queue
          </p>
          <span
            class="control-rounded border-border bg-glass-control text-muted border px-2 py-1 text-[0.62rem] tracking-[0.12em] uppercase"
          >
            sample library
          </span>
        </div>
        <h2
          id="airing-queue-heading"
          class="text-foreground mt-2 text-xl font-light tracking-[0.03em] sm:text-2xl"
        >
          Your week, one episode at a time.
        </h2>
        <p class="text-muted mt-2 max-w-2xl text-xs leading-5 sm:text-sm">
          Layout preview only — sample entries and progress are not saved.
        </p>
      </div>

      <div
        role="group"
        aria-label="Preview Home state"
        class="glass-control control-group border-border grid w-fit grid-cols-2 gap-1 border p-1"
      >
        <button
          type="button"
          aria-pressed={mode === "watching"}
          class="glass-interactive control-rounded px-3 py-2 text-xs transition-colors"
          class:glass-accent={mode === "watching"}
          class:text-muted={mode !== "watching"}
          onclick={() => (mode = "watching")}
        >
          watching
        </button>
        <button
          type="button"
          aria-pressed={mode === "empty"}
          class="glass-interactive control-rounded px-3 py-2 text-xs transition-colors"
          class:glass-accent={mode === "empty"}
          class:text-muted={mode !== "empty"}
          onclick={() => (mode = "empty")}
        >
          empty state
        </button>
      </div>
    </header>

    {#if mode === "watching"}
      <div
        class="flex flex-col gap-4 py-5 sm:flex-row sm:items-center sm:justify-between"
      >
        <div class="flex flex-wrap items-center gap-x-5 gap-y-2">
          <p class="text-muted text-xs tracking-[0.1em] uppercase">
            <span class="text-foreground text-base">{shows.length}</span>
            in queue
          </p>
          <p class="text-muted text-xs tracking-[0.1em] uppercase">
            <span class="text-warning text-base">{waitingCount}</span>
            known {waitingCount === 1 ? "episode" : "episodes"} waiting
          </p>
          <p class="text-muted text-xs tracking-[0.1em] uppercase">
            <span class="text-foreground text-base">{unknownScheduleCount}</span
            >
            schedule unknown
          </p>
        </div>

        <div class="flex items-center gap-2">
          <span
            aria-live="polite"
            aria-atomic="true"
            class="text-muted min-w-16 text-center text-xs tracking-[0.14em]"
          >
            <span aria-hidden="true">
              {(selectedIndex + 1).toString().padStart(2, "0")} //
              {shows.length.toString().padStart(2, "0")}
            </span>
            <span class="sr-only">
              Slide {selectedIndex + 1} of {shows.length}, {selectedShow()
                .title}
            </span>
          </span>
          <button
            type="button"
            aria-label="Previous anime"
            class="glass-control glass-interactive control-rounded text-foreground grid size-10 place-items-center transition-colors"
            onclick={() => selectSlide(selectedIndex - 1)}
            onkeydown={handleCarouselKeydown}
          >
            <span aria-hidden="true">←</span>
          </button>
          <button
            type="button"
            aria-label="Next anime"
            class="glass-control glass-interactive control-rounded text-foreground grid size-10 place-items-center transition-colors"
            onclick={() => selectSlide(selectedIndex + 1)}
            onkeydown={handleCarouselKeydown}
          >
            <span aria-hidden="true">→</span>
          </button>
        </div>
      </div>

      {@const show = selectedShow()}
      <div
        role="region"
        aria-roledescription="carousel"
        aria-label="Airing anime"
        aria-describedby="airing-carousel-help"
      >
        <p id="airing-carousel-help" class="sr-only">
          Use the previous and next buttons, or focus a carousel control and
          press Left, Right, Home, or End.
        </p>

        <article
          role="group"
          aria-roledescription="slide"
          aria-label={`${selectedIndex + 1} of ${shows.length}: ${show.title}`}
          class="glass-control control-rounded border-border grid min-w-0 gap-5 border p-4 sm:grid-cols-[11rem_minmax(0,1fr)] sm:items-stretch sm:p-5 lg:grid-cols-[13rem_minmax(0,1fr)] lg:gap-7 lg:p-6"
        >
          <div class="mx-auto w-36 sm:w-full">
            <HomePreviewCover
              title={show.title}
              motif={show.motif}
              code={show.coverCode}
            />
          </div>

          <div class="flex min-w-0 flex-col sm:py-1">
            <div class="flex flex-wrap items-center gap-2">
              {#if effectiveState(show) === "on-time"}
                <span
                  class="control-rounded border-success/40 bg-success/10 text-success border px-2 py-1 text-[0.65rem] tracking-[0.12em] uppercase"
                >
                  on time
                </span>
              {:else if effectiveState(show) === "catch-up"}
                <span
                  class="control-rounded border-warning/50 bg-warning/10 text-warning border px-2 py-1 text-[0.65rem] tracking-[0.12em] uppercase"
                >
                  catch up
                </span>
              {:else}
                <span
                  class="control-rounded border-border bg-glass-control text-muted border px-2 py-1 text-[0.65rem] tracking-[0.12em] uppercase"
                >
                  unknown
                </span>
              {/if}
              <span
                class="text-muted text-[0.68rem] tracking-[0.08em] uppercase"
              >
                {show.format}
              </span>
            </div>

            <h3
              class="text-foreground mt-4 text-xl leading-tight font-light tracking-[0.025em] sm:text-2xl lg:text-3xl"
            >
              {show.title}
            </h3>
            <p
              class="text-muted mt-2 text-[0.68rem] tracking-[0.08em] uppercase"
            >
              {show.freshness}
            </p>

            <div class="border-border mt-5 border-l-2 pl-4 sm:mt-6">
              {#if effectiveState(show) === "catch-up"}
                <p class="text-warning text-2xl font-light sm:text-3xl">
                  {unwatchedCount(show)}
                  {unwatchedCount(show) === 1 ? "episode" : "episodes"}
                  waiting
                </p>
                <p class="text-muted mt-2 text-xs leading-5 sm:text-sm">
                  Episode {show.nextEpisode} airs in {show.countdown}.
                  <span aria-hidden="true"> (=；ェ；=)</span>
                </p>
              {:else if effectiveState(show) === "unknown"}
                <p class="text-foreground text-2xl font-light sm:text-3xl">
                  schedule unknown
                </p>
                <p class="text-muted mt-2 text-xs leading-5 sm:text-sm">
                  No countdown or Home quick update until available episodes can
                  be confirmed.
                </p>
              {:else}
                <p class="text-foreground text-3xl font-light sm:text-4xl">
                  {show.countdown}
                </p>
                <p class="text-muted mt-2 text-xs leading-5 sm:text-sm">
                  until episode {show.nextEpisode}; you are right on time.
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
                  {progressLabel(show)}
                </dd>
              </div>
              <div>
                <dt
                  class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                >
                  next episode
                </dt>
                <dd class="text-foreground mt-1 text-base tracking-[0.08em]">
                  {show.nextEpisode?.toString().padStart(2, "0") ?? "—"}
                </dd>
              </div>
            </dl>

            <div
              aria-hidden="true"
              class="home-progress-track bg-glass-control mt-3 h-1.5 overflow-hidden rounded-sm"
            >
              <span
                class="bg-accent block h-full rounded-sm transition-[width]"
                style:width={`${progressPercent(show)}%`}
              ></span>
            </div>

            <div class="home-progress-action mt-5 shrink-0 sm:mt-auto sm:pt-5">
              <button
                type="button"
                disabled={!canMarkWatched(show)}
                class="glass-accent control-rounded w-full px-4 py-3 text-xs tracking-[0.06em] transition disabled:cursor-default disabled:opacity-55 sm:w-fit sm:min-w-60"
                onclick={() => markWatched(selectedIndex)}
              >
                {actionLabel(show)}
              </button>
            </div>
          </div>
        </article>
      </div>

      <div class="mt-3 flex items-center justify-center gap-1">
        {#each shows as carouselShow, index (carouselShow.id)}
          <button
            type="button"
            aria-label={`Show ${carouselShow.title}`}
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
    {:else}
      <div
        class="grid min-h-80 items-center gap-6 py-8 sm:grid-cols-[minmax(13rem,0.7fr)_minmax(0,1fr)] sm:py-12"
      >
        <div
          aria-hidden="true"
          class="glass-control control-rounded border-border text-accent mx-auto grid min-h-48 w-full max-w-xs place-items-center border p-6"
        >
          <div class="text-center">
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
          <h3
            class="text-foreground mt-2 text-2xl font-light tracking-[0.03em]"
          >
            Nothing is airing in your watching list.
          </h3>
          <p class="text-muted mt-3 text-sm leading-6">
            Your cat checked twice. Add something from Discover or move a
            planned title into Watching when you are ready.
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
    {/if}
  </section>
</div>
