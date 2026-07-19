<script lang="ts">
    interface Props {
        title: string;
        coverUrl: string | null;
        fill?: boolean;
    }

    let { title, coverUrl, fill = false }: Props = $props();

    let failedCoverUrl = $state<string | null>(null);
    let normalizedCoverUrl = $derived(coverUrl?.trim() || null);
    let showCover = $derived(
        normalizedCoverUrl !== null && failedCoverUrl !== normalizedCoverUrl,
    );

    function handleCoverError() {
        failedCoverUrl = normalizedCoverUrl;
    }
</script>

<div
    data-cover-state={showCover ? "image" : "fallback"}
    class={`control-rounded border-border bg-surface-sunken relative w-full max-w-full min-w-0 overflow-hidden border ${fill ? "h-full" : "aspect-[2/3]"}`}
>
    {#if showCover && normalizedCoverUrl}
        <img
            src={normalizedCoverUrl}
            alt={`Cover artwork for ${title}`}
            class="block size-full object-cover"
            loading="lazy"
            decoding="async"
            onerror={handleCoverError}
        />
    {:else}
        <div
            role="img"
            aria-label={`Cover unavailable for ${title}`}
            class="text-muted absolute inset-0 grid min-w-0 place-items-center p-3 text-center"
        >
            <span
                aria-hidden="true"
                class="text-accent absolute top-2.5 left-3 text-[0.58rem] tracking-[0.14em] uppercase"
            >
                &gt;_ no_art
            </span>
            <div aria-hidden="true" class="mt-3 max-w-full">
                <p class="text-foreground text-sm leading-4 tracking-[-0.08em]">
                    /\_/\
                </p>
                <p class="text-foreground text-sm leading-4 tracking-[-0.08em]">
                    ( o.o )
                </p>
                <p class="text-foreground text-sm leading-4 tracking-[-0.08em]">
                    &gt; ^ &lt;
                </p>
                <p class="mt-3 text-[0.58rem] tracking-[0.12em] uppercase">
                    cover unavailable
                </p>
            </div>
        </div>
    {/if}
</div>
