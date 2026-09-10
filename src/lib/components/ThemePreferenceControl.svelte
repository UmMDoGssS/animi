<script lang="ts">
    import type { CommandError, ThemePreference } from "$lib/ipc/types";
    import { themeOptions } from "$lib/theme/theme";

    import AsyncStateNotice from "./AsyncStateNotice.svelte";

    interface Props {
        preference: ThemePreference;
        status: "loading" | "ready" | "saving" | "error";
        error: CommandError | null;
        selectPreference: (preference: ThemePreference) => Promise<void>;
        retryPreference: () => Promise<void>;
    }

    let {
        preference,
        status,
        error,
        selectPreference,
        retryPreference,
    }: Props = $props();
    let displayedPreference = $derived(preference);
    let hasRequestedChange = $state(false);
    let activeOption = $derived(
        themeOptions.find((option) => option.value === preference) ??
            themeOptions[0],
    );

    async function choosePreference(nextPreference: ThemePreference) {
        hasRequestedChange = true;
        await selectPreference(nextPreference);
        displayedPreference = preference;
    }
</script>

<section
    aria-labelledby="theme-preference-heading"
    aria-busy={status === "loading" || status === "saving"}
    class="glass-panel panel-large border-border border p-5 sm:p-6"
>
    <div class="max-w-2xl">
        <h2 id="theme-preference-heading" class="text-foreground font-medium">
            Appearance
        </h2>
        <p class="text-muted mt-1 text-sm">
            Choose light, dark, or system colors.
        </p>
    </div>

    <fieldset
        class="mt-5"
        disabled={status === "loading" || status === "saving"}
    >
        <legend class="sr-only">Theme preference</legend>
        <div class="grid grid-cols-3 gap-2">
            {#each themeOptions as option (option.value)}
                <label
                    class="theme-option glass-control glass-interactive control-rounded border-border has-checked:border-accent has-checked:ring-accent/30 flex cursor-pointer flex-col gap-3 border p-3 transition has-checked:ring-2"
                >
                    <span
                        aria-hidden="true"
                        class="theme-swatch"
                        data-palette={option.value}><i></i><i></i><i></i></span
                    >
                    <input
                        type="radio"
                        name="theme-preference"
                        value={option.value}
                        bind:group={displayedPreference}
                        class="accent-accent mt-0.5"
                        onchange={() => void choosePreference(option.value)}
                    />
                    <span>
                        <span class="text-foreground block text-sm font-medium">
                            {option.label}
                        </span>
                        <span class="text-muted mt-1 block text-xs leading-5">
                            {option.description}
                        </span>
                    </span>
                </label>
            {/each}
        </div>
    </fieldset>

    {#if status === "loading"}
        <div class="mt-4">
            <AsyncStateNotice
                status="loading"
                title="Loading your theme"
                description="System colors stay active for now."
                compact
            />
        </div>
    {:else if status === "saving"}
        <div class="mt-4">
            <AsyncStateNotice
                status="loading"
                title="Saving your theme"
                description="The current colors stay active for now."
                compact
            />
        </div>
    {:else if status === "error" && error}
        <div class="mt-4">
            <AsyncStateNotice
                status="error"
                title="Appearance stayed unchanged"
                {error}
                onRetry={retryPreference}
                retryLabel="Retry preference"
                compact
            />
        </div>
    {:else}
        <p class="text-muted mt-4 text-sm leading-6">
            <span class="text-foreground">{activeOption.label} is active.</span>
            {activeOption.description}
        </p>
        {#if hasRequestedChange}
            <p class="sr-only" role="status">
                Theme preference saved. {activeOption.label} is active.
            </p>
        {/if}
    {/if}
</section>

<style>
    .theme-swatch {
        display: block;
        width: 100%;
        height: 86px;
        padding: 12px;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--preview-dark);
    }
    .theme-swatch[data-palette="LATTE"] {
        background: var(--preview-light);
    }
    .theme-swatch[data-palette="SYSTEM"] {
        background: linear-gradient(
            90deg,
            var(--preview-light) 50%,
            var(--preview-dark) 50%
        );
    }
    .theme-swatch i {
        display: block;
        width: 75%;
        height: 7px;
        border-radius: 3px;
        margin: 8px 0;
        background: var(--preview-line);
    }

    .theme-option:has(input:focus-visible) {
        outline: 2px solid var(--accent);
        outline-offset: 3px;
    }
</style>
