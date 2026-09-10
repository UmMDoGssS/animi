<script lang="ts">
    import type { MediaKind } from "$lib/ipc/types";

    interface Props {
        value: MediaKind;
        onChange: (kind: MediaKind) => void;
        label?: string;
    }

    let { value, onChange, label = "Media type" }: Props = $props();
</script>

<fieldset class="glass-control flex shrink-0 gap-1 rounded-xl p-1">
    <legend class="sr-only">{label}</legend>
    {#each ["ANIME", "MANGA"] as kind (kind)}
        <label
            class="control-rounded has-focus-visible:outline-accent relative cursor-pointer px-4 py-2 text-sm has-focus-visible:outline-2 has-focus-visible:outline-offset-2"
            class:glass-accent={value === kind}
        >
            <input
                class="sr-only"
                type="radio"
                name={label}
                value={kind}
                data-media-kind={kind}
                checked={value === kind}
                onchange={() => onChange(kind === "ANIME" ? "ANIME" : "MANGA")}
            />
            {kind === "ANIME" ? "Anime" : "Manga"}
        </label>
    {/each}
</fieldset>
