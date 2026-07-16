<script lang="ts">
  import type { CommandError, ThemePreference } from "$lib/ipc/types";
  import { themeOptions } from "$lib/theme/theme";

  interface Props {
    preference: ThemePreference;
    status: "loading" | "ready" | "saving" | "error";
    error: CommandError | null;
    selectPreference: (preference: ThemePreference) => Promise<void>;
  }

  let { preference, status, error, selectPreference }: Props = $props();
  let displayedPreference = $derived(preference);

  async function choosePreference(nextPreference: ThemePreference) {
    await selectPreference(nextPreference);
    displayedPreference = preference;
  }
</script>

<section
  aria-labelledby="theme-preference-heading"
  class="glass-panel border-border rounded-3xl border p-5 sm:p-6"
>
  <div class="max-w-2xl">
    <h2 id="theme-preference-heading" class="text-foreground font-semibold">
      Appearance
    </h2>
    <p class="text-muted mt-1 text-sm">
      Choose how Animi picks its Catppuccin palette. Your choice stays on this
      device.
    </p>
  </div>

  <fieldset class="mt-5" disabled={status === "loading" || status === "saving"}>
    <legend class="sr-only">Theme preference</legend>
    <div class="grid gap-3 sm:grid-cols-3">
      {#each themeOptions as option (option.value)}
        <label
          class="border-border bg-surface has-checked:border-accent has-checked:ring-accent/30 hover:bg-overlay flex cursor-pointer gap-3 rounded-2xl border p-4 transition has-checked:ring-2"
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
            <span class="text-foreground block text-sm font-semibold">
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
    <p class="text-muted mt-3 text-sm" role="status">Loading preference…</p>
  {:else if status === "saving"}
    <p class="text-muted mt-3 text-sm" role="status">Saving preference…</p>
  {:else if status === "error" && error}
    <p class="text-danger mt-3 text-sm" role="alert">{error.message}</p>
  {:else}
    <p class="text-muted mt-3 text-sm" role="status">Preference saved.</p>
  {/if}
</section>
