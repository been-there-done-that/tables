<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import { updaterStore } from '$lib/stores/updater.svelte';
  import { onMount } from 'svelte';

  let currentVersion = $state('...');

  onMount(async () => {
    currentVersion = await getVersion();
  });
</script>

<div class="p-6 space-y-6">
  <div>
    <h2 class="text-sm font-semibold text-foreground mb-1">Updates</h2>
    <p class="text-xs text-muted-foreground">Manage application updates.</p>
  </div>

  <div class="space-y-4">
    <div class="flex items-center justify-between py-2 border-b border-border">
      <div>
        <p class="text-sm font-medium text-foreground">Current version</p>
        <p class="text-xs text-muted-foreground font-mono">v{currentVersion}</p>
      </div>
    </div>

    <div class="flex items-center justify-between">
      <div>
        <p class="text-sm font-medium text-foreground">Latest version</p>
        <p class="text-xs text-muted-foreground">
          {#if updaterStore.status === 'idle'}
            Up to date
          {:else if updaterStore.status === 'available'}
            v{updaterStore.pendingVersion} available — use the titlebar chip to install
          {:else if updaterStore.status === 'downloading'}
            Downloading… {updaterStore.downloadPercent}%
          {:else if updaterStore.status === 'ready'}
            v{updaterStore.pendingVersion} downloaded — click "Restart to install" in the toolbar
          {:else if updaterStore.status === 'error'}
            {updaterStore.errorMessage ?? 'Unknown error'}
          {/if}
        </p>
      </div>
      <button
        class="px-3 py-1.5 text-xs font-medium rounded-md border border-border hover:bg-muted transition-colors disabled:opacity-50"
        disabled={updaterStore.status === 'downloading'}
        onclick={() => updaterStore.checkForUpdateManual()}
      >
        Check for updates
      </button>
    </div>
  </div>
</div>
