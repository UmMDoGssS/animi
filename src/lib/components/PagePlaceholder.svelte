<script lang="ts">
  interface Props {
    title: string;
    description: string;
    checkpoint: string;
  }

  let { title, description, checkpoint }: Props = $props();
</script>

<section
  aria-labelledby="page-placeholder-heading"
  class="glass-panel border-border grid gap-5 rounded-[2rem] border p-5 sm:grid-cols-[minmax(0,1fr)_12rem] sm:items-center sm:p-7"
>
  <div>
    <p class="text-accent text-xs font-medium tracking-[0.18em] uppercase">
      {checkpoint}
    </p>
    <h2
      id="page-placeholder-heading"
      class="text-foreground mt-2 text-xl font-light tracking-[0.03em]"
    >
      {title}
    </h2>
    <p class="text-muted mt-2 max-w-xl text-sm leading-6">{description}</p>
  </div>

  <div
    aria-hidden="true"
    class="border-border bg-surface-sunken grid grid-cols-[3.5rem_1fr] gap-3 rounded-2xl border p-3"
  >
    <div
      class="border-border bg-overlay aspect-[2/3] w-full rounded-xl border"
    ></div>
    <div class="flex min-w-0 flex-col justify-center gap-2">
      <span class="bg-overlay h-2.5 w-full rounded-full"></span>
      <span class="bg-overlay h-2.5 w-3/4 rounded-full"></span>
      <span class="bg-accent mt-1 h-6 w-16 rounded-lg opacity-40"></span>
    </div>
  </div>
</section>
