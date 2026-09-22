<script lang="ts">
    import { onMount, tick } from "svelte";
    import {
        advanceLibraryProgress,
        getHomeReading,
        normalizeCommandError,
    } from "$lib/ipc/client";
    import type {
        CommandError,
        LibraryItem,
        MediaSummary,
    } from "$lib/ipc/types";
    import type { ComponentProps } from "svelte";
    import AsyncStateNotice from "../AsyncStateNotice.svelte";
    import MediaCover from "../MediaCover.svelte";
    import MediaDetailsDialog from "../MediaDetailsDialog.svelte";

    interface Props {
        loadReading?: () => Promise<LibraryItem[]>;
        advanceProgress?: typeof advanceLibraryProgress;
        loadDetails?: ComponentProps<typeof MediaDetailsDialog>["loadDetails"];
        saveEntry?: ComponentProps<typeof MediaDetailsDialog>["saveEntry"];
        deleteEntry?: ComponentProps<typeof MediaDetailsDialog>["deleteEntry"];
        onLibraryChanged?: () => void;
        openDiscovery?: () => void;
    }
    let {
        loadReading = getHomeReading,
        advanceProgress = advanceLibraryProgress,
        loadDetails,
        saveEntry,
        deleteEntry,
        onLibraryChanged,
        openDiscovery,
    }: Props = $props();
    let items = $state<LibraryItem[]>([]);
    let loading = $state(true);
    let error = $state<CommandError | null>(null);
    let pending = $state<number | null>(null);
    let announcement = $state("");
    let selected = $state<LibraryItem | null>(null);
    let opener: HTMLButtonElement | null = null;
    let rail = $state<HTMLOListElement>();

    onMount(() => {
        void refresh();
    });

    async function refresh() {
        error = null;
        try {
            items = await loadReading();
        } catch (reason) {
            error = normalizeCommandError(reason);
        } finally {
            loading = false;
        }
    }

    function canAdvance(item: LibraryItem) {
        return (
            item.entry.progress < 2_147_483_647 &&
            (item.media?.chapters == null ||
                item.entry.progress < item.media.chapters)
        );
    }

    async function advance(item: LibraryItem) {
        if (pending !== null || !canAdvance(item)) return;
        pending = item.entry.anilistId;
        error = null;
        try {
            const entry = await advanceProgress({
                mediaKind: "MANGA",
                anilistId: item.entry.anilistId,
            });
            items = items.map((candidate) =>
                candidate.entry.anilistId === entry.anilistId
                    ? { ...candidate, entry }
                    : candidate,
            );
            announcement = `${item.media?.title ?? `AniList #${entry.anilistId}`}: chapter ${entry.progress} recorded.`;
            onLibraryChanged?.();
        } catch (reason) {
            error = normalizeCommandError(reason);
        } finally {
            pending = null;
        }
    }

    function summary(item: LibraryItem): MediaSummary {
        return {
            mediaKind: "MANGA",
            anilistId: item.entry.anilistId,
            title: item.media?.title ?? `AniList #${item.entry.anilistId}`,
            coverUrl: item.media?.coverUrl ?? null,
            mediaFormat: item.media?.mediaFormat ?? null,
            status: item.media?.status ?? null,
            episodes: null,
            durationMinutes: null,
            chapters: item.media?.chapters ?? null,
            volumes: item.media?.volumes ?? null,
            releaseDate: item.media?.releaseDate ?? null,
            genres: item.media?.genres ?? [],
            tags: item.media?.tags ?? [],
            trending: 0,
        };
    }

    async function closeDetails() {
        selected = null;
        await tick();
        opener?.focus();
    }
</script>

<div
    class="mb-6"
    id="home-panel"
    role="tabpanel"
    aria-labelledby="home-tab"
    tabindex="-1"
>
    <h1 class="sr-only">Home</h1>
    <section
        aria-label="Continue Reading"
        class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-4 sm:p-6"
    >
        <header class="flex items-center justify-between gap-3">
            <div>
                <h2 class="text-foreground text-xl font-light">
                    Continue Reading
                </h2>
                <p class="text-muted mt-1 text-xs">
                    Pick up where you left off.
                </p>
            </div>
            {#if items.length > 1}
                <div class="flex gap-2">
                    <button
                        class="glass-control control-rounded min-h-11 min-w-11"
                        aria-label="Previous reading titles"
                        onclick={() =>
                            rail?.scrollBy({
                                left: -(rail?.clientWidth ?? 0),
                                behavior: "auto",
                            })}>‹</button
                    >
                    <button
                        class="glass-control control-rounded min-h-11 min-w-11"
                        aria-label="Next reading titles"
                        onclick={() =>
                            rail?.scrollBy({
                                left: rail?.clientWidth ?? 0,
                                behavior: "auto",
                            })}>›</button
                    >
                </div>
            {/if}
        </header>
        <p role="status" class="sr-only">{announcement}</p>
        {#if loading}
            <AsyncStateNotice
                status="loading"
                description="Gathering your reading shelf."
            />
        {:else}
            {#if error}<AsyncStateNotice
                    status="error"
                    title="Reading shelf needs attention"
                    {error}
                    onRetry={refresh}
                />{/if}
            {#if items.length === 0 && !error}
                <p class="text-muted mt-6">
                    Your reading shelf is asleep. Add a manga to Reading to
                    begin.
                </p>
                <button
                    class="glass-control control-rounded mt-4 min-h-11 px-4"
                    onclick={openDiscovery}>Find manga</button
                >
            {:else if items.length > 0}
                <ol
                    bind:this={rail}
                    aria-label="Current manga by recent activity"
                    class="reading-rail mt-5 flex snap-x snap-mandatory gap-4 overflow-x-auto pb-3"
                >
                    {#each items as item (item.entry.anilistId)}
                        <li
                            class="glass-control control-rounded border-border grid w-full min-w-0 shrink-0 snap-start grid-cols-[5rem_minmax(0,1fr)] gap-4 border p-4 sm:w-[calc(50%-0.5rem)]"
                        >
                            <div>
                                <MediaCover
                                    title={summary(item).title}
                                    coverUrl={item.media?.coverUrl ?? null}
                                />
                            </div>
                            <div class="min-w-0">
                                <button
                                    class="text-foreground text-left text-lg leading-snug"
                                    aria-label={`Open details for ${summary(item).title}`}
                                    onclick={(event) => {
                                        opener = event.currentTarget;
                                        selected = item;
                                    }}>{summary(item).title}</button
                                >
                                <p class="text-muted mt-2 text-sm">
                                    {item.entry.progress} / {item.media
                                        ?.chapters ?? "?"} chapters
                                </p>
                                <p class="text-muted text-xs">
                                    {item.entry.volumeProgress ?? 0} volumes read
                                </p>
                                {#if !item.media}<p
                                        class="text-muted mt-2 text-xs"
                                    >
                                        Details are not cached.
                                    </p>{/if}
                                {#if item.media?.isStale}<p
                                        class="text-warning mt-2 text-xs"
                                    >
                                        Cached details may be out of date.
                                    </p>{/if}
                                {#if item.media?.chapters == null}<p
                                        class="text-muted mt-2 text-xs"
                                    >
                                        Chapter total unknown.
                                    </p>{:else if item.entry.progress >= item.media.chapters}<p
                                        class="text-success mt-2 text-xs"
                                    >
                                        All known chapters read.
                                    </p>{/if}
                                <button
                                    class="glass-accent control-rounded mt-3 min-h-11 px-4 text-sm disabled:opacity-50"
                                    aria-label={`Read one more chapter of ${summary(item).title}`}
                                    disabled={pending !== null ||
                                        !canAdvance(item)}
                                    onclick={() => advance(item)}
                                    >{pending === item.entry.anilistId
                                        ? "Saving…"
                                        : "+1 chapter"}</button
                                >
                            </div>
                        </li>
                    {/each}
                </ol>
            {/if}
        {/if}
    </section>
    {#if selected}
        <MediaDetailsDialog
            summary={summary(selected)}
            entry={selected.entry}
            {loadDetails}
            {saveEntry}
            {deleteEntry}
            onClose={closeDetails}
            onLibraryChanged={async () => {
                await refresh();
                onLibraryChanged?.();
            }}
        />
    {/if}
</div>
