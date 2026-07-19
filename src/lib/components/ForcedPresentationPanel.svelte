<script lang="ts">
  import type { CommandError } from "$lib/ipc/types";
  import type { PrimaryViewId } from "$lib/navigation";
  import type { DebugPresentationMode } from "$lib/state/debugPresentation";

  import AsyncStateNotice from "./AsyncStateNotice.svelte";

  type ForcedMode = Exclude<DebugPresentationMode, "off">;

  interface Props {
    view: Exclude<PrimaryViewId, "settings">;
    mode: ForcedMode;
    openSettings: () => void;
  }

  const viewLabels: Record<Exclude<PrimaryViewId, "settings">, string> = {
    home: "Home",
    discovery: "Discover",
    dashboard: "Dashboard",
  };

  const simulatedError: CommandError = {
    code: "DEBUG_FORCED_ERROR",
    message: "This is a simulated error from the Settings state debugger.",
    retryable: true,
  };

  let { view, mode, openSettings }: Props = $props();
  let replayKey = $state(0);
  let viewLabel = $derived(viewLabels[view]);

  function replay() {
    replayKey += 1;
  }
</script>

<div
  id={`${view}-panel`}
  role="tabpanel"
  aria-labelledby={`${view}-tab`}
  tabindex="0"
  data-debug-presentation={mode}
  class="grid min-w-0 gap-4 pb-8"
>
  <h1 class="sr-only">{viewLabel}</h1>

  <div
    class="glass-control control-rounded border-warning/60 bg-warning/10 flex flex-col gap-3 border p-4 sm:flex-row sm:items-center sm:justify-between"
  >
    <div class="min-w-0">
      <p
        class="text-warning text-[0.65rem] font-medium tracking-[0.14em] uppercase"
      >
        Debug override active
      </p>
      <p class="text-foreground mt-1 text-sm">
        {viewLabel} is forced into its
        <strong class="font-medium">{mode}</strong>
        presentation.
      </p>
    </div>
    <div class="flex shrink-0 flex-wrap gap-2">
      <button
        type="button"
        onclick={replay}
        class="glass-control glass-interactive control-rounded border-border min-h-10 border px-3 py-2 text-xs"
      >
        Replay animation
      </button>
      <button
        type="button"
        onclick={openSettings}
        class="glass-accent control-rounded min-h-10 px-3 py-2 text-xs font-medium"
      >
        Debug settings
      </button>
    </div>
  </div>

  {#key replayKey}
    <AsyncStateNotice
      status={mode}
      error={mode === "error" ? simulatedError : null}
      onRetry={mode === "error" ? replay : undefined}
      retryLabel="Replay error"
    />
  {/key}
</div>
