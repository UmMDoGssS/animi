<script lang="ts">
  import PageHeader from "../PageHeader.svelte";
  import PagePlaceholder from "../PagePlaceholder.svelte";
</script>

<div
  id="discovery-panel"
  role="tabpanel"
  aria-labelledby="discovery-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <PageHeader
    eyebrow="Find a new favorite"
    title="Discover"
    description="Search, trending anime, and private recommendations will live together in one cover-focused space."
  />
  <PagePlaceholder
    checkpoint="Visual checkpoint three"
    title="Search and cover collections"
    description="Discovery cards and the secondary anime-details presentation follow after the Home layout is approved."
  />
</div>
