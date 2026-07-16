<script lang="ts">
  import { navigationItems, type PrimaryViewId } from "$lib/navigation";

  import SmokePanel from "./SmokePanel.svelte";
  import ViewPlaceholder from "./ViewPlaceholder.svelte";

  let activeView = $state<PrimaryViewId>("library");
  let activeItem = $derived(
    navigationItems.find((item) => item.id === activeView) ??
      navigationItems[0],
  );
</script>

<div class="bg-surface text-foreground min-h-screen">
  <header class="border-border bg-surface-raised border-b">
    <div class="mx-auto flex max-w-7xl items-center gap-3 px-4 py-4 sm:px-6">
      <div
        aria-hidden="true"
        class="bg-accent text-surface-raised grid size-10 place-items-center rounded-2xl text-xl"
      >
        ⌁
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
              class="hover:bg-overlay aria-[current=page]:bg-accent aria-[current=page]:text-surface-raised w-full rounded-2xl px-3 py-3 text-left text-sm font-medium transition"
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
      <SmokePanel />
    </main>
  </div>
</div>
