<script lang="ts">
  import { updaterStore } from '$lib/stores/updater.svelte';
  import ConfirmationModal from '$lib/components/ConfirmationModal.svelte';

  let showConfirm = $state(false);

  function handleChipClick() {
    const s = updaterStore.status;
    if (s === 'available') {
      showConfirm = true;
    } else if (s === 'ready') {
      updaterStore.install();
    } else if (s === 'error') {
      updaterStore.retry();
    }
  }

  async function confirmDownload() {
    showConfirm = false;
    await updaterStore.download();
  }
</script>

{#if updaterStore.status !== 'idle'}
  <button
    class="flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-medium transition-opacity select-none"
    style={
      updaterStore.status === 'ready'
        ? 'background: var(--chip-result-bg); border-color: var(--chip-result-border); color: var(--chip-result-color);'
        : updaterStore.status === 'error'
        ? 'background: rgba(239,68,68,0.15); border-color: rgba(239,68,68,0.4); color: rgb(248,113,113);'
        : 'background: color-mix(in srgb, var(--theme-accent-primary) 15%, transparent); border-color: color-mix(in srgb, var(--theme-accent-primary) 40%, transparent); color: var(--theme-accent-primary);'
    }
    onclick={handleChipClick}
  >
    {#if updaterStore.status === 'downloading'}
      <span class="inline-block size-[7px] rounded-full border border-current border-t-transparent animate-spin"></span>
      Downloading… {updaterStore.downloadPercent}%
    {:else}
      <span class="inline-block size-[6px] rounded-full bg-current flex-shrink-0"></span>
      {#if updaterStore.status === 'available'}
        v{updaterStore.pendingVersion} available
      {:else if updaterStore.status === 'ready'}
        Restart to install
      {:else}
        Update failed
      {/if}
    {/if}
  </button>
{/if}

{#if showConfirm}
  <ConfirmationModal
    open={showConfirm}
    title="Update Available"
    message="v{updaterStore.pendingVersion} is ready to download. The app will restart to apply the update."
    confirmText="Download & Install"
    cancelText="Later"
    variant="info"
    onConfirm={confirmDownload}
    onCancel={() => (showConfirm = false)}
  />
{/if}
