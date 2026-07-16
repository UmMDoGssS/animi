<script lang="ts">
  import { tick } from "svelte";

  import { navigationItems, type PrimaryViewId } from "$lib/navigation";

  import NavigationIcon from "./NavigationIcon.svelte";

  interface Props {
    activeView: PrimaryViewId;
    selectView: (view: PrimaryViewId) => void;
  }

  let { activeView, selectView }: Props = $props();
  let navigationButtons: HTMLButtonElement[] = [];

  async function moveTo(index: number) {
    const item = navigationItems[index];
    if (!item) {
      return;
    }

    selectView(item.id);
    await tick();
    navigationButtons[index]?.focus();
  }

  function handleKeydown(event: KeyboardEvent, index: number) {
    let nextIndex: number | null = null;

    if (event.key === "ArrowDown" || event.key === "ArrowRight") {
      nextIndex = (index + 1) % navigationItems.length;
    } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
      nextIndex = (index - 1 + navigationItems.length) % navigationItems.length;
    } else if (event.key === "Home") {
      nextIndex = 0;
    } else if (event.key === "End") {
      nextIndex = navigationItems.length - 1;
    }

    if (nextIndex === null) {
      return;
    }

    event.preventDefault();
    void moveTo(nextIndex);
  }
</script>

<div
  class="glass-panel border-border flex w-full flex-col items-center rounded-[1.75rem] border p-1.5 shadow-sm sm:p-2"
>
  <div
    aria-hidden="true"
    class="bg-accent text-on-accent grid size-10 place-items-center rounded-2xl text-base font-black shadow-sm sm:size-11"
  >
    A
  </div>

  <div class="bg-border my-2 h-px w-8 sm:my-3"></div>

  <nav aria-label="Primary navigation" class="w-full">
    <div
      role="tablist"
      aria-label="Primary pages"
      aria-orientation="vertical"
      class="grid gap-1"
    >
      {#each navigationItems as item, index (item.id)}
        <button
          bind:this={navigationButtons[index]}
          id={`${item.id}-tab`}
          type="button"
          role="tab"
          aria-label={item.label}
          aria-selected={activeView === item.id}
          aria-controls={`${item.id}-panel`}
          tabindex={activeView === item.id ? 0 : -1}
          title={item.label}
          class="text-muted hover:bg-overlay hover:text-foreground aria-selected:bg-accent aria-selected:text-on-accent flex min-h-12 w-full items-center justify-center gap-2 rounded-2xl px-2 py-2 text-xs font-semibold transition sm:min-h-14 sm:flex-col sm:gap-1 sm:px-1"
          onclick={() => selectView(item.id)}
          onkeydown={(event) => handleKeydown(event, index)}
        >
          <NavigationIcon name={item.icon} />
          <span class="hidden leading-none sm:block">{item.label}</span>
        </button>
      {/each}
    </div>
  </nav>

  <div class="bg-border my-2 h-px w-8 sm:my-3"></div>
  <div class="flex items-center gap-1.5" title="Stored only on this device">
    <span class="bg-success size-2 rounded-full"></span>
    <span class="text-muted hidden text-[0.625rem] font-semibold sm:inline">
      LOCAL
    </span>
  </div>
</div>
