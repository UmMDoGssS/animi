<script lang="ts">
    export type HomeCoverMotif = "moon" | "rail" | "signal";

    interface Props {
        title: string;
        motif: HomeCoverMotif;
        code: string;
    }

    let { title, motif, code }: Props = $props();
</script>

<div
    role="img"
    aria-label={`Sample cover artwork for ${title}`}
    class="control-rounded border-border bg-glass-control relative aspect-[2/3] w-full overflow-hidden border"
>
    <svg
        aria-hidden="true"
        viewBox="0 0 160 240"
        class="absolute inset-0 size-full"
    >
        <rect width="160" height="240" fill="var(--surface-sunken)" />

        {#if motif === "moon"}
            <circle
                cx="112"
                cy="58"
                r="45"
                fill="var(--accent)"
                opacity="0.28"
            />
            <circle
                cx="126"
                cy="45"
                r="38"
                fill="var(--surface-sunken)"
                opacity="0.92"
            />
            <path
                d="M24 171 46 133l23 38 18-25 35 57H7l17-32Z"
                fill="var(--accent-strong)"
                opacity="0.32"
            />
            <path
                d="m45 94 9-13 9 13v25H45V94Zm7 10h4m-2-8v12"
                fill="none"
                stroke="var(--text)"
                stroke-width="2"
                stroke-linecap="round"
            />
            <path
                d="m28 42 2.5 5 5.5.8-4 3.9.9 5.5-4.9-2.6-4.9 2.6.9-5.5-4-3.9 5.5-.8L28 42Z"
                fill="var(--text)"
                opacity="0.72"
            />
        {:else if motif === "rail"}
            <rect
                x="18"
                y="24"
                width="124"
                height="154"
                rx="8"
                fill="var(--accent-strong)"
                opacity="0.18"
            />
            <path
                d="M44 28v145M116 28v145M44 75h72M44 123h72"
                fill="none"
                stroke="var(--text)"
                stroke-width="2"
                opacity="0.55"
            />
            <path
                d="m24 216 38-48m74 48-38-48M59 216l12-48m30 48-12-48"
                fill="none"
                stroke="var(--accent)"
                stroke-width="3"
                opacity="0.7"
            />
            <path
                d="M68 96c0-8 6-14 12-14s12 6 12 14v17H68V96Zm2-12-5-10 11 6m14 4 5-10-11 6"
                fill="var(--surface-sunken)"
                stroke="var(--text)"
                stroke-width="2"
                stroke-linejoin="round"
            />
        {:else}
            <circle
                cx="80"
                cy="92"
                r="49"
                fill="var(--overlay)"
                opacity="0.56"
            />
            <circle
                cx="80"
                cy="92"
                r="34"
                fill="none"
                stroke="var(--accent)"
                stroke-width="2"
                stroke-dasharray="4 7"
            />
            <path
                d="M65 78c2-13 28-15 30 1 1 11-15 13-15 24"
                fill="none"
                stroke="var(--text)"
                stroke-width="5"
                stroke-linecap="round"
            />
            <circle cx="80" cy="119" r="3" fill="var(--text)" />
            <path
                d="M22 179h116M34 194h92M48 209h64"
                fill="none"
                stroke="var(--accent-strong)"
                stroke-width="2"
                opacity="0.64"
            />
        {/if}
    </svg>

    <div
        aria-hidden="true"
        class="text-foreground absolute inset-x-2 bottom-2 flex items-center justify-between text-[0.58rem] tracking-[0.14em] uppercase"
    >
        <span>animi</span>
        <span>{code}</span>
    </div>
</div>
