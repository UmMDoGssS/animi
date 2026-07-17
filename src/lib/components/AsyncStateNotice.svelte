<script lang="ts">
  import type { CommandError } from "$lib/ipc/types";
  import type { AsyncPresentationStatus } from "$lib/state/asyncState";

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
  class="glass-control control-rounded border-border min-w-0 border {compact
    ? 'p-3'
    : 'p-4 sm:p-5'}"
>
  <div class="flex min-w-0 gap-3">
    <span
      aria-hidden="true"
      class="mt-1.5 size-2 shrink-0 rounded-full"
      class:bg-accent={status === "loading"}
      class:animate-pulse={status === "loading"}
      class:bg-muted={status === "empty"}
      class:bg-warning={status === "offline" || status === "stale"}
      class:bg-danger={status === "error"}
    ></span>

    <div class="min-w-0 flex-1">
      <p
        class="text-foreground mt-1 font-medium"
        class:text-sm={compact}
        class:text-base={!compact}
      >
        {resolvedTitle}
      </p>
      <p class="text-muted mt-1 text-sm leading-6">{resolvedDescription}</p>

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
