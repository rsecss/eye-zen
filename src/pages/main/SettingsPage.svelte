<script lang="ts">
  import { onMount } from 'svelte';
  import { getDetectorCapabilities } from '$lib/commands';
  import BehaviorSection from './settings/BehaviorSection.svelte';
  import DisplaySection from './settings/DisplaySection.svelte';
  import HotkeySection from './settings/HotkeySection.svelte';
  import ScheduleSection from './settings/ScheduleSection.svelte';
  import TimerSection from './settings/TimerSection.svelte';
  import WhitelistSection from './settings/WhitelistSection.svelte';

  let capabilitiesLoaded = $state(false);
  let afkSupported = $state(false);
  let processSupported = $state(false);
  let fullscreenSupported = $state(false);

  onMount(() => {
    getDetectorCapabilities()
      .then((capabilities) => {
        afkSupported = capabilities.afk_detection_supported;
        processSupported = capabilities.foreground_process_detection_supported;
        fullscreenSupported = capabilities.fullscreen_detection_supported;
        capabilitiesLoaded = true;
      })
      .catch((err) => {
        console.error('Failed to load detector capabilities:', err);
        afkSupported = false;
        processSupported = false;
        fullscreenSupported = false;
        capabilitiesLoaded = true;
      });
  });
</script>

<div class="settings-page">
  <TimerSection />
  <HotkeySection />
  <BehaviorSection {capabilitiesLoaded} {afkSupported} {fullscreenSupported} />
  <WhitelistSection {capabilitiesLoaded} {processSupported} />
  <ScheduleSection />
  <DisplaySection />
</div>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
</style>
