<script lang="ts">
  interface Props {
    title: string;
    description: string;
  }

  let { title, description }: Props = $props();
</script>

<section
  aria-labelledby="view-title"
  class="glass-panel border-border bg-glass rounded-3xl border p-6 shadow-sm sm:p-8"
>
  <p class="text-accent mb-2 text-sm font-semibold tracking-wide">
    LOCAL-FIRST · PRIVATE
  </p>
  <h1
    id="view-title"
    class="text-foreground text-3xl font-semibold tracking-tight"
  >
    {title}
  </h1>
  <p class="text-muted mt-3 max-w-2xl text-base leading-7">{description}</p>

  <div
    class="border-border bg-surface-raised mt-8 rounded-2xl border border-dashed p-6"
  >
    <p class="text-foreground font-medium">The shelf is ready.</p>
    <p class="text-muted mt-1 text-sm">
      Feature work begins after this engineering foundation is verified.
    </p>
  </div>
</section>
