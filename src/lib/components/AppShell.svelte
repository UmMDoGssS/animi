<script lang="ts">
  import { onMount } from "svelte";

  import {
    clearLocalData,
    exportBackup,
    getThemePreference,
    importBackup,
    normalizeCommandError,
    previewImport,
    setThemePreference,
  } from "$lib/ipc/client";
  import type {
    CommandError,
    AdvanceLibraryProgressRequest,
    ClearLocalDataRequest,
    ClearLocalDataResponse,
    DashboardInsights,
    ExportBackupResponse,
    HomeAiringResponse,
    ImportBackupRequest,
    ImportBackupResponse,
    ImportPreviewResponse,
    LibraryEntry,
    LibraryItem,
    MediaDetailsRequest,
    MediaDetailsResponse,
    MediaPageRequest,
    MediaPageResponse,
    RefreshLibraryMetadataResponse,
    RecommendationsRequest,
    RecommendationsResponse,
    RemoveLibraryEntryRequest,
    RemoveLibraryEntryResponse,
    SearchMediaRequest,
    ThemePreference,
    UpsertLibraryEntryRequest,
  } from "$lib/ipc/types";
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
    p4Services?: P4ServiceOverrides;
    p5Services?: P5ServiceOverrides;
    p6Services?: P6ServiceOverrides;
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

  interface P6ServiceOverrides {
    saveBackup?: () => Promise<ExportBackupResponse | null>;
    chooseImport?: () => Promise<ImportPreviewResponse | null>;
    restoreBackup?: (
      request: ImportBackupRequest,
    ) => Promise<ImportBackupResponse>;
    clearData?: (
      request: ClearLocalDataRequest,
    ) => Promise<ClearLocalDataResponse>;
  }

  interface P5ServiceOverrides {
    loadAiring?: () => Promise<HomeAiringResponse>;
    advanceProgress?: (
      request: AdvanceLibraryProgressRequest,
    ) => Promise<LibraryEntry>;
    refreshIntervalMs?: number;
    loadRecommendations?: (
      request: RecommendationsRequest,
    ) => Promise<RecommendationsResponse>;
  }

  type FailedThemeOperation =
    { kind: "load" } | { kind: "save"; preference: ThemePreference };

  let {
    loadThemePreference = getThemePreference,
    saveThemePreference = setThemePreference,
    p4Services = {},
    p5Services = {},
    p6Services = {},
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

<div
  class="app-chrome bg-surface text-foreground relative min-h-screen overflow-clip"
  data-page={activeView}
>
  <div
    aria-hidden="true"
    class="pointer-events-none fixed inset-0 overflow-hidden"
  >
    <span class="rainbow-band"></span>
  </div>

  <div class="relative mx-auto min-h-screen max-w-[96rem] p-2 sm:p-4">
    <div class="min-w-0">
      <header
        aria-label="Application header"
        class="flex min-h-16 items-center justify-between gap-3 px-2 py-2 sm:min-h-20 sm:px-3"
      >
        <div class="min-w-0">
          <p
            class="text-foreground truncate text-lg font-light tracking-[0.18em]"
          >
            animi
          </p>
          <p class="text-muted hidden truncate text-xs tracking-wide sm:block">
            Your quiet anime corner
          </p>
        </div>

        <AppNavigation {activeView} {selectView} />
      </header>

      <main id="main-content" tabindex="-1" class="min-w-0 px-1 pt-2 sm:px-3">
        <div hidden={activeView !== "home"}>
          <HomePage
            loadAiring={p5Services.loadAiring}
            advanceProgress={p5Services.advanceProgress}
            refreshIntervalMs={p5Services.refreshIntervalMs}
            openDiscovery={() => selectView("discovery")}
          />
        </div>
        {#if activeView === "discovery"}
          <DiscoveryPage
            loadTrending={p4Services.loadTrending}
            searchAnime={p4Services.searchAnime}
            loadDetails={p4Services.loadDetails}
            loadLibrary={p4Services.loadLibrary}
            saveEntry={p4Services.saveEntry}
            deleteEntry={p4Services.deleteEntry}
            loadRecommendations={p5Services.loadRecommendations}
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
        {:else if activeView === "settings"}
          <SettingsPage
            {themePreference}
            {themeStatus}
            {themeError}
            {selectThemePreference}
            {retryThemePreference}
            saveBackup={p6Services.saveBackup ?? exportBackup}
            chooseImport={p6Services.chooseImport ?? previewImport}
            restoreBackup={p6Services.restoreBackup ?? importBackup}
            clearData={p6Services.clearData ?? clearLocalData}
            localStateChanged={loadPreference}
          />
        {/if}
      </main>
    </div>
  </div>
</div>
