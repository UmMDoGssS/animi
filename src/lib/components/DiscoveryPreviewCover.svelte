<script lang="ts">
  import type { DiscoveryCoverMotif } from "./discoveryPreview";

  interface Props {
    title: string;
    motif: DiscoveryCoverMotif;
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
    <path
      d="M14 20h132M14 220h132"
      fill="none"
      stroke="var(--border)"
      stroke-width="1"
      opacity="0.7"
    />

    {#if motif === "crane"}
      <circle cx="116" cy="56" r="38" fill="var(--accent)" opacity="0.24" />
      <path
        d="m27 138 49-28 19 12 37-25-27 37-12-4-20 43-7-42-39 7Zm49-28 1-36 18 48"
        fill="none"
        stroke="var(--text)"
        stroke-width="2.5"
        stroke-linejoin="round"
      />
      <path
        d="M18 188h124"
        fill="none"
        stroke="var(--accent-strong)"
        stroke-width="10"
        opacity="0.2"
      />
    {:else if motif === "shrine"}
      <rect
        x="28"
        y="52"
        width="104"
        height="132"
        fill="var(--accent-strong)"
        opacity="0.12"
      />
      <path
        d="M35 74h90M45 57h70M52 74v110m56-110v110M39 184h82"
        fill="none"
        stroke="var(--accent)"
        stroke-width="5"
        stroke-linecap="square"
      />
      <path
        d="M67 137c0-9 6-16 13-16s13 7 13 16v20H67v-20Zm2-13-7-12 14 7m15 5 7-12-14 7"
        fill="var(--surface-sunken)"
        stroke="var(--text)"
        stroke-width="2"
        stroke-linejoin="round"
      />
    {:else if motif === "comet"}
      <circle cx="103" cy="75" r="24" fill="var(--accent)" opacity="0.64" />
      <path
        d="M112 61C82 73 55 100 34 139m91-64c-35 14-58 37-76 68m87-49c-37 16-60 38-74 66"
        fill="none"
        stroke="var(--accent-strong)"
        stroke-width="3"
        stroke-linecap="round"
        opacity="0.62"
      />
      <path
        d="m30 52 2.8 5.6 6.2.9-4.5 4.4 1.1 6.1-5.6-2.9-5.6 2.9 1.1-6.1-4.5-4.4 6.2-.9L30 52Z"
        fill="var(--text)"
        opacity="0.74"
      />
    {:else if motif === "archive"}
      <path d="M26 42h108v146H26z" fill="var(--overlay)" opacity="0.42" />
      <path
        d="M47 42v146m33-146v146m33-146v146M26 82h108M26 128h108M26 166h108"
        fill="none"
        stroke="var(--text)"
        stroke-width="1.5"
        opacity="0.58"
      />
      <path
        d="M16 151c16-10 31-10 46 0s30 10 46 0 28-10 42-1v48H16v-47Z"
        fill="var(--accent-strong)"
        opacity="0.32"
      />
      <circle cx="80" cy="105" r="18" fill="var(--accent)" opacity="0.64" />
      <circle cx="80" cy="105" r="7" fill="var(--surface-sunken)" />
    {:else if motif === "tide"}
      <circle cx="82" cy="74" r="43" fill="var(--accent)" opacity="0.18" />
      <path
        d="m37 64 2.8 5.6 6.2.9-4.5 4.4 1.1 6.1-5.6-2.9-5.6 2.9 1.1-6.1-4.5-4.4 6.2-.9L37 64Zm80 27 2 4 4.5.7-3.3 3.1.8 4.5-4-2.1-4 2.1.8-4.5-3.3-3.1 4.5-.7 2-4Z"
        fill="var(--text)"
      />
      <path
        d="M10 145c19-15 39-15 58 0s39 15 58 0 31-15 43-4m-159 27c19-15 39-15 58 0s39 15 58 0 31-15 43-4m-159 28c19-15 39-15 58 0s39 15 58 0 31-15 43-4"
        fill="none"
        stroke="var(--accent-strong)"
        stroke-width="4"
        opacity="0.55"
      />
    {:else}
      <rect
        x="28"
        y="58"
        width="104"
        height="105"
        rx="6"
        fill="var(--accent)"
        opacity="0.2"
      />
      <path
        d="M37 83h86v70H37zM37 103h86M37 126h86M64 83v70m34-70v70"
        fill="none"
        stroke="var(--text)"
        stroke-width="2"
      />
      <circle
        cx="80"
        cy="115"
        r="16"
        fill="var(--accent-strong)"
        opacity="0.6"
      />
      <path
        d="M54 177c12-8 40-8 52 0m-64 17h76"
        fill="none"
        stroke="var(--accent)"
        stroke-width="3"
        stroke-linecap="round"
      />
    {/if}
  </svg>

  <div
    aria-hidden="true"
    class="text-foreground absolute inset-x-2 bottom-2 flex items-center justify-between text-[0.56rem] tracking-[0.12em] uppercase"
  >
    <span>sample</span>
    <span>{code}</span>
  </div>
</div>
