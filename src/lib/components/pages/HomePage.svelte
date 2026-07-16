<script lang="ts">
  import PageHeader from "../PageHeader.svelte";
  import PagePlaceholder from "../PagePlaceholder.svelte";
</script>

<div
  id="home-panel"
  role="tabpanel"
  aria-labelledby="home-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <PageHeader
    eyebrow="Today at a glance"
    title="Home"
    description="Your currently airing shows, next episodes, and catch-up queue will meet you here."
  />
  <PagePlaceholder
    checkpoint="Next visual checkpoint"
    title="Airing carousel"
    description="The next review adds the cover-led carousel and its on-time, catch-up, unknown-schedule, and empty states."
  />
</div>
