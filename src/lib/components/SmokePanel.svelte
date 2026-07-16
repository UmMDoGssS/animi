<script lang="ts">
  import { checkBackend, normalizeCommandError } from "$lib/ipc/client";
  import type { CommandError, SmokeResponse } from "$lib/ipc/types";

  interface Props {
    runCheck?: () => Promise<SmokeResponse>;
  }

  let { runCheck = checkBackend }: Props = $props();

  let status = $state<"idle" | "checking" | "ready" | "error">("idle");
  let response = $state<SmokeResponse | null>(null);
  let error = $state<CommandError | null>(null);

  async function verifyBackend() {
    status = "checking";
    response = null;
    error = null;

    try {
      response = await runCheck();
      status = "ready";
    } catch (reason) {
      error = normalizeCommandError(reason);
      status = "error";
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
      {#if status === "idle"}
        <p class="text-muted mt-1 text-sm">
          Verify the typed frontend-to-Rust boundary.
        </p>
      {:else if status === "checking"}
        <p class="text-muted mt-1 text-sm" role="status">
          Checking the local core…
        </p>
      {:else if status === "ready" && response}
        <p class="text-success mt-1 text-sm" role="status">
          {response.message} · v{response.version}
        </p>
      {:else if status === "error" && error}
        <p class="text-danger mt-1 text-sm" role="alert">
          {error.message} ({error.code})
        </p>
      {/if}
    </div>

    <button
      type="button"
      class="bg-accent text-surface-raised rounded-xl px-4 py-2 text-sm font-semibold transition hover:opacity-90 disabled:cursor-wait disabled:opacity-60"
      disabled={status === "checking"}
      onclick={verifyBackend}
    >
      {status === "checking" ? "Checking…" : "Check core"}
    </button>
  </div>
</aside>
