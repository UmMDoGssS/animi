<script lang="ts">
    import type { CommandError } from "$lib/ipc/types";
    import type { AsyncPresentationStatus } from "$lib/state/asyncState";

    import SleepingCatLoader from "./SleepingCatLoader.svelte";
    import StaticCatArt from "./StaticCatArt.svelte";

    interface Props {
        status: AsyncPresentationStatus;
        title?: string;
        description?: string;
        error?: CommandError | null;
        onRetry?: () => void | Promise<void>;
        retryLabel?: string;
        compact?: boolean;
    }

    const defaultCopy: Record<
        AsyncPresentationStatus,
        { title: string; description: string }
    > = {
        loading: {
            title: "Loading",
            description: "This may take a moment.",
        },
        empty: {
            title: "Nothing here yet",
            description: "There is nothing to show yet.",
        },
        offline: {
            title: "You're offline",
            description: "Some anime may be unavailable until you reconnect.",
        },
        stale: {
            title: "This may be out of date",
            description: "Reconnect or try again to update it.",
        },
        error: {
            title: "This could not load",
            description: "Try again.",
        },
    };

    let {
        status,
        title,
        description,
        error = null,
        onRetry,
        retryLabel = "Try again",
        compact = false,
    }: Props = $props();

    let resolvedTitle = $derived(title ?? defaultCopy[status].title);
    let resolvedDescription = $derived(
        description ??
            (status === "error" && error
                ? error.message
                : defaultCopy[status].description),
    );
    let noticeRole = $derived(status === "error" ? "alert" : "status");
    let canRetry = $derived(
        error !== null && error.retryable && onRetry !== undefined,
    );
</script>

<section
    role={noticeRole}
    aria-atomic="true"
    aria-busy={status === "loading"}
    data-status={status}
    class="async-state-notice glass-control control-rounded border-border min-w-0 border {compact
        ? 'p-3'
        : 'p-4 sm:p-5'}"
>
    <div class="flex min-w-0 items-center gap-3">
        {#if status === "loading"}
            <SleepingCatLoader {compact} />
        {:else}
            <StaticCatArt {status} {compact} />
        {/if}

        <div class="min-w-0 flex-1">
            <p
                class="text-foreground mt-1 font-medium"
                class:text-sm={compact}
                class:text-base={!compact}
            >
                {resolvedTitle}
            </p>
            <p class="text-muted mt-1 text-sm leading-6">
                {resolvedDescription}
            </p>

            {#if canRetry}
                <button
                    type="button"
                    class="glass-accent control-rounded mt-3 px-3 py-2 text-xs font-medium transition hover:opacity-90"
                    onclick={() => void onRetry?.()}
                >
                    {retryLabel}
                </button>
            {/if}
        </div>
    </div>
</section>

<style>
    .async-state-notice {
        animation: async-notice-enter 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    @keyframes async-notice-enter {
        from {
            opacity: 0;
            transform: translateY(0.35rem) scale(0.995);
        }

        to {
            opacity: 1;
            transform: translateY(0) scale(1);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .async-state-notice {
            animation: none;
        }
    }
</style>
