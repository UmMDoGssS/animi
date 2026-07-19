<script lang="ts">
    import type { AsyncPresentationStatus } from "$lib/state/asyncState";

    type StaticAsyncStatus = Exclude<AsyncPresentationStatus, "loading">;

    interface Props {
        status: StaticAsyncStatus;
        compact?: boolean;
    }

    const artwork: Record<StaticAsyncStatus, string> = {
        empty: [
            "  /\\_/\\  (",
            " ( ^.^ ) _)",
            '   \\"/  (',
            " ( | | )",
            "(__d b__)",
        ].join("\n"),
        stale: "ฅ(´・ω・｀)ฅ",
        offline: ["  /\\_/\\", " ( o.o )", "  m   m"].join("\n"),
        error: [
            ' _._     _,-\'""`-._',
            "(,-.`._,'(       |\\`-/|",
            "    `-.-' \\ )-`( , o o)",
            "          `-    \\`_`\"'-",
        ].join("\n"),
    };

    let { status, compact = false }: Props = $props();
</script>

<div
    aria-hidden="true"
    class:compact
    data-static-cat-art
    data-status={status}
    data-size={compact ? "compact" : "standard"}
>
    <pre>{artwork[status]}</pre>
</div>

<style>
    div {
        display: grid;
        width: 32ch;
        min-width: 32ch;
        height: 5em;
        place-items: center;
        overflow: hidden;
        color: var(--text-muted);
        font-family: inherit;
        font-size: clamp(0.45rem, 1.35vw, 0.6rem);
        font-weight: 500;
        line-height: 1;
        user-select: none;
    }

    pre {
        margin: 0;
        font: inherit;
        line-height: inherit;
        white-space: pre;
    }

    div[data-status="stale"],
    div[data-status="offline"] {
        color: var(--warning);
    }

    div[data-status="error"] {
        color: var(--danger);
    }

    div[data-status="stale"] pre {
        font-size: 1.45em;
    }

    div[data-status="offline"] pre {
        font-size: 1.2em;
    }

    div.compact {
        font-size: clamp(0.42rem, 1.2vw, 0.54rem);
    }
</style>
