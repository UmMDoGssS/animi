<script lang="ts">
  import { tick } from "svelte";

  import { navigationItems, type PrimaryViewId } from "$lib/navigation";

  import NavigationIcon from "./NavigationIcon.svelte";

  interface Props {
    activeView: PrimaryViewId;
    selectView: (view: PrimaryViewId) => void;
  }

  let { activeView, selectView }: Props = $props();
  let navigationButtons = $state<HTMLButtonElement[]>([]);

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

    if (event.key === "ArrowRight") {
      nextIndex = (index + 1) % navigationItems.length;
    } else if (event.key === "ArrowLeft") {
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
  class="glass-panel control-group border-border flex items-center border p-1.5"
>
  <nav aria-label="Primary navigation">
    <div
      role="tablist"
      aria-label="Primary pages"
      aria-orientation="horizontal"
      class="flex items-center gap-1"
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
          class="glass-control glass-interactive glass-tab control-rounded text-muted hover:text-foreground flex h-10 items-center justify-center gap-2 px-2.5 transition-colors sm:px-3"
          onclick={() => selectView(item.id)}
          onkeydown={(event) => handleKeydown(event, index)}
        >
          <NavigationIcon name={item.icon} />
          <span
            aria-hidden="true"
            class="relative top-0.5 hidden text-xs leading-none font-medium sm:inline"
          >
            {item.label}
          </span>
        </button>
      {/each}
    </div>
  </nav>
</div>
