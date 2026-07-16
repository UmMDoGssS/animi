<script lang="ts">
  import { onMount } from "svelte";

  import {
    getThemePreference,
    normalizeCommandError,
    setThemePreference,
  } from "$lib/ipc/client";
  import type { CommandError, ThemePreference } from "$lib/ipc/types";
  import {
    detailsPlaceholder,
    navigationItems,
    type AppViewId,
  } from "$lib/navigation";
  import { applyThemePreference } from "$lib/theme/theme";

  import SmokePanel from "./SmokePanel.svelte";
  import ThemePreferenceControl from "./ThemePreferenceControl.svelte";
  import ViewPlaceholder from "./ViewPlaceholder.svelte";

  interface Props {
    loadThemePreference?: () => Promise<ThemePreference>;
    saveThemePreference?: (
      preference: ThemePreference,
    ) => Promise<ThemePreference>;
  }

  let {
    loadThemePreference = getThemePreference,
    saveThemePreference = setThemePreference,
  }: Props = $props();

  let activeView = $state<AppViewId>("library");
  let themePreference = $state<ThemePreference>("SYSTEM");
  let themeStatus = $state<"loading" | "ready" | "saving" | "error">("loading");
  let themeError = $state<CommandError | null>(null);
  let activeItem = $derived(
    activeView === "details"
      ? detailsPlaceholder
      : (navigationItems.find((item) => item.id === activeView) ??
          navigationItems[0]),
  );

  onMount(() => {
    void loadPreference();
  });

  async function loadPreference() {
    try {
      themePreference = await loadThemePreference();
      applyThemePreference(themePreference);
      themeStatus = "ready";
    } catch (reason) {
      themeError = normalizeCommandError(reason);
      themeStatus = "error";
    }
  }

  async function selectThemePreference(preference: ThemePreference) {
    if (preference === themePreference && themeStatus !== "error") {
      return;
    }

    themeStatus = "saving";
    themeError = null;
    try {
      themePreference = await saveThemePreference(preference);
      applyThemePreference(themePreference);
      themeStatus = "ready";
    } catch (reason) {
      themeError = normalizeCommandError(reason);
      themeStatus = "error";
    }
  }
</script>

<div class="bg-surface text-foreground min-h-screen">
  <header class="border-border bg-surface-raised border-b">
    <div class="mx-auto flex max-w-7xl items-center gap-3 px-4 py-4 sm:px-6">
      <div
        aria-hidden="true"
        class="bg-accent text-on-accent grid size-10 place-items-center rounded-2xl text-xl"
      >
        =^
      </div>
      <div>
        <p class="text-lg font-bold tracking-tight">Animi</p>
        <p class="text-muted text-xs">Your quiet anime corner</p>
      </div>
    </div>
  </header>

  <div
    class="mx-auto grid max-w-7xl gap-5 px-4 py-5 sm:px-6 lg:grid-cols-[14rem_1fr] lg:py-8"
  >
    <nav
      aria-label="Primary"
      class="glass-panel border-border bg-glass rounded-3xl border p-2 lg:self-start"
    >
      <ul class="grid grid-cols-2 gap-1 sm:grid-cols-4 lg:grid-cols-1">
        {#each navigationItems as item (item.id)}
          <li>
            <button
              type="button"
              aria-current={activeView === item.id ? "page" : undefined}
              class="hover:bg-overlay aria-[current=page]:bg-accent aria-[current=page]:text-on-accent w-full rounded-2xl px-3 py-3 text-left text-sm font-medium transition"
              onclick={() => (activeView = item.id)}
            >
              {item.label}
            </button>
          </li>
        {/each}
      </ul>
    </nav>

    <main class="grid min-w-0 gap-5">
      <ViewPlaceholder
        title={activeItem.title}
        description={activeItem.description}
      />
      {#if activeView === "settings"}
        <ThemePreferenceControl
          preference={themePreference}
          status={themeStatus}
          error={themeError}
          selectPreference={selectThemePreference}
        />
      {/if}
      {#if activeView === "discover"}
        <button
          type="button"
          class="border-border bg-surface-raised text-foreground hover:bg-overlay justify-self-start rounded-xl border px-4 py-2 text-sm font-semibold transition"
          onclick={() => (activeView = "details")}
        >
          Preview details view
        </button>
      {:else if activeView === "details"}
        <button
          type="button"
          class="border-border bg-surface-raised text-foreground hover:bg-overlay justify-self-start rounded-xl border px-4 py-2 text-sm font-semibold transition"
          onclick={() => (activeView = "discover")}
        >
          Back to discover
        </button>
      {/if}
      <SmokePanel />
    </main>
  </div>
</div>
