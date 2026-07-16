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
  class="glass-panel border-border flex w-14 flex-col items-center rounded-[1.25rem] border p-1"
>
  <nav aria-label="Primary navigation" class="w-full">
    <div
      role="tablist"
      aria-label="Primary pages"
      aria-orientation="vertical"
      class="grid justify-items-center gap-1"
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
          class="group text-muted hover:bg-overlay hover:text-foreground aria-selected:bg-accent aria-selected:text-on-accent relative grid size-11 place-items-center rounded-[0.875rem] transition-colors"
          onclick={() => selectView(item.id)}
          onkeydown={(event) => handleKeydown(event, index)}
        >
          <NavigationIcon name={item.icon} />
          <span
            aria-hidden="true"
            class="glass-panel border-border text-foreground pointer-events-none absolute top-1/2 left-[calc(100%+0.75rem)] z-40 translate-x-1 -translate-y-1/2 rounded-lg border px-2.5 py-1.5 text-xs font-medium whitespace-nowrap opacity-0 transition group-hover:translate-x-0 group-hover:opacity-100 group-focus-visible:translate-x-0 group-focus-visible:opacity-100"
          >
            {item.label}
          </span>
        </button>
      {/each}
    </div>
  </nav>
</div>
