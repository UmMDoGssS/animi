<script lang="ts">
    import type { NavigationIconId } from "$lib/navigation";

    interface Props {
        name: NavigationIconId;
    }

    let { name }: Props = $props();
</script>

<svg
    aria-hidden="true"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.6"
    stroke-linecap="round"
    stroke-linejoin="round"
    class="size-5"
>
    {#if name === "home"}
        <path d="m3.5 10.5 8.5-7 8.5 7" />
        <path d="M5.5 9v10.5h13V9M9.5 19.5v-6h5v6" />
    {:else if name === "discover"}
        <circle cx="11" cy="11" r="7.25" />
        <path d="m16.25 16.25 4.25 4.25M13.75 8.25l-1.5 4-4 1.5 1.5-4 4-1.5Z" />
    {:else if name === "dashboard"}
        <path
            d="M4 19.5V12h4v7.5H4ZM10 19.5V4.5h4v15h-4ZM16 19.5v-11h4v11h-4Z"
        />
    {:else}
        <path d="M4 6h10M18 6h2M4 12h3M11 12h9M4 18h8M16 18h4" />
        <circle cx="16" cy="6" r="2" />
        <circle cx="9" cy="12" r="2" />
        <circle cx="14" cy="18" r="2" />
    {/if}
</svg>
