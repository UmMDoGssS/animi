<script lang="ts">
  import { onMount, tick } from "svelte";

  interface GlassMultiSelectOption {
    value: string;
    label: string;
  }

  interface Props {
    id: string;
    label: string;
    defaultLabel: string;
    selectedValues: readonly string[];
    options: readonly GlassMultiSelectOption[];
    onChange: (values: string[]) => void;
  }

  let { id, label, defaultLabel, selectedValues, options, onChange }: Props =
    $props();

  let open = $state(false);
  let activeIndex = $state(0);
  let container = $state<HTMLDivElement | null>(null);
  let trigger = $state<HTMLButtonElement | null>(null);

  const listboxId = $derived(`${id}-listbox`);
  const activeOptionId = $derived(
    open && options[activeIndex] ? `${id}-option-${activeIndex}` : undefined,
  );
  const triggerText = $derived.by(() => {
    if (selectedValues.length === 0) return defaultLabel;
    if (selectedValues.length === 1) return selectedValues[0] ?? defaultLabel;
    return `${selectedValues.length} ${label}s`;
  });

  onMount(() => {
    function closeOnOutsidePointer(event: PointerEvent): void {
      if (
        open &&
        event.target instanceof Node &&
        container?.contains(event.target) === false
      ) {
        open = false;
      }
    }

    document.addEventListener("pointerdown", closeOnOutsidePointer);
    return () =>
      document.removeEventListener("pointerdown", closeOnOutsidePointer);
  });

  function openMenu(): void {
    const firstSelectedIndex = options.findIndex((option) =>
      selectedValues.includes(option.value),
    );
    activeIndex = firstSelectedIndex >= 0 ? firstSelectedIndex : 0;
    open = true;
  }

  function closeMenu(restoreFocus = false): void {
    open = false;
    if (restoreFocus) {
      void tick().then(() => trigger?.focus());
    }
  }

  function toggleMenu(): void {
    if (open) {
      closeMenu();
      return;
    }
    openMenu();
  }

  function moveActiveOption(offset: number): void {
    if (options.length === 0) return;
    activeIndex = (activeIndex + offset + options.length) % options.length;
  }

  function toggleOption(option: GlassMultiSelectOption): void {
    const nextSelection = selectedValues.includes(option.value)
      ? selectedValues.filter((value) => value !== option.value)
      : options
          .filter(
            (candidate) =>
              selectedValues.includes(candidate.value) ||
              candidate.value === option.value,
          )
          .map((candidate) => candidate.value);
    onChange(nextSelection);
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (!open) {
      if (
        event.key === "ArrowDown" ||
        event.key === "ArrowUp" ||
        event.key === "Enter" ||
        event.key === " "
      ) {
        event.preventDefault();
        openMenu();
      }
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveActiveOption(1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveActiveOption(-1);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      activeIndex = 0;
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      activeIndex = Math.max(0, options.length - 1);
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      const option = options[activeIndex];
      if (option) toggleOption(option);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeMenu(true);
      return;
    }
    if (event.key === "Tab") {
      closeMenu();
    }
  }
</script>

<div bind:this={container} class="relative min-w-0">
  <button
    bind:this={trigger}
    {id}
    type="button"
    role="combobox"
    aria-label={label}
    aria-haspopup="listbox"
    aria-controls={listboxId}
    aria-expanded={open}
    aria-activedescendant={activeOptionId}
    class="glass-select-trigger control-rounded border-border text-foreground flex min-h-10 w-full min-w-0 cursor-pointer items-center justify-between gap-3 border py-2 pr-3 pl-3 text-left text-sm transition-colors"
    onclick={toggleMenu}
    onkeydown={handleKeydown}
  >
    <span class="min-w-0 truncate">{triggerText}</span>
    <span class="flex shrink-0 items-center gap-2">
      {#if selectedValues.length > 0}
        <span
          aria-hidden="true"
          class="bg-accent text-on-accent grid size-5 place-items-center rounded-full text-[0.62rem] tabular-nums"
        >
          {selectedValues.length}
        </span>
      {/if}
      <svg
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="text-accent size-4 transition-transform"
        class:rotate-180={open}
      >
        <path d="m7 9.5 5 5 5-5" />
      </svg>
    </span>
  </button>

  {#if open}
    <div
      class="glass-select-menu control-rounded border-border absolute top-full right-0 left-0 z-50 mt-2 grid overflow-hidden border p-1.5"
    >
      <ul
        id={listboxId}
        role="listbox"
        aria-label={`${label} options`}
        aria-multiselectable="true"
        class="grid max-h-64 gap-1 overflow-y-auto"
      >
        {#each options as option, index (option.value)}
          <li role="presentation">
            <button
              id={`${id}-option-${index}`}
              type="button"
              role="option"
              aria-selected={selectedValues.includes(option.value)}
              tabindex="-1"
              data-active={activeIndex === index ? "true" : undefined}
              class="glass-select-option control-rounded text-foreground flex min-h-9 w-full items-center justify-between gap-3 px-2.5 py-2 text-left text-xs transition-colors"
              onmouseenter={() => (activeIndex = index)}
              onclick={() => toggleOption(option)}
            >
              <span>{option.label}</span>
              <span
                aria-hidden="true"
                class="border-border grid size-4 shrink-0 place-items-center rounded border text-[0.62rem]"
                class:bg-accent={selectedValues.includes(option.value)}
                class:text-on-accent={selectedValues.includes(option.value)}
              >
                {selectedValues.includes(option.value) ? "✓" : ""}
              </span>
            </button>
          </li>
        {/each}
      </ul>
      {#if selectedValues.length > 0}
        <button
          type="button"
          class="glass-interactive control-rounded text-muted border-border mt-1 min-h-9 border-t px-2.5 py-2 text-left text-xs transition-colors"
          onclick={() => onChange([])}
        >
          clear {label} selections
        </button>
      {/if}
    </div>
  {/if}
</div>
