<script lang="ts">
  import { onMount } from "svelte";

  import {
    getThemePreference,
    normalizeCommandError,
    setThemePreference,
  } from "$lib/ipc/client";
  import type {
    CommandError,
    DashboardInsights,
    LibraryEntry,
    LibraryItem,
    MediaDetailsRequest,
    MediaDetailsResponse,
    MediaPageRequest,
    MediaPageResponse,
    RefreshLibraryMetadataResponse,
    RemoveLibraryEntryRequest,
    RemoveLibraryEntryResponse,
    SearchMediaRequest,
    ThemePreference,
    UpsertLibraryEntryRequest,
  } from "$lib/ipc/types";
  import type { PrimaryViewId } from "$lib/navigation";
  import { applyThemePreference } from "$lib/theme/theme";

  import AppNavigation from "./AppNavigation.svelte";
  import BackgroundCat from "./BackgroundCat.svelte";
  import DashboardPage from "./pages/DashboardPage.svelte";
  import DiscoveryPage from "./pages/DiscoveryPage.svelte";
  import HomePage from "./pages/HomePage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";

  interface Props {
    loadThemePreference?: () => Promise<ThemePreference>;
    saveThemePreference?: (
      preference: ThemePreference,
    ) => Promise<ThemePreference>;
    p4Services?: P4ServiceOverrides;
  }

  interface P4ServiceOverrides {
    loadTrending?: (request: MediaPageRequest) => Promise<MediaPageResponse>;
    searchAnime?: (request: SearchMediaRequest) => Promise<MediaPageResponse>;
    loadDetails?: (
      request: MediaDetailsRequest,
    ) => Promise<MediaDetailsResponse>;
    loadLibrary?: () => Promise<LibraryItem[]>;
    saveEntry?: (request: UpsertLibraryEntryRequest) => Promise<LibraryEntry>;
    deleteEntry?: (
      request: RemoveLibraryEntryRequest,
    ) => Promise<RemoveLibraryEntryResponse>;
    loadInsights?: () => Promise<DashboardInsights>;
    refreshMetadata?: () => Promise<RefreshLibraryMetadataResponse>;
  }

  type FailedThemeOperation =
    { kind: "load" } | { kind: "save"; preference: ThemePreference };

  let {
    loadThemePreference = getThemePreference,
    saveThemePreference = setThemePreference,
    p4Services = {},
  }: Props = $props();

  let activeView = $state<PrimaryViewId>("home");
  let themePreference = $state<ThemePreference>("SYSTEM");
  let themeStatus = $state<"loading" | "ready" | "saving" | "error">("loading");
  let themeError = $state<CommandError | null>(null);
  let failedThemeOperation = $state<FailedThemeOperation | null>(null);

  onMount(() => {
    void loadPreference();
  });

  async function loadPreference() {
    themeStatus = "loading";
    themeError = null;

    try {
      themePreference = await loadThemePreference();
      applyThemePreference(themePreference);
      failedThemeOperation = null;
      themeStatus = "ready";
    } catch (reason) {
      themeError = normalizeCommandError(reason);
      failedThemeOperation = { kind: "load" };
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
      failedThemeOperation = null;
      themeStatus = "ready";
    } catch (reason) {
      themeError = normalizeCommandError(reason);
      failedThemeOperation = { kind: "save", preference };
      themeStatus = "error";
    }
  }

  async function retryThemePreference() {
    if (!themeError?.retryable || failedThemeOperation === null) {
      return;
    }

    if (failedThemeOperation.kind === "load") {
      await loadPreference();
      return;
    }

    await selectThemePreference(failedThemeOperation.preference);
  }

  function selectView(view: PrimaryViewId) {
    activeView = view;
    document.documentElement.scrollTop = 0;
    document.body.scrollTop = 0;
  }
</script>

<a
  href="#main-content"
  class="control-rounded bg-accent text-on-accent fixed top-3 left-3 z-50 -translate-y-20 px-4 py-2 text-sm font-medium transition focus:translate-y-0"
>
  Skip to content
</a>

<div class="bg-surface text-foreground relative min-h-screen overflow-clip">
  <div
    aria-hidden="true"
    class="pointer-events-none fixed inset-0 overflow-hidden"
  >
    <span
      class="bg-accent absolute -top-24 left-[12%] size-[24rem] rounded-full opacity-[0.14] blur-[72px]"
    ></span>
    <span
      class="bg-accent-strong absolute top-[38%] -right-28 size-[28rem] rounded-full opacity-[0.12] blur-[88px]"
    ></span>
    <span
      class="bg-overlay absolute bottom-[-10rem] left-[24%] size-[22rem] rounded-full opacity-35 blur-[64px]"
    ></span>
    <BackgroundCat />
  </div>

  <div
    class="relative mx-auto grid min-h-screen max-w-[96rem] grid-cols-[3.5rem_minmax(0,1fr)] gap-2 p-2 sm:gap-3 sm:p-4"
  >
    <aside
      aria-label="Primary navigation rail"
      class="sticky top-2 z-30 flex h-[calc(100dvh-1rem)] items-center self-start py-2 sm:top-4 sm:h-[calc(100dvh-2rem)] sm:py-4"
    >
      <AppNavigation {activeView} {selectView} />
    </aside>

    <div class="min-w-0">
      <header class="flex min-h-20 items-center px-2 py-3 sm:min-h-24 sm:px-3">
        <div class="min-w-0">
          <p
            class="text-foreground truncate text-lg font-light tracking-[0.18em]"
          >
            animi
          </p>
          <p class="text-muted truncate text-xs tracking-wide">
            Your quiet anime corner
          </p>
        </div>
      </header>

      <main id="main-content" tabindex="-1" class="min-w-0 px-1 sm:px-3">
        {#if activeView === "home"}
          <HomePage openDiscovery={() => selectView("discovery")} />
        {:else if activeView === "discovery"}
          <DiscoveryPage
            loadTrending={p4Services.loadTrending}
            searchAnime={p4Services.searchAnime}
            loadDetails={p4Services.loadDetails}
            loadLibrary={p4Services.loadLibrary}
            saveEntry={p4Services.saveEntry}
            deleteEntry={p4Services.deleteEntry}
          />
        {:else if activeView === "dashboard"}
          <DashboardPage
            loadInsights={p4Services.loadInsights}
            refreshMetadata={p4Services.refreshMetadata}
            loadDetails={p4Services.loadDetails}
            saveEntry={p4Services.saveEntry}
            deleteEntry={p4Services.deleteEntry}
            openDiscovery={() => selectView("discovery")}
          />
        {:else}
          <SettingsPage
            {themePreference}
            {themeStatus}
            {themeError}
            {selectThemePreference}
            {retryThemePreference}
          />
        {/if}
      </main>
    </div>
  </div>
</div>
