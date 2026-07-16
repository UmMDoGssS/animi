<script lang="ts">
  import { checkBackend, normalizeCommandError } from "$lib/ipc/client";
  import type { SmokeResponse } from "$lib/ipc/types";
  import type { AsyncState } from "$lib/state/asyncState";

  interface Props {
    runCheck?: () => Promise<SmokeResponse>;
  }

  let { runCheck = checkBackend }: Props = $props();

  let state = $state<AsyncState<SmokeResponse>>({ status: "idle" });

  async function verifyBackend() {
    state = { status: "loading" };

    try {
      state = { status: "ready", data: await runCheck() };
    } catch (reason) {
      state = { status: "error", error: normalizeCommandError(reason) };
    }
  }
</script>

<aside
  aria-labelledby="core-status-heading"
  class="border-border bg-surface-raised rounded-2xl border p-4"
>
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h2 id="core-status-heading" class="text-foreground font-semibold">
        Local core
      </h2>
      {#if state.status === "idle"}
        <p class="text-muted mt-1 text-sm">
          Verify the typed frontend-to-Rust boundary.
        </p>
      {:else if state.status === "loading"}
        <p class="text-muted mt-1 text-sm" role="status">
          Checking the local core…
        </p>
      {:else if state.status === "ready"}
        <p class="text-success mt-1 text-sm" role="status">
          {state.data.message} · v{state.data.version}
        </p>
      {:else if state.status === "error"}
        <p class="text-danger mt-1 text-sm" role="alert">
          {state.error.message} ({state.error.code})
        </p>
      {/if}
    </div>

    <button
      type="button"
      class="bg-accent text-on-accent rounded-xl px-4 py-2 text-sm font-semibold transition hover:opacity-90 disabled:cursor-wait disabled:opacity-60"
      disabled={state.status === "loading"}
      onclick={verifyBackend}
    >
      {state.status === "loading" ? "Checking…" : "Check core"}
    </button>
  </div>
</aside>
