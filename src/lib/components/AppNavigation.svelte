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
  class="glass-panel control-group border-border flex w-14 flex-col items-center border p-1.5"
>
  <nav aria-label="Primary navigation" class="w-full">
    <div
      role="tablist"
      aria-label="Primary pages"
      aria-orientation="vertical"
      class="grid justify-items-center gap-2"
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
          class="glass-control glass-interactive glass-tab control-rounded group text-muted hover:text-foreground relative grid size-10 place-items-center transition-colors"
          onclick={() => selectView(item.id)}
          onkeydown={(event) => handleKeydown(event, index)}
        >
          <NavigationIcon name={item.icon} />
          <span
            aria-hidden="true"
            class="glass-control control-rounded border-border text-foreground pointer-events-none absolute top-1/2 left-[calc(100%+0.75rem)] z-40 translate-x-1 -translate-y-1/2 border px-2.5 py-1.5 text-xs font-medium whitespace-nowrap opacity-0 transition group-hover:translate-x-0 group-hover:opacity-100 group-focus-visible:translate-x-0 group-focus-visible:opacity-100"
          >
            {item.label}
          </span>
        </button>
      {/each}
    </div>
  </nav>
</div>
