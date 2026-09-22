<script lang="ts">
    import { onMount, tick } from "svelte";

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
        DashboardInsightsRequest,
        RefreshLibraryMetadataRequest,
        MediaKind,
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
    import type { DebugPresentationMode } from "$lib/state/debugPresentation";
    import { applyThemePreference } from "$lib/theme/theme";

    import MediaKindSelector from "./MediaKindSelector.svelte";
    import AppNavigation from "./AppNavigation.svelte";
    import DashboardPage from "./pages/DashboardPage.svelte";
    import ForcedPresentationPanel from "./ForcedPresentationPanel.svelte";
    import DiscoveryPage from "./pages/DiscoveryPage.svelte";
    import HomePage from "./pages/HomePage.svelte";
    import ReadingHome from "./pages/ReadingHome.svelte";
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
        loadTrending?: (
            request: MediaPageRequest,
        ) => Promise<MediaPageResponse>;
        searchAnime?: (
            request: SearchMediaRequest,
        ) => Promise<MediaPageResponse>;
        loadDetails?: (
            request: MediaDetailsRequest,
        ) => Promise<MediaDetailsResponse>;
        loadLibrary?: () => Promise<LibraryItem[]>;
        saveEntry?: (
            request: UpsertLibraryEntryRequest,
        ) => Promise<LibraryEntry>;
        deleteEntry?: (
            request: RemoveLibraryEntryRequest,
        ) => Promise<RemoveLibraryEntryResponse>;
        loadInsights?: (
            request: DashboardInsightsRequest,
        ) => Promise<DashboardInsights>;
        refreshMetadata?: (
            request: RefreshLibraryMetadataRequest,
        ) => Promise<RefreshLibraryMetadataResponse>;
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
        loadReading?: () => Promise<LibraryItem[]>;
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

    let homeRevision = $state(0);
    let libraryRevision = $state(0);
    let mediaKind = $state<MediaKind>("ANIME");
    let activeView = $state<PrimaryViewId>("home");
    let themePreference = $state<ThemePreference>("SYSTEM");
    let themeStatus = $state<"loading" | "ready" | "saving" | "error">(
        "loading",
    );
    let themeError = $state<CommandError | null>(null);
    let failedThemeOperation = $state<FailedThemeOperation | null>(null);
    let debugPresentationMode = $state<DebugPresentationMode>("off");

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

    async function selectMediaKind(kind: MediaKind): Promise<void> {
        mediaKind = kind;
        // Discover recreates its search tools on kind changes; retain keyboard focus.
        await tick();
        document
            .querySelector<HTMLInputElement>(`input[data-media-kind="${kind}"]`)
            ?.focus();
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
    data-debug-presentation={debugPresentationMode}
>
    <div
        aria-hidden="true"
        class="pointer-events-none fixed inset-0 overflow-hidden"
    >
        <span class="rainbow-band"></span>
    </div>

    <div
        class="relative mx-auto min-h-screen max-w-[90rem] px-4 py-[18px] sm:px-6 lg:px-10"
    >
        <div class="min-w-0">
            <header
                aria-label="Application header"
                class="app-header glass-panel panel-large"
            >
                <div class="app-brand">
                    <img src="/favicon.png" alt="" width="38" height="38" />
                    <strong>animi</strong>
                </div>

                <AppNavigation {activeView} {selectView} />
            </header>

            <main id="main-content" tabindex="-1" class="min-w-0">
                {#if activeView !== "settings" && (activeView !== "discovery" || debugPresentationMode !== "off")}
                    <div class="scope-row">
                        <p class="text-foreground text-xl font-semibold">
                            {activeView === "home"
                                ? mediaKind === "MANGA"
                                    ? "Your reading library"
                                    : "Continue watching"
                                : activeView === "dashboard"
                                  ? "Activity"
                                  : "Discover"}
                        </p>
                        <MediaKindSelector
                            value={mediaKind}
                            onChange={selectMediaKind}
                            label={`${activeView} media type`}
                        />
                    </div>
                {/if}
                {#key mediaKind}
                    {#if mediaKind === "ANIME" && activeView === "home" && debugPresentationMode === "off"}
                        {#key homeRevision}
                            <div>
                                <HomePage
                                    onLibraryChanged={() =>
                                        (libraryRevision += 1)}
                                    loadAiring={p5Services.loadAiring}
                                    advanceProgress={p5Services.advanceProgress}
                                    loadDetails={p4Services.loadDetails}
                                    saveEntry={p4Services.saveEntry}
                                    deleteEntry={p4Services.deleteEntry}
                                    refreshIntervalMs={p5Services.refreshIntervalMs}
                                    openDiscovery={() =>
                                        selectView("discovery")}
                                />
                            </div>
                        {/key}
                    {/if}
                    {#if activeView === "home" && debugPresentationMode === "off"}
                        {#if mediaKind === "MANGA"}
                            {#key homeRevision}
                                <ReadingHome
                                    loadReading={p5Services.loadReading}
                                    advanceProgress={p5Services.advanceProgress}
                                    loadDetails={p4Services.loadDetails}
                                    saveEntry={p4Services.saveEntry}
                                    deleteEntry={p4Services.deleteEntry}
                                    onLibraryChanged={() =>
                                        (libraryRevision += 1)}
                                    openDiscovery={() =>
                                        selectView("discovery")}
                                />
                            {/key}
                        {/if}
                        {#key libraryRevision}
                            <DashboardPage
                                {mediaKind}
                                onLibraryChanged={() => (homeRevision += 1)}
                                libraryOnly
                                loadInsights={p4Services.loadInsights}
                                refreshMetadata={p4Services.refreshMetadata}
                                loadDetails={p4Services.loadDetails}
                                saveEntry={p4Services.saveEntry}
                                deleteEntry={p4Services.deleteEntry}
                                openDiscovery={() => selectView("discovery")}
                            />
                        {/key}
                    {/if}
                    {#if activeView === "settings"}
                        <SettingsPage
                            {themePreference}
                            {themeStatus}
                            {themeError}
                            {selectThemePreference}
                            {retryThemePreference}
                            {debugPresentationMode}
                            setDebugPresentationMode={(
                                mode: DebugPresentationMode,
                            ) => (debugPresentationMode = mode)}
                            saveBackup={p6Services.saveBackup ?? exportBackup}
                            chooseImport={p6Services.chooseImport ??
                                previewImport}
                            restoreBackup={p6Services.restoreBackup ??
                                importBackup}
                            clearData={p6Services.clearData ?? clearLocalData}
                            localStateChanged={loadPreference}
                        />
                    {:else if debugPresentationMode !== "off"}
                        <ForcedPresentationPanel
                            view={activeView}
                            mode={debugPresentationMode}
                            openSettings={() => selectView("settings")}
                        />
                    {:else if activeView === "discovery"}
                        <DiscoveryPage
                            {mediaKind}
                            onMediaKindChange={selectMediaKind}
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
                            hideLibrary
                            {mediaKind}
                            loadInsights={p4Services.loadInsights}
                            refreshMetadata={p4Services.refreshMetadata}
                            loadDetails={p4Services.loadDetails}
                            saveEntry={p4Services.saveEntry}
                            deleteEntry={p4Services.deleteEntry}
                            openDiscovery={() => selectView("discovery")}
                        />
                    {/if}
                {/key}
            </main>
        </div>
    </div>
</div>
