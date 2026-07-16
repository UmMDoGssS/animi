<script lang="ts">
  import PageHeader from "../PageHeader.svelte";
  import PagePlaceholder from "../PagePlaceholder.svelte";
</script>

<div
  id="dashboard-panel"
  role="tabpanel"
  aria-labelledby="dashboard-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <PageHeader
    eyebrow="Your local story"
    title="Dashboard"
    description="Library totals, viewing activity, favorite tags, and every tracked anime will come together here."
  />
  <PagePlaceholder
    checkpoint="Visual checkpoint four"
    title="Stats and library history"
    description="Charts will use recorded watch activity only, with clear empty states for dates Animi could not observe."
  />
</div>
