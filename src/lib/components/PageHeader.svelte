<script lang="ts">
  interface Props {
    eyebrow: string;
    title: string;
    description: string;
  }

  let { eyebrow, title, description }: Props = $props();
</script>

<header
  class="glass-panel panel-large border-border relative overflow-hidden border p-5 sm:p-8 lg:p-10"
>
  <div
    aria-hidden="true"
    class="bg-accent pointer-events-none absolute -top-16 -right-12 size-40 rounded-full opacity-[0.08] blur-3xl"
  ></div>
  <div class="relative max-w-3xl">
    <p
      class="text-accent mb-3 text-xs font-medium tracking-[0.2em] uppercase sm:text-sm"
    >
      {eyebrow}
    </p>
    <h1
      class="text-foreground text-3xl font-light tracking-[0.04em] sm:text-5xl"
    >
      {title}
    </h1>
    <p
      class="text-muted mt-3 max-w-2xl text-sm leading-6 sm:text-base sm:leading-7"
    >
      {description}
    </p>
  </div>
</header>
