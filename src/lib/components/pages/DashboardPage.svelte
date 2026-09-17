<script lang="ts">
    import { onMount, tick } from "svelte";

    import {
        getDashboardInsights,
        getMediaDetails,
        normalizeCommandError,
        refreshLibraryMetadata,
        removeLibraryEntry,
        upsertLibraryEntry,
    } from "$lib/ipc/client";
    import type {
        CommandError,
        DashboardActivityBucket,
        DashboardInsights,
        DashboardInsightsRequest,
        RefreshLibraryMetadataRequest,
        MediaKind,
        Freshness,
        LibraryEntry,
        LibraryItem,
        LibraryStatus,
        MediaDetailsRequest,
        MediaDetailsResponse,
        MediaSummary,
        RefreshLibraryMetadataResponse,
        RemoveLibraryEntryRequest,
        RemoveLibraryEntryResponse,
        UpsertLibraryEntryRequest,
        WatchActivity,
    } from "$lib/ipc/types";

    import AsyncStateNotice from "../AsyncStateNotice.svelte";
    import GlassMultiSelect from "../GlassMultiSelect.svelte";
    import GlassSelect from "../GlassSelect.svelte";
    import MediaCover from "../MediaCover.svelte";
    import MediaDetailsDialog from "../MediaDetailsDialog.svelte";
    import RefreshIcon from "../RefreshIcon.svelte";

    interface Props {
        mediaKind?: MediaKind;
        libraryOnly?: boolean;
        hideLibrary?: boolean;
        onLibraryChanged?: () => void;
        loadInsights?: (
            request: DashboardInsightsRequest,
        ) => Promise<DashboardInsights>;
        refreshMetadata?: (
            request: RefreshLibraryMetadataRequest,
        ) => Promise<RefreshLibraryMetadataResponse>;
        loadDetails?: (
            request: MediaDetailsRequest,
        ) => Promise<MediaDetailsResponse>;
        saveEntry?: (
            request: UpsertLibraryEntryRequest,
        ) => Promise<LibraryEntry>;
        deleteEntry?: (
            request: RemoveLibraryEntryRequest,
        ) => Promise<RemoveLibraryEntryResponse>;
        openDiscovery?: () => void;
    }

    type LibraryStatusFilter = "ALL" | LibraryStatus;
    type LibrarySort =
        | "RELEASE_DESC"
        | "UPDATED_DESC"
        | "TITLE_ASC"
        | "PROGRESS_DESC"
        | "SCORE_DESC";

    const librarySortOptions = [
        { value: "RELEASE_DESC", label: "Newest release" },
        { value: "UPDATED_DESC", label: "Recently updated" },
        { value: "TITLE_ASC", label: "Title A–Z" },
        { value: "PROGRESS_DESC", label: "Most progress" },
        { value: "SCORE_DESC", label: "Highest score" },
    ] as const;

    let {
        mediaKind = "ANIME",
        libraryOnly = false,
        hideLibrary = false,
        onLibraryChanged,
        loadInsights = getDashboardInsights,
        refreshMetadata = refreshLibraryMetadata,
        loadDetails = getMediaDetails,
        saveEntry = upsertLibraryEntry,
        deleteEntry = removeLibraryEntry,
        openDiscovery,
    }: Props = $props();

    const libraryStatusOptions = $derived([
        { value: "ALL", label: "All statuses" },
        {
            value: "WATCHING",
            label: mediaKind === "MANGA" ? "Reading" : "Watching",
        },
        { value: "COMPLETED", label: "Completed" },
        { value: "PLANNING", label: "Planning" },
    ] as const);
    let kindLabel = $derived(mediaKind === "MANGA" ? "manga" : "anime");
    let insights = $state<DashboardInsights | null>(null);
    let loading = $state(true);
    let loadError = $state<CommandError | null>(null);
    let refreshing = $state(false);
    let refreshResult = $state<RefreshLibraryMetadataResponse | null>(null);
    let refreshError = $state<CommandError | null>(null);
    let selectedItem = $state<LibraryItem | null>(null);
    let detailsOpener: HTMLButtonElement | null = null;
    let libraryIndexHeading = $state<HTMLElement | null>(null);
    let libraryQuery = $state("");
    let libraryStatusFilter = $state<LibraryStatusFilter>("ALL");
    let libraryGenreFilters = $state<string[]>([]);
    let libraryTagFilters = $state<string[]>([]);
    let librarySort = $state<LibrarySort>("RELEASE_DESC");
    let libraryRail = $state<HTMLElement | null>(null);
    let libraryTargetIndex: number | null = null;
    let releaseDateBackfillChecked = false;

    $effect(() => {
        const rail = libraryRail;
        // Re-measure after filtering, metadata updates, or responsive resizing.
        const itemCount = filteredLibraryItems.length;
        libraryTargetIndex = null;
        if (rail === null || itemCount === 0) return;
        alignLibraryScroll();
        if (typeof ResizeObserver === "undefined") return;
        const observer = new ResizeObserver(() => {
            libraryTargetIndex = null;
            alignLibraryScroll();
        });
        observer.observe(rail);
        return () => observer.disconnect();
    });

    const selectedSummary = $derived.by<MediaSummary | null>(() => {
        if (selectedItem === null) {
            return null;
        }
        return summaryForItem(selectedItem);
    });

    const filteredLibraryItems = $derived.by<LibraryItem[]>(() => {
        const normalizedQuery = libraryQuery.trim().toLocaleLowerCase();
        const matchingItems = (insights?.libraryItems ?? []).filter((item) => {
            const matchesStatus =
                libraryStatusFilter === "ALL" ||
                item.entry.status === libraryStatusFilter;
            const matchesQuery =
                normalizedQuery.length === 0 ||
                titleForItem(item)
                    .toLocaleLowerCase()
                    .includes(normalizedQuery) ||
                item.entry.anilistId.toString().includes(normalizedQuery);
            const itemGenres = item.media?.genres ?? [];
            const itemTags = (item.media?.tags ?? []).map((tag) => tag.name);
            const matchesGenres = libraryGenreFilters.every((genre) =>
                itemGenres.includes(genre),
            );
            const matchesTags = libraryTagFilters.every((tag) =>
                itemTags.includes(tag),
            );
            return (
                matchesStatus && matchesQuery && matchesGenres && matchesTags
            );
        });
        return [...matchingItems].sort(compareLibraryItems);
    });

    const libraryFiltersActive = $derived(
        libraryQuery.trim().length > 0 ||
            libraryStatusFilter !== "ALL" ||
            libraryGenreFilters.length > 0 ||
            libraryTagFilters.length > 0,
    );

    const libraryGenreOptions = $derived.by(() =>
        uniqueLibraryLabels(
            (insights?.libraryItems ?? []).flatMap((item) =>
                item.media === null ? [] : item.media.genres,
            ),
        ),
    );

    const libraryTagOptions = $derived.by(() =>
        uniqueLibraryLabels(
            (insights?.libraryItems ?? []).flatMap((item) =>
                item.media === null
                    ? []
                    : item.media.tags.map((tag) => tag.name),
            ),
        ),
    );

    onMount(() => {
        void loadDashboard();
    });

    async function loadDashboard(): Promise<void> {
        if (insights === null) {
            loading = true;
        }
        loadError = null;

        try {
            insights = await loadInsights({ mediaKind });
            if (
                !releaseDateBackfillChecked &&
                insights.libraryItems.some(
                    (item) =>
                        item.media !== null &&
                        item.media.releaseDate === null &&
                        item.media.isStale,
                )
            ) {
                releaseDateBackfillChecked = true;
                void checkTrackedMetadata();
            }
        } catch (reason) {
            loadError = normalizeCommandError(reason);
        } finally {
            loading = false;
        }
    }

    async function checkTrackedMetadata(): Promise<void> {
        refreshing = true;
        refreshResult = null;
        refreshError = null;

        try {
            refreshResult = await refreshMetadata({ mediaKind });
            await loadDashboard();
        } catch (reason) {
            refreshError = normalizeCommandError(reason);
        } finally {
            refreshing = false;
        }
    }

    function titleForItem(item: LibraryItem): string {
        return item.media?.title ?? `AniList #${item.entry.anilistId}`;
    }

    function compareLibraryItems(
        left: LibraryItem,
        right: LibraryItem,
    ): number {
        if (librarySort === "RELEASE_DESC") {
            const leftRelease = left.media?.releaseDate ?? null;
            const rightRelease = right.media?.releaseDate ?? null;
            if (leftRelease !== rightRelease) {
                if (leftRelease === null) return 1;
                if (rightRelease === null) return -1;
                return leftRelease < rightRelease ? 1 : -1;
            }
            return compareLibraryTitles(left, right);
        }
        if (librarySort === "TITLE_ASC") {
            return compareLibraryTitles(left, right);
        }
        if (librarySort === "PROGRESS_DESC") {
            return (
                right.entry.progress - left.entry.progress ||
                compareLibraryTitles(left, right)
            );
        }
        if (librarySort === "SCORE_DESC") {
            return (
                (right.entry.score ?? -1) - (left.entry.score ?? -1) ||
                compareLibraryTitles(left, right)
            );
        }
        return (
            right.entry.updatedAt.localeCompare(left.entry.updatedAt) ||
            compareLibraryTitles(left, right)
        );
    }

    function compareLibraryTitles(
        left: LibraryItem,
        right: LibraryItem,
    ): number {
        return (
            titleForItem(left).localeCompare(titleForItem(right), undefined, {
                sensitivity: "base",
                numeric: true,
            }) || left.entry.anilistId - right.entry.anilistId
        );
    }

    function changeLibraryStatusFilter(value: string): void {
        const option = libraryStatusOptions.find(
            (candidate) => candidate.value === value,
        );
        if (!option) return;
        libraryStatusFilter = option.value;
        resetLibraryScroll();
    }

    function changeLibrarySort(value: string): void {
        const option = librarySortOptions.find(
            (candidate) => candidate.value === value,
        );
        if (!option) return;
        librarySort = option.value;
        resetLibraryScroll();
    }

    function changeLibraryGenreFilters(values: string[]): void {
        libraryGenreFilters = values;
        resetLibraryScroll();
    }

    function changeLibraryTagFilters(values: string[]): void {
        libraryTagFilters = values;
        resetLibraryScroll();
    }

    function uniqueLibraryLabels(
        values: string[],
    ): { value: string; label: string }[] {
        return [...new Set(values)]
            .sort((left, right) =>
                left.localeCompare(right, undefined, { sensitivity: "base" }),
            )
            .map((value) => ({ value, label: value }));
    }

    function resetLibraryFilters(): void {
        libraryQuery = "";
        libraryStatusFilter = "ALL";
        libraryGenreFilters = [];
        libraryTagFilters = [];
        resetLibraryScroll();
    }

    function removeLibraryGenreFilter(genre: string): void {
        changeLibraryGenreFilters(
            libraryGenreFilters.filter((candidate) => candidate !== genre),
        );
    }

    function removeLibraryTagFilter(tag: string): void {
        changeLibraryTagFilters(
            libraryTagFilters.filter((candidate) => candidate !== tag),
        );
    }

    function resetLibraryScroll(): void {
        libraryTargetIndex = null;
        libraryRail?.scrollTo?.({ left: 0, behavior: "instant" });
    }

    function libraryScrollPositions(): {
        positions: number[];
        visible: number;
    } {
        if (libraryRail === null) return { positions: [], visible: 1 };
        const cards = Array.from(libraryRail.children);
        const firstLeft = cards[0]?.getBoundingClientRect().left ?? 0;
        const visible = Math.max(
            1,
            Number.parseInt(
                getComputedStyle(libraryRail).getPropertyValue(
                    "--visible-cards",
                ),
                10,
            ) || 1,
        );
        // Only allow leading cards that leave a full group visible at the end.
        return {
            positions: cards
                .slice(0, Math.max(1, cards.length - visible + 1))
                .map((card) => card.getBoundingClientRect().left - firstLeft),
            visible,
        };
    }

    function nearestLibraryIndex(positions: number[]): number {
        const left = libraryRail?.scrollLeft ?? 0;
        return positions.reduce(
            (nearest, position, index) =>
                Math.abs(position - left) < Math.abs(positions[nearest]! - left)
                    ? index
                    : nearest,
            0,
        );
    }

    function alignLibraryScroll(): void {
        const { positions } = libraryScrollPositions();
        if (libraryRail === null || positions.length === 0) return;
        const index = libraryTargetIndex ?? nearestLibraryIndex(positions);
        libraryTargetIndex = null;
        const left = positions[Math.min(index, positions.length - 1)]!;
        if (Math.abs(libraryRail.scrollLeft - left) > 1) {
            libraryRail.scrollTo?.({ left, behavior: "instant" });
        }
    }

    function scrollLibrary(direction: -1 | 1): void {
        const { positions, visible } = libraryScrollPositions();
        if (libraryRail === null || positions.length === 0) return;
        const current = libraryTargetIndex ?? nearestLibraryIndex(positions);
        libraryTargetIndex = Math.max(
            0,
            Math.min(positions.length - 1, current + direction * visible),
        );
        libraryRail.scrollTo({
            left: positions[libraryTargetIndex]!,
            behavior: "auto",
        });
    }

    function summaryForItem(item: LibraryItem): MediaSummary {
        return {
            mediaKind: item.entry.mediaKind,
            anilistId: item.entry.anilistId,
            title: titleForItem(item),
            coverUrl: item.media?.coverUrl ?? null,
            mediaFormat: item.media?.mediaFormat ?? null,
            status: item.media?.status ?? null,
            episodes: item.media?.episodes ?? null,
            chapters: item.media?.chapters ?? null,
            volumes: item.media?.volumes ?? null,
            durationMinutes: item.media?.durationMinutes ?? null,
            releaseDate: item.media?.releaseDate ?? null,
            genres: item.media?.genres ?? [],
            tags: item.media?.tags ?? [],
            trending: 0,
        };
    }

    function freshnessForItem(item: LibraryItem): Freshness | null {
        if (item.media === null) {
            return null;
        }
        return {
            fetchedAt: item.media.cachedAt,
            expiresAt: item.media.expiresAt,
            isStale: item.media.isStale,
            refreshError: null,
        };
    }

    function openItemDetails(item: LibraryItem, event: MouseEvent): void {
        if (!(event.currentTarget instanceof HTMLButtonElement)) {
            return;
        }
        detailsOpener = event.currentTarget;
        selectedItem = item;
    }

    function closeItemDetails(): void {
        const anilistId = selectedItem?.entry.anilistId ?? null;
        selectedItem = null;
        const focusTarget = detailsOpener;
        detailsOpener = null;
        void tick().then(() => {
            if (focusTarget?.isConnected) {
                focusTarget.focus();
                return;
            }
            if (anilistId !== null) {
                const movedCard = document.querySelector<HTMLButtonElement>(
                    `[data-library-anilist-id="${anilistId}"]`,
                );
                if (movedCard !== null) {
                    movedCard.focus();
                    return;
                }
            }
            libraryIndexHeading?.focus();
        });
    }

    async function handleLibraryChanged(
        entry: LibraryEntry | null,
    ): Promise<void> {
        onLibraryChanged?.();
        if (selectedItem === null || insights === null) {
            await loadDashboard();
            return;
        }

        const previousEntry = selectedItem.entry;
        if (entry === null) {
            optimisticallyRemoveEntry(previousEntry);
        } else {
            selectedItem = { ...selectedItem, entry };
            optimisticallyUpdateEntry(previousEntry, entry);
        }
        await loadDashboard();
    }

    async function handleDetailsLoaded(
        response: MediaDetailsResponse,
    ): Promise<void> {
        onLibraryChanged?.();
        if (selectedItem === null || insights === null) {
            return;
        }
        const refreshedMedia = {
            title: response.media.title,
            coverUrl: response.media.coverUrl,
            mediaFormat: response.media.mediaFormat,
            genres: response.media.genres,
            tags: response.media.tags,
            status: response.media.status,
            episodes: response.media.episodes,
            chapters: response.media.chapters,
            volumes: response.media.volumes,
            durationMinutes: response.media.durationMinutes,
            releaseDate: response.media.releaseDate,
            cachedAt: response.freshness.fetchedAt,
            expiresAt: response.freshness.expiresAt,
            isStale: response.freshness.isStale,
        };
        selectedItem = { ...selectedItem, media: refreshedMedia };
        insights = {
            ...insights,
            libraryItems: insights.libraryItems.map((item) =>
                item.entry.anilistId === selectedItem?.entry.anilistId
                    ? { ...item, media: refreshedMedia }
                    : item,
            ),
        };
        await loadDashboard();
    }

    function optimisticallyUpdateEntry(
        previousEntry: LibraryEntry,
        entry: LibraryEntry,
    ): void {
        if (insights === null) {
            return;
        }
        const totals = { ...insights.libraryTotals };
        if (previousEntry.status !== entry.status) {
            totals[statusTotalKey(previousEntry.status)] -= 1;
            totals[statusTotalKey(entry.status)] += 1;
        }
        totals.currentProgress += entry.progress - previousEntry.progress;
        totals.currentVolumeProgress +=
            (entry.volumeProgress ?? 0) - (previousEntry.volumeProgress ?? 0);
        insights = {
            ...insights,
            libraryTotals: totals,
            libraryItems: insights.libraryItems.map((item) =>
                item.entry.anilistId === entry.anilistId
                    ? { ...item, entry }
                    : item,
            ),
        };
    }

    function optimisticallyRemoveEntry(entry: LibraryEntry): void {
        if (insights === null) {
            return;
        }
        const totals = { ...insights.libraryTotals };
        totals.tracked -= 1;
        totals[statusTotalKey(entry.status)] -= 1;
        totals.currentProgress -= entry.progress;
        totals.currentVolumeProgress -= entry.volumeProgress ?? 0;
        insights = {
            ...insights,
            libraryTotals: totals,
            libraryItems: insights.libraryItems.filter(
                (item) => item.entry.anilistId !== entry.anilistId,
            ),
        };
    }

    function statusTotalKey(
        status: LibraryStatus,
    ): "watching" | "completed" | "planning" {
        if (status === "WATCHING") {
            return "watching";
        }
        if (status === "COMPLETED") {
            return "completed";
        }
        return "planning";
    }

    function titleForActivity(activity: WatchActivity): string {
        return (
            insights?.libraryItems.find(
                (item) => item.entry.anilistId === activity.anilistId,
            )?.media?.title ?? `AniList #${activity.anilistId}`
        );
    }

    function formatMediaFormat(value: string | null): string {
        return value === null
            ? "format unknown"
            : value.replaceAll("_", " ").toLocaleLowerCase();
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

    function formatDuration(minutes: number): string {
        const hours = Math.floor(minutes / 60);
        const remainingMinutes = minutes % 60;
        if (hours === 0) {
            return `${remainingMinutes}m`;
        }
        return remainingMinutes === 0
            ? `${hours}h`
            : `${hours}h ${remainingMinutes.toString().padStart(2, "0")}m`;
    }

    function progressLabel(item: LibraryItem): string {
        return mediaKind === "MANGA"
            ? `${item.entry.progress} / ${item.media?.chapters ?? "??"} chapters · ${item.entry.volumeProgress ?? 0} / ${item.media?.volumes ?? "??"} volumes`
            : `${item.entry.progress} / ${item.media?.episodes ?? "??"} eps`;
    }

    function activityDuration(activity: WatchActivity): string {
        if (activity.episodeDurationMinutes === null) {
            return "duration unknown";
        }
        return formatDuration(
            activity.progressDelta * activity.episodeDurationMinutes,
        );
    }

    function padCount(value: number): string {
        return value.toString().padStart(2, "0");
    }

    function countLabel(
        value: number,
        singular: string,
        plural = `${singular}s`,
    ): string {
        return `${value} ${value === 1 ? singular : plural}`;
    }

    function activityBarHeight(
        bucket: DashboardActivityBucket,
        buckets: DashboardActivityBucket[],
    ): number {
        const maximum = Math.max(
            1,
            ...buckets.map((candidate) => candidate.recordedMinutes),
        );
        if (bucket.recordedMinutes === 0) {
            return bucket.recordedEpisodes > 0 ? 8 : 2;
        }
        return Math.max(12, (bucket.recordedMinutes / maximum) * 100);
    }

    function formatMonthPeriod(period: string): string {
        const date = new Date(`${period}-01T00:00:00.000Z`);
        if (Number.isNaN(date.getTime())) return period;
        return new Intl.DateTimeFormat(undefined, {
            month: "short",
            timeZone: "UTC",
        }).format(date);
    }

    function formatMonthName(period: string): string {
        const date = new Date(`${period}-01T00:00:00.000Z`);
        if (Number.isNaN(date.getTime())) return period;
        return new Intl.DateTimeFormat(undefined, {
            month: "long",
            year: "numeric",
            timeZone: "UTC",
        }).format(date);
    }

    function monthlyBucketLabel(bucket: DashboardActivityBucket): string {
        const known = formatDuration(bucket.recordedMinutes);
        const excluded = countLabel(
            bucket.unknownDurationEpisodes,
            "episode with unknown duration",
            "episodes with unknown duration",
        );
        return `${formatMonthName(bucket.period)}: ${known} of recorded watch time from ${countLabel(bucket.recordedEpisodes, "episode")}; ${excluded}.`;
    }

    function bucketLabel(bucket: DashboardActivityBucket): string {
        const known = formatDuration(bucket.recordedMinutes);
        const excluded = countLabel(
            bucket.unknownDurationEpisodes,
            "episode with unknown duration",
            "episodes with unknown duration",
        );
        return `${bucket.period}: ${known} recorded from ${countLabel(bucket.recordedEpisodes, "episode")}; ${excluded}.`;
    }
</script>

<div
    id={libraryOnly ? "home-library" : "dashboard-panel"}
    class:manga-dashboard={mediaKind === "MANGA"}
    role={libraryOnly ? "region" : "tabpanel"}
    aria-labelledby={libraryOnly ? "library-index-heading" : "dashboard-tab"}
    tabindex="-1"
    class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
    {#if !libraryOnly}<h1 class="sr-only">Dashboard</h1>{/if}

    {#if loading && insights === null}
        <AsyncStateNotice
            status="loading"
            title="Loading your dashboard"
            description="Gathering your library and activity."
        />
    {:else if loadError && insights === null}
        <AsyncStateNotice
            status="error"
            title="Your dashboard could not load"
            error={loadError}
            onRetry={loadError.retryable ? loadDashboard : undefined}
            retryLabel="Retry Dashboard"
        />
    {:else if insights}
        {#if !libraryOnly}
            <section
                aria-labelledby="library-manifest-heading"
                class="overview-panel"
            >
                <div class="overview-totals min-w-0">
                    <div class="flex flex-wrap items-end justify-between gap-3">
                        <div>
                            <h2
                                id="library-manifest-heading"
                                class="text-foreground text-xl font-light tracking-[0.03em] sm:text-2xl"
                            >
                                {insights.libraryTotals.tracked === 0
                                    ? "Your shelves are quiet."
                                    : `${insights.libraryTotals.tracked} ${insights.libraryTotals.tracked === 1 ? "title" : "titles"}, filed by what comes next.`}
                            </h2>
                        </div>
                        <button
                            type="button"
                            aria-label={refreshing
                                ? `refreshing ${kindLabel} details`
                                : `refresh ${kindLabel} details`}
                            title={refreshing
                                ? `Refreshing ${kindLabel} details`
                                : `Refresh ${kindLabel} details`}
                            class="glass-control glass-interactive control-rounded grid size-10 shrink-0 place-items-center transition-colors disabled:cursor-default disabled:opacity-60"
                            disabled={refreshing ||
                                insights.libraryTotals.tracked === 0}
                            onclick={() => void checkTrackedMetadata()}
                        >
                            <span class:animate-spin={refreshing}>
                                <RefreshIcon />
                            </span>
                        </button>
                    </div>

                    <dl
                        role="group"
                        aria-label="Library totals"
                        class="library-totals mt-4 grid grid-cols-2 gap-4 sm:grid-cols-4"
                    >
                        <div class="border-border border-r p-3 sm:p-4">
                            <dt
                                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                            >
                                tracked
                            </dt>
                            <dd
                                class="text-stat-tracked mt-2 text-3xl font-light tracking-[0.06em] tabular-nums sm:text-5xl"
                                data-stat="tracked"
                            >
                                {padCount(insights.libraryTotals.tracked)}
                            </dd>
                        </div>
                        <div class="border-border p-3 sm:p-4">
                            <dt
                                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                            >
                                {mediaKind === "MANGA" ? "reading" : "watching"}
                            </dt>
                            <dd
                                class="text-stat-watching mt-2 text-3xl font-light tracking-[0.06em] tabular-nums sm:text-5xl"
                                data-stat="watching"
                            >
                                {padCount(insights.libraryTotals.watching)}
                            </dd>
                        </div>
                        <div class="border-border border-t border-r p-3 sm:p-4">
                            <dt
                                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                            >
                                completed
                            </dt>
                            <dd
                                class="text-stat-completed mt-2 text-3xl font-light tracking-[0.06em] tabular-nums sm:text-5xl"
                                data-stat="completed"
                            >
                                {padCount(insights.libraryTotals.completed)}
                            </dd>
                        </div>
                        <div class="border-border border-t p-3 sm:p-4">
                            <dt
                                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                            >
                                planning
                            </dt>
                            <dd
                                class="text-stat-planning mt-2 text-3xl font-light tracking-[0.06em] tabular-nums sm:text-5xl"
                                data-stat="planning"
                            >
                                {padCount(insights.libraryTotals.planning)}
                            </dd>
                        </div>
                    </dl>

                    <p class="text-muted mt-4 text-xs leading-5">
                        <span class="text-foreground tabular-nums">
                            {insights.libraryTotals.currentProgress}
                            {mediaKind === "MANGA" ? "chapters" : "episodes"}
                        </span>
                        currently marked {mediaKind === "MANGA"
                            ? "read"
                            : "watched"}.
                        {#if mediaKind === "MANGA"}<strong
                                >{insights.libraryTotals.currentVolumeProgress} volumes</strong
                            > recorded independently.{/if}
                    </p>

                    {#if loadError}
                        <div class="mt-4">
                            <AsyncStateNotice
                                status="error"
                                title="The dashboard could not be refreshed"
                                description="Your previous dashboard is still shown."
                                error={loadError}
                                onRetry={loadError.retryable
                                    ? loadDashboard
                                    : undefined}
                                retryLabel="Retry reload"
                                compact
                            />
                        </div>
                    {/if}

                    {#if refreshError}
                        <div class="mt-4">
                            <AsyncStateNotice
                                status={refreshError.code
                                    .toLocaleUpperCase()
                                    .includes("OFFLINE")
                                    ? "offline"
                                    : "error"}
                                title={`${mediaKind === "MANGA" ? "Manga" : "Anime"} details could not be refreshed`}
                                description="Your library is still available."
                                error={refreshError}
                                onRetry={refreshError.retryable
                                    ? checkTrackedMetadata
                                    : undefined}
                                retryLabel="Check again"
                                compact
                            />
                        </div>
                    {:else if refreshResult}
                        <p
                            role="status"
                            class="control-rounded border-border bg-glass-control mt-4 border px-3 py-2 text-xs leading-5"
                            class:text-warning={refreshResult.failures.length >
                                0}
                            class:text-success={refreshResult.failures
                                .length === 0}
                        >
                            {#if refreshResult.failures.length > 0}
                                Updated {countLabel(
                                    refreshResult.updated,
                                    "title",
                                )}; {countLabel(
                                    refreshResult.failures.length,
                                    "title",
                                )} could not be refreshed.
                            {:else}
                                {mediaKind === "MANGA" ? "Manga" : "Anime"} details
                                refreshed.
                            {/if}
                        </p>
                    {/if}
                </div>

                <div
                    class="overview-preferences glass-panel panel-large min-w-0 p-6"
                >
                    <h2
                        class="text-foreground text-xl font-light tracking-[0.03em]"
                    >
                        Your favorite genres and tags
                    </h2>

                    {#if insights.preferenceSignals.genres.length > 0}
                        <div class="mt-5">
                            <h3
                                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                            >
                                favorite genres
                            </h3>
                            <ul
                                aria-label="Genre preference signals"
                                class="mt-3 grid gap-3"
                            >
                                {#each insights.preferenceSignals.genres.slice(0, 5) as signal (signal.label)}
                                    <li>
                                        <div
                                            class="flex items-center justify-between gap-3 text-xs"
                                        >
                                            <span class="text-foreground"
                                                >{signal.label}</span
                                            >
                                            <span
                                                class="text-muted tabular-nums"
                                            >
                                                {signal.titleCount}
                                                {signal.titleCount === 1
                                                    ? "title"
                                                    : "titles"}
                                            </span>
                                        </div>
                                        <div
                                            aria-hidden="true"
                                            class="bg-glass-control mt-1.5 h-1.5 overflow-hidden rounded-sm"
                                        >
                                            <span
                                                class="bg-accent block h-full rounded-sm"
                                                style:width={`${(signal.titleCount / (insights.preferenceSignals.genres[0]?.titleCount ?? 1)) * 100}%`}
                                            ></span>
                                        </div>
                                    </li>
                                {/each}
                            </ul>
                        </div>
                    {:else}
                        <p class="text-muted mt-5 text-xs leading-5">
                            Genres will appear as you progress and complete
                            titles.
                        </p>
                    {/if}

                    {#if insights.preferenceSignals.tags.length > 0}
                        <div class="mt-5">
                            <h3
                                class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                            >
                                favorite tags
                            </h3>
                            <ul
                                aria-label="Tag preference signals"
                                class="mt-3 flex flex-wrap gap-2"
                            >
                                {#each insights.preferenceSignals.tags.slice(0, 6) as tag (tag.label)}
                                    <li
                                        class="control-rounded border-border bg-glass-control text-foreground border px-2.5 py-1.5 text-[0.68rem]"
                                    >
                                        <span class="block">{tag.label}</span>
                                        <span
                                            class="text-muted mt-0.5 block text-[0.58rem]"
                                        >
                                            {countLabel(
                                                tag.titleCount,
                                                "title",
                                            )}
                                        </span>
                                    </li>
                                {/each}
                            </ul>
                        </div>
                    {:else}
                        <p class="text-muted mt-5 text-xs leading-5">
                            Tags will appear as you progress and complete
                            titles.
                        </p>
                    {/if}
                </div>
            </section>

            {#if mediaKind === "ANIME"}
                <section
                    aria-labelledby="activity-ledger-heading"
                    class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-4 sm:p-6 lg:p-7"
                >
                    <header
                        class="border-border flex flex-col gap-3 border-b pb-5 sm:flex-row sm:items-end sm:justify-between"
                    >
                        <div>
                            <h2
                                id="activity-ledger-heading"
                                class="text-foreground text-xl font-light tracking-[0.03em] sm:text-2xl"
                            >
                                Your watch history
                            </h2>
                        </div>
                    </header>

                    {#if insights.activity.eventCount === 0}
                        <div
                            class="grid min-h-56 items-center gap-5 py-6 sm:grid-cols-[10rem_minmax(0,1fr)]"
                        >
                            <div
                                aria-hidden="true"
                                class="glass-control control-rounded border-border text-accent mx-auto grid min-h-32 w-full max-w-40 place-items-center border p-4 text-center"
                            >
                                <div>
                                    <p class="text-lg">(=^..^=)</p>
                                    <p
                                        class="text-muted mt-3 text-[0.62rem] tracking-[0.12em] uppercase"
                                    >
                                        no history yet
                                    </p>
                                </div>
                            </div>
                            <div>
                                <h3 class="text-foreground text-lg font-light">
                                    Your watch history is empty.
                                </h3>
                                <p class="text-muted mt-2 text-sm leading-6">
                                    Watch an episode and update its progress to
                                    start your history.
                                </p>
                            </div>
                        </div>
                    {:else}
                        <dl
                            role="group"
                            aria-label="Recorded activity summary"
                            class="mt-5 grid grid-cols-2 gap-3 lg:grid-cols-4"
                        >
                            <div class="border-border border-l-2 pl-3">
                                <dt
                                    class="text-muted text-[0.62rem] tracking-[0.1em] uppercase"
                                >
                                    updates
                                </dt>
                                <dd
                                    class="text-foreground mt-1 text-lg tabular-nums"
                                >
                                    {insights.activity.eventCount}
                                </dd>
                            </div>
                            <div class="border-border border-l-2 pl-3">
                                <dt
                                    class="text-muted text-[0.62rem] tracking-[0.1em] uppercase"
                                >
                                    recorded episodes
                                </dt>
                                <dd
                                    class="text-foreground mt-1 text-lg tabular-nums"
                                >
                                    {insights.activity.recordedEpisodes}
                                </dd>
                            </div>
                            <div class="border-border border-l-2 pl-3">
                                <dt
                                    class="text-muted text-[0.62rem] tracking-[0.1em] uppercase"
                                >
                                    known watch time
                                </dt>
                                <dd
                                    class="text-foreground mt-1 text-lg tabular-nums"
                                >
                                    {formatDuration(
                                        insights.activity.recordedMinutes,
                                    )}
                                </dd>
                            </div>
                            <div class="border-warning border-l-2 pl-3">
                                <dt
                                    class="text-muted text-[0.62rem] tracking-[0.1em] uppercase"
                                >
                                    time unknown
                                </dt>
                                <dd
                                    class="text-warning mt-1 text-sm tabular-nums"
                                >
                                    {countLabel(
                                        insights.activity
                                            .unknownDurationEpisodes,
                                        "episode",
                                    )}
                                </dd>
                            </div>
                        </dl>

                        <div
                            class="mt-6 grid min-w-0 gap-4 lg:grid-cols-[minmax(0,1.6fr)_minmax(12rem,0.4fr)]"
                        >
                            <section
                                aria-labelledby="monthly-activity-heading"
                                class="glass-control control-rounded border-border min-w-0 border p-3 sm:p-4"
                            >
                                <div
                                    class="flex flex-wrap items-end justify-between gap-2"
                                >
                                    <div>
                                        <h3
                                            id="monthly-activity-heading"
                                            class="text-foreground text-base font-light"
                                        >
                                            Time spent watching anime each month
                                        </h3>
                                        <p
                                            id="monthly-activity-description"
                                            class="sr-only"
                                        >
                                            Each bar totals known watch time
                                            recorded during one calendar month.
                                        </p>
                                    </div>
                                </div>
                                <div class="mt-4 min-w-0 overflow-x-auto pb-1">
                                    <ol
                                        aria-label="Monthly recorded watch activity"
                                        aria-describedby="monthly-activity-description"
                                        class="grid min-w-[36rem] grid-cols-12 gap-2"
                                    >
                                        {#each insights.activity.monthly as bucket (bucket.period)}
                                            <li
                                                aria-label={monthlyBucketLabel(
                                                    bucket,
                                                )}
                                                class="grid min-w-0 gap-2"
                                            >
                                                <div
                                                    aria-hidden="true"
                                                    class="bg-glass-control flex h-28 items-end overflow-hidden rounded-sm"
                                                >
                                                    <span
                                                        class="bg-accent block w-full rounded-t-sm opacity-80"
                                                        class:bg-warning={bucket.recordedMinutes ===
                                                            0 &&
                                                            bucket.unknownDurationEpisodes >
                                                                0}
                                                        style:height={`${activityBarHeight(bucket, insights.activity.monthly)}%`}
                                                    ></span>
                                                </div>
                                                <div
                                                    aria-hidden="true"
                                                    class="text-center"
                                                >
                                                    <p
                                                        class="text-foreground text-[0.62rem] tabular-nums"
                                                    >
                                                        {bucket.recordedMinutes >
                                                        0
                                                            ? formatDuration(
                                                                  bucket.recordedMinutes,
                                                              )
                                                            : bucket.unknownDurationEpisodes >
                                                                0
                                                              ? "?"
                                                              : "—"}
                                                    </p>
                                                    <time
                                                        datetime={`${bucket.period}-01`}
                                                        aria-label={formatMonthName(
                                                            bucket.period,
                                                        )}
                                                        class="text-muted mt-1 block text-[0.55rem] uppercase"
                                                    >
                                                        {formatMonthPeriod(
                                                            bucket.period,
                                                        )}
                                                    </time>
                                                </div>
                                            </li>
                                        {/each}
                                    </ol>
                                </div>
                            </section>

                            <section
                                aria-labelledby="yearly-activity-heading"
                                class="glass-control control-rounded border-border border p-3 sm:p-4"
                            >
                                <h3
                                    id="yearly-activity-heading"
                                    class="text-foreground text-base font-light"
                                >
                                    Yearly totals
                                </h3>
                                <ul
                                    aria-label="Yearly recorded watch activity"
                                    class="border-border mt-4 grid divide-y border-y"
                                >
                                    {#each insights.activity.yearly as bucket (bucket.period)}
                                        <li
                                            aria-label={bucketLabel(bucket)}
                                            class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-3 py-3"
                                        >
                                            <span
                                                class="text-accent text-lg font-light tabular-nums"
                                                >{bucket.period}</span
                                            >
                                            <span
                                                class="text-muted text-right text-[0.62rem] leading-4"
                                            >
                                                <span
                                                    class="text-foreground block text-sm"
                                                    >{formatDuration(
                                                        bucket.recordedMinutes,
                                                    )}</span
                                                >
                                                {countLabel(
                                                    bucket.recordedEpisodes,
                                                    "episode",
                                                )}
                                                {#if bucket.unknownDurationEpisodes > 0}
                                                    <span
                                                        class="text-warning block"
                                                        >{bucket.unknownDurationEpisodes}
                                                        duration excluded</span
                                                    >
                                                {/if}
                                            </span>
                                        </li>
                                    {/each}
                                </ul>
                            </section>
                        </div>

                        <ol
                            aria-label="Recent recorded progress events"
                            class="mt-5 grid gap-2"
                        >
                            {#each insights.activity.events.slice(0, 8) as activity, index (`${activity.occurredAt}:${activity.anilistId}:${index}`)}
                                <li
                                    class="glass-control control-rounded border-border grid min-w-0 gap-2 border px-3 py-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center"
                                >
                                    <div class="min-w-0">
                                        <p
                                            class="text-foreground truncate text-sm"
                                        >
                                            {titleForActivity(activity)}
                                        </p>
                                        <time
                                            class="text-muted mt-1 block text-[0.65rem]"
                                            datetime={activity.occurredAt}
                                        >
                                            {formatTimestamp(
                                                activity.occurredAt,
                                            )}
                                        </time>
                                    </div>
                                    <p
                                        class="text-accent text-xs tabular-nums sm:text-right"
                                    >
                                        +{activity.progressDelta}
                                        {activity.progressDelta === 1
                                            ? "episode"
                                            : "episodes"}
                                        <span class="text-muted block"
                                            >{activityDuration(activity)}</span
                                        >
                                    </p>
                                </li>
                            {/each}
                        </ol>
                    {/if}
                </section>
            {/if}
        {/if}

        {#if !hideLibrary}
            <section
                aria-labelledby="library-index-heading"
                class="library-panel glass-panel panel-large border-border min-w-0 overflow-hidden border p-4 sm:p-6 lg:p-7"
            >
                <header
                    class="border-border flex flex-col gap-3 border-b pb-5 sm:flex-row sm:items-end sm:justify-between"
                >
                    <div>
                        <h2
                            bind:this={libraryIndexHeading}
                            id="library-index-heading"
                            tabindex="-1"
                            class="text-foreground text-xl font-light tracking-[0.03em] sm:text-2xl"
                        >
                            Your library
                        </h2>
                    </div>
                    {#if libraryOnly}
                        <button
                            type="button"
                            aria-label={`refresh ${kindLabel} details`}
                            class="glass-control glass-interactive control-rounded grid size-10 place-items-center"
                            disabled={refreshing}
                            onclick={() => void checkTrackedMetadata()}
                            ><RefreshIcon /></button
                        >
                    {/if}
                </header>

                {#if libraryOnly && (loadError || refreshError)}
                    {@const error = loadError ?? refreshError}
                    {#if error}<AsyncStateNotice
                            status={error.code.includes("OFFLINE")
                                ? "offline"
                                : "error"}
                            title="Keeping your saved library"
                            {error}
                            onRetry={error.retryable
                                ? checkTrackedMetadata
                                : undefined}
                            compact
                        />{/if}
                {/if}
                {#if libraryOnly && refreshResult}<p
                        role="status"
                        class="text-muted mt-3 text-sm"
                    >
                        Updated {refreshResult.updated} titles; {refreshResult.stale}
                        using cached details.
                    </p>{/if}
                {#if insights.libraryTotals.tracked === 0}
                    <div
                        class="grid min-h-64 place-items-center py-8 text-center"
                    >
                        <div class="max-w-xl">
                            <p aria-hidden="true" class="text-accent text-xl">
                                =^.^=
                            </p>
                            <h3 class="text-foreground mt-4 text-xl font-light">
                                Your library has no {kindLabel} yet.
                            </h3>
                            <p class="text-muted mt-2 text-sm leading-6">
                                Open a title in Discover and add it to your
                                library.
                            </p>
                            {#if openDiscovery}
                                <button
                                    type="button"
                                    class="glass-accent control-rounded mt-5 min-h-10 px-4 py-2 text-xs tracking-[0.08em]"
                                    onclick={openDiscovery}
                                >
                                    open Discover
                                </button>
                            {/if}
                        </div>
                    </div>
                {:else}
                    <div class="mt-5 grid min-w-0 gap-4">
                        <div
                            role="search"
                            aria-label="Filter library index"
                            class="glass-control control-rounded border-border relative z-20 grid min-w-0 gap-3 border p-3 sm:grid-cols-2 sm:p-4 lg:grid-cols-3 xl:grid-cols-[minmax(14rem,1.2fr)_repeat(4,minmax(8.5rem,0.6fr))_auto] xl:items-end"
                        >
                            <label class="grid min-w-0 gap-1.5">
                                <span
                                    class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                                >
                                    find a title
                                </span>
                                <input
                                    type="search"
                                    placeholder="Title or AniList ID"
                                    bind:value={libraryQuery}
                                    oninput={resetLibraryScroll}
                                    class="glass-control control-rounded border-border text-foreground placeholder:text-muted min-h-10 w-full min-w-0 border px-3 py-2 text-sm outline-none"
                                />
                            </label>

                            <div class="grid min-w-0 gap-1.5">
                                <span
                                    class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                                >
                                    status
                                </span>
                                <GlassSelect
                                    id="library-status-filter"
                                    label="status"
                                    value={libraryStatusFilter}
                                    options={libraryStatusOptions}
                                    onChange={changeLibraryStatusFilter}
                                />
                            </div>

                            <div class="grid min-w-0 gap-1.5">
                                <span
                                    class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                                >
                                    genres
                                </span>
                                <GlassMultiSelect
                                    id="library-genre-filter"
                                    label="genre"
                                    defaultLabel="All genres"
                                    selectedValues={libraryGenreFilters}
                                    options={libraryGenreOptions}
                                    onChange={changeLibraryGenreFilters}
                                />
                            </div>

                            <div class="grid min-w-0 gap-1.5">
                                <span
                                    class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                                >
                                    tags
                                </span>
                                <GlassMultiSelect
                                    id="library-tag-filter"
                                    label="tag"
                                    defaultLabel="All tags"
                                    selectedValues={libraryTagFilters}
                                    options={libraryTagOptions}
                                    onChange={changeLibraryTagFilters}
                                />
                            </div>

                            <div class="grid min-w-0 gap-1.5">
                                <span
                                    class="text-muted text-[0.65rem] tracking-[0.12em] uppercase"
                                >
                                    sort by
                                </span>
                                <GlassSelect
                                    id="library-sort"
                                    label="sort by"
                                    value={librarySort}
                                    options={librarySortOptions}
                                    onChange={changeLibrarySort}
                                />
                            </div>

                            <button
                                type="button"
                                class="glass-interactive control-rounded min-h-10 px-3 py-2 text-xs transition-colors disabled:cursor-not-allowed disabled:opacity-45"
                                disabled={!libraryFiltersActive}
                                onclick={resetLibraryFilters}
                            >
                                clear filters
                            </button>

                            {#if libraryGenreFilters.length > 0 || libraryTagFilters.length > 0}
                                <div
                                    role="group"
                                    aria-label="Active genre and tag filters"
                                    class="flex min-w-0 flex-wrap items-center gap-2 sm:col-span-2 lg:col-span-3 xl:col-span-6"
                                >
                                    <span
                                        class="text-muted mr-1 text-[0.62rem] tracking-[0.1em] uppercase"
                                    >
                                        must include
                                    </span>
                                    {#each libraryGenreFilters as genre (genre)}
                                        <button
                                            type="button"
                                            aria-label={`remove genre ${genre}`}
                                            class="glass-interactive control-rounded border-border text-foreground flex min-h-8 items-center gap-2 border px-2.5 py-1 text-xs transition-colors"
                                            onclick={() =>
                                                removeLibraryGenreFilter(genre)}
                                        >
                                            <span
                                                class="text-accent text-[0.58rem] tracking-[0.08em] uppercase"
                                                >genre</span
                                            >
                                            <span>{genre}</span>
                                            <span
                                                aria-hidden="true"
                                                class="text-muted">×</span
                                            >
                                        </button>
                                    {/each}
                                    {#each libraryTagFilters as tag (tag)}
                                        <button
                                            type="button"
                                            aria-label={`remove tag ${tag}`}
                                            class="glass-interactive control-rounded border-border text-foreground flex min-h-8 items-center gap-2 border px-2.5 py-1 text-xs transition-colors"
                                            onclick={() =>
                                                removeLibraryTagFilter(tag)}
                                        >
                                            <span
                                                class="text-accent text-[0.58rem] tracking-[0.08em] uppercase"
                                                >tag</span
                                            >
                                            <span>{tag}</span>
                                            <span
                                                aria-hidden="true"
                                                class="text-muted">×</span
                                            >
                                        </button>
                                    {/each}
                                </div>
                            {/if}
                        </div>

                        <div
                            class="flex flex-wrap items-center justify-between gap-2"
                        >
                            <p
                                class="text-muted text-xs leading-5"
                                aria-live="polite"
                            >
                                <span class="text-foreground tabular-nums"
                                    >{filteredLibraryItems.length}</span
                                >
                                matching {filteredLibraryItems.length === 1
                                    ? "title"
                                    : "titles"}
                                shown.
                            </p>
                            {#if filteredLibraryItems.length > 1}
                                <div
                                    role="group"
                                    aria-label="Library shelf controls"
                                    class="flex items-center gap-2"
                                >
                                    <button
                                        type="button"
                                        aria-label="Previous library titles"
                                        aria-controls="library-shelf"
                                        class="glass-control glass-interactive control-rounded grid size-10 place-items-center transition-colors"
                                        onclick={() => scrollLibrary(-1)}
                                    >
                                        <svg
                                            aria-hidden="true"
                                            class="size-4"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="1.8"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        >
                                            <path d="m15 18-6-6 6-6" />
                                        </svg>
                                    </button>
                                    <button
                                        type="button"
                                        aria-label="Next library titles"
                                        aria-controls="library-shelf"
                                        class="glass-control glass-interactive control-rounded grid size-10 place-items-center transition-colors"
                                        onclick={() => scrollLibrary(1)}
                                    >
                                        <svg
                                            aria-hidden="true"
                                            class="size-4"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="1.8"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        >
                                            <path d="m9 18 6-6-6-6" />
                                        </svg>
                                    </button>
                                </div>
                            {/if}
                        </div>

                        {#if filteredLibraryItems.length === 0}
                            <div
                                class="border-border grid min-h-48 place-items-center border-y py-8 text-center"
                            >
                                <div class="max-w-lg">
                                    <h3
                                        class="text-foreground text-lg font-light"
                                    >
                                        No titles match these filters.
                                    </h3>
                                    <p
                                        class="text-muted mt-2 text-xs leading-5"
                                    >
                                        Try another title, status, genre, or
                                        tag, or clear the filters to restore
                                        your full library.
                                    </p>
                                    <button
                                        type="button"
                                        class="glass-interactive control-rounded mt-4 min-h-10 px-4 py-2 text-xs"
                                        onclick={resetLibraryFilters}
                                    >
                                        clear filters
                                    </button>
                                </div>
                            </div>
                        {:else}
                            <ul
                                bind:this={libraryRail}
                                id="library-shelf"
                                onscrollend={alignLibraryScroll}
                                onwheel={() => (libraryTargetIndex = null)}
                                onpointerdown={() =>
                                    (libraryTargetIndex = null)}
                                aria-label="Library index results"
                                class="library-shelf flex min-w-0 gap-4 overflow-x-auto overscroll-x-contain pb-3"
                            >
                                {#each filteredLibraryItems as item (item.entry.anilistId)}
                                    {@const itemTitle = titleForItem(item)}
                                    <li class="min-w-0">
                                        <button
                                            type="button"
                                            aria-haspopup="dialog"
                                            aria-label={`Open library details for ${itemTitle}`}
                                            aria-describedby={`library-card-summary-${item.entry.anilistId}`}
                                            data-library-anilist-id={item.entry
                                                .anilistId}
                                            class="glass-control glass-interactive panel-large flex h-full w-full min-w-0 flex-col gap-3 p-3 text-left transition-colors"
                                            onclick={(event) =>
                                                openItemDetails(item, event)}
                                        >
                                            <span
                                                id={`library-card-summary-${item.entry.anilistId}`}
                                                class="sr-only"
                                            >
                                                {mediaKind === "MANGA" &&
                                                item.entry.status === "WATCHING"
                                                    ? "reading"
                                                    : item.entry.status.toLocaleLowerCase()}.
                                                {progressLabel(item)}.
                                                {item.entry.score === null
                                                    ? "No personal score."
                                                    : `Personal score ${item.entry.score} out of 10.`}
                                            </span>
                                            <div
                                                class="w-full min-w-0 self-stretch"
                                            >
                                                <MediaCover
                                                    title={itemTitle}
                                                    coverUrl={item.media
                                                        ?.coverUrl ?? null}
                                                    fill
                                                />
                                            </div>

                                            <span
                                                class="flex min-w-0 flex-col py-0.5"
                                            >
                                                <span
                                                    class="text-foreground line-clamp-2 text-sm leading-5 font-light"
                                                >
                                                    {itemTitle}
                                                </span>
                                                <span
                                                    class="text-accent mt-1 text-[0.62rem] tracking-[0.08em] uppercase"
                                                >
                                                    {mediaKind === "MANGA" &&
                                                    item.entry.status ===
                                                        "WATCHING"
                                                        ? "reading"
                                                        : item.entry.status.toLocaleLowerCase()}
                                                </span>
                                                <span
                                                    class="text-muted mt-1 text-[0.62rem] tracking-[0.08em] uppercase"
                                                >
                                                    {formatMediaFormat(
                                                        item.media
                                                            ?.mediaFormat ??
                                                            null,
                                                    )}
                                                </span>
                                                <span class="mt-3">
                                                    <span
                                                        class="text-foreground block text-xs tabular-nums"
                                                    >
                                                        {progressLabel(item)}
                                                    </span>
                                                    {#if item.entry.score !== null}
                                                        <span
                                                            class="text-muted mt-1 block text-[0.62rem]"
                                                        >
                                                            score // {item.entry
                                                                .score} / 10
                                                        </span>
                                                    {/if}
                                                </span>
                                            </span>
                                        </button>
                                    </li>
                                {/each}
                            </ul>
                        {/if}
                    </div>
                {/if}
            </section>
        {/if}
    {/if}

    {#if selectedItem && selectedSummary}
        <MediaDetailsDialog
            summary={selectedSummary}
            entry={selectedItem.entry}
            fallbackFreshness={freshnessForItem(selectedItem)}
            {loadDetails}
            {saveEntry}
            {deleteEntry}
            onDetailsLoaded={handleDetailsLoaded}
            onLibraryChanged={handleLibraryChanged}
            onClose={closeItemDetails}
        />
    {/if}
</div>

<style>
    .overview-panel {
        display: contents;
    }
    .overview-totals {
        grid-column: 1 / -1;
    }
    .library-totals > div {
        border: 1px solid var(--border);
        border-radius: 14px;
        padding: 16px 18px;
        background: var(--surface-raised);
    }
    .library-totals dd {
        font-size: 30px;
        letter-spacing: 0;
        font-weight: 500;
    }
    .library-totals dt {
        text-transform: capitalize;
        font-size: 13px;
        letter-spacing: 0;
    }
    .library-panel {
        container-type: inline-size;
    }
    .library-shelf {
        --visible-cards: 1;
        scroll-behavior: smooth;
        scroll-snap-type: x mandatory;
    }
    .library-shelf > li {
        flex: 0 0
            calc(
                (100% - (var(--visible-cards) - 1) * 1rem) /
                    var(--visible-cards)
            );
        scroll-snap-align: start;
    }
    @container (min-width: 280px) {
        .library-shelf {
            --visible-cards: 2;
        }
    }
    @container (min-width: 560px) {
        .library-shelf {
            --visible-cards: 3;
        }
    }
    @container (min-width: 760px) {
        .library-shelf {
            --visible-cards: 4;
        }
    }
    @container (min-width: 960px) {
        .library-shelf {
            --visible-cards: 5;
        }
    }
    @container (min-width: 1160px) {
        .library-shelf {
            --visible-cards: 6;
        }
    }
    .library-shelf :global([data-cover-state]) {
        height: 200px;
        aspect-ratio: auto;
    }
    @supports (backdrop-filter: blur(20px)) {
        .library-totals > div {
            background: var(--glass);
            backdrop-filter: blur(20px);
        }
    }
    @media (min-width: 1000px) {
        #dashboard-panel {
            grid-template-columns: minmax(0, 1.7fr) minmax(0, 1fr);
            align-items: start;
        }
        .manga-dashboard .overview-preferences {
            grid-column: 1 / -1;
        }
        .overview-preferences {
            grid-column: 2;
            grid-row: 2;
        }
        section[aria-labelledby="activity-ledger-heading"] {
            grid-column: 1;
            grid-row: 2;
        }
        section[aria-labelledby="library-index-heading"] {
            grid-column: 1 / -1;
        }
    }
    @media (max-width: 600px) {
        .library-shelf :global([data-cover-state]) {
            height: 150px;
        }
    }
</style>
