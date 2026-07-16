<script lang="ts">
  import { onMount } from "svelte";

  import {
    getThemePreference,
    normalizeCommandError,
    setThemePreference,
  } from "$lib/ipc/client";
  import type { CommandError, ThemePreference } from "$lib/ipc/types";
  import type { PrimaryViewId } from "$lib/navigation";
  import { applyThemePreference } from "$lib/theme/theme";

  import AppNavigation from "./AppNavigation.svelte";
  import DashboardPage from "./pages/DashboardPage.svelte";
  import DiscoveryPage from "./pages/DiscoveryPage.svelte";
  import HomePage from "./pages/HomePage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";

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

  let activeView = $state<PrimaryViewId>("home");
  let themePreference = $state<ThemePreference>("SYSTEM");
  let themeStatus = $state<"loading" | "ready" | "saving" | "error">("loading");
  let themeError = $state<CommandError | null>(null);

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

  function selectView(view: PrimaryViewId) {
    activeView = view;
  }
</script>

<a
  href="#main-content"
  class="bg-accent text-on-accent fixed top-3 left-3 z-50 -translate-y-20 rounded-xl px-4 py-2 text-sm font-semibold transition focus:translate-y-0"
>
  Skip to content
</a>

<div class="bg-surface text-foreground relative min-h-screen overflow-clip">
  <div
    aria-hidden="true"
    class="pointer-events-none fixed inset-0 overflow-hidden"
  >
    <span
      class="bg-accent absolute top-[8%] -left-24 size-72 rounded-full opacity-[0.08] blur-3xl"
    ></span>
    <span
      class="bg-accent-strong absolute right-[-8rem] bottom-[4%] size-96 rounded-full opacity-[0.07] blur-3xl"
    ></span>
  </div>

  <div
    class="relative mx-auto grid min-h-screen max-w-[96rem] grid-cols-[4rem_minmax(0,1fr)] gap-2 p-2 sm:grid-cols-[5.5rem_minmax(0,1fr)] sm:gap-4 sm:p-4"
  >
    <aside class="sticky top-0 flex h-screen items-center py-2 sm:py-4">
      <AppNavigation {activeView} {selectView} />
    </aside>

    <div class="min-w-0">
      <header
        class="flex min-h-20 items-center justify-between gap-3 px-2 py-3 sm:min-h-24 sm:px-3"
      >
        <div class="min-w-0">
          <p class="text-foreground truncate text-lg font-black tracking-tight">
            animi
          </p>
          <p class="text-muted truncate text-xs">Your quiet anime corner</p>
        </div>
        <div
          class="glass-panel border-border flex shrink-0 items-center gap-2 rounded-full border px-3 py-2"
          title="Your library stays on this device"
        >
          <span class="bg-success size-2 rounded-full"></span>
          <span class="text-muted hidden text-xs font-semibold sm:inline">
            Private & local
          </span>
          <span class="text-muted text-xs font-semibold sm:hidden">Local</span>
        </div>
      </header>

      <main id="main-content" class="min-w-0 px-1 sm:px-3">
        {#if activeView === "home"}
          <HomePage />
        {:else if activeView === "discovery"}
          <DiscoveryPage />
        {:else if activeView === "dashboard"}
          <DashboardPage />
        {:else}
          <SettingsPage
            {themePreference}
            {themeStatus}
            {themeError}
            {selectThemePreference}
          />
        {/if}
      </main>
    </div>
  </div>
</div>
