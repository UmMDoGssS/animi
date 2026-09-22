<script lang="ts">
    import type {
        DashboardActivitySummary,
        DashboardActivityBucket,
    } from "$lib/ipc/types";
    let { activity }: { activity: DashboardActivitySummary } = $props();
    function height(
        bucket: DashboardActivityBucket,
        unit: "recordedChapters" | "recordedVolumes",
    ) {
        const maximum = Math.max(
            1,
            ...activity.monthly.map((item) => item[unit]),
        );
        return (bucket[unit] / maximum) * 100;
    }
</script>

<section
    aria-labelledby="reading-activity-heading"
    class="glass-panel panel-large border-border min-w-0 overflow-hidden border p-4 sm:p-6 lg:p-7"
>
    <h2
        id="reading-activity-heading"
        class="text-foreground text-xl font-light"
    >
        Your reading history
    </h2>
    {#if activity.eventCount === 0}
        <p class="text-muted mt-5 text-sm">
            No reading activity yet. Record chapter or volume progress to begin.
        </p>
    {:else}
        <dl
            aria-label="Recorded reading activity"
            class="mt-5 grid grid-cols-2 gap-4"
        >
            <div>
                <dt class="text-muted text-xs">Chapters recorded</dt>
                <dd class="text-foreground mt-1 text-2xl">
                    {activity.recordedChapters}
                </dd>
            </div>
            <div>
                <dt class="text-muted text-xs">Volumes recorded</dt>
                <dd class="text-foreground mt-1 text-2xl">
                    {activity.recordedVolumes}
                </dd>
            </div>
        </dl>
        <p class="text-muted mt-3 text-xs">
            Recorded from {activity.coveredFrom?.slice(0, 10)} to {activity.coveredTo?.slice(
                0,
                10,
            )} (UTC). Corrections do not add activity.
        </p>
        {#each [{ unit: "recordedChapters", label: "chapters" }, { unit: "recordedVolumes", label: "volumes" }] as series (series.unit)}
            <h3 class="text-foreground mt-6 text-base">
                Monthly {series.label}
            </h3>
            <p class="sr-only">
                Each chart uses its own scale. Months are grouped in UTC.
            </p>
            <div class="mt-3 overflow-x-auto pb-2">
                <ol
                    aria-label={`Monthly recorded ${series.label}`}
                    class="grid min-w-[32rem] grid-cols-12 gap-2"
                >
                    {#each activity.monthly as bucket (bucket.period)}
                        <li
                            aria-label={`${bucket.period}: ${series.unit === "recordedChapters" ? bucket.recordedChapters : bucket.recordedVolumes} ${series.label}`}
                        >
                            <div
                                aria-hidden="true"
                                class="bg-glass-control flex h-20 items-end rounded-sm"
                            >
                                <span
                                    class="bg-accent block w-full rounded-t-sm"
                                    style:height={`${height(bucket, series.unit === "recordedChapters" ? "recordedChapters" : "recordedVolumes")}%`}
                                ></span>
                            </div>
                            <p class="text-foreground mt-1 text-center text-xs">
                                {series.unit === "recordedChapters"
                                    ? bucket.recordedChapters
                                    : bucket.recordedVolumes}
                            </p>
                            <time
                                class="text-muted block text-center text-[0.6rem]"
                                datetime={`${bucket.period}-01`}
                                >{bucket.period.slice(5)}</time
                            >
                        </li>
                    {/each}
                </ol>
            </div>
        {/each}
        <h3 class="text-foreground mt-5">Yearly totals</h3>
        <ul
            aria-label="Yearly recorded reading activity"
            class="border-border mt-2 divide-y border-y"
        >
            {#each activity.yearly as bucket (bucket.period)}
                <li class="flex flex-wrap justify-between gap-2 py-3 text-sm">
                    <span>{bucket.period}</span><span
                        >{bucket.recordedChapters} chapters · {bucket.recordedVolumes}
                        volumes</span
                    >
                </li>
            {/each}
        </ul>
    {/if}
</section>
