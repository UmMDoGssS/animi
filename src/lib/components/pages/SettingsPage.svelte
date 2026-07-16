<script lang="ts">
  import type { CommandError, ThemePreference } from "$lib/ipc/types";

  import PageHeader from "../PageHeader.svelte";
  import ThemePreferenceControl from "../ThemePreferenceControl.svelte";

  interface Props {
    themePreference: ThemePreference;
    themeStatus: "loading" | "ready" | "saving" | "error";
    themeError: CommandError | null;
    selectThemePreference: (preference: ThemePreference) => Promise<void>;
  }

  let {
    themePreference,
    themeStatus,
    themeError,
    selectThemePreference,
  }: Props = $props();
</script>

<div
  id="settings-panel"
  role="tabpanel"
  aria-labelledby="settings-tab"
  tabindex="0"
  class="grid min-w-0 gap-4 pb-8 sm:gap-5"
>
  <PageHeader
    eyebrow="Make it yours"
    title="Settings"
    description="Appearance, local cache details, backups, and recovery controls stay together on this device."
  />
  <ThemePreferenceControl
    preference={themePreference}
    status={themeStatus}
    error={themeError}
    selectPreference={selectThemePreference}
  />
</div>
