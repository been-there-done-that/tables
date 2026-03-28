import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'svelte-sonner';

export type UpdaterStatus = 'idle' | 'available' | 'downloading' | 'ready' | 'error';

interface UpdateInfo {
  version: string;
  body?: string;
  date?: string;
}

interface DownloadProgress {
  downloaded: number;
  total: number | null;
}

let status = $state<UpdaterStatus>('idle');
let pendingVersion = $state<string | null>(null);
let downloadPercent = $state(0);
let errorMessage = $state<string | null>(null);

// Listen to download progress events from Rust
listen<DownloadProgress>('update://progress', (event) => {
  const { downloaded, total } = event.payload;
  if (total && total > 0) {
    downloadPercent = Math.round((downloaded / total) * 100);
  }
});

export const updaterStore = {
  get status() { return status; },
  get pendingVersion() { return pendingVersion; },
  get downloadPercent() { return downloadPercent; },
  get errorMessage() { return errorMessage; },

  async checkForUpdate() {
    try {
      const info = await invoke<UpdateInfo | null>('check_for_update');
      if (info) {
        pendingVersion = info.version;
        status = 'available';
        toast.info(`Tables ${info.version} is available`, {
          description: 'Click the chip in the toolbar to update.',
          duration: 6000,
        });
      } else {
        status = 'idle';
      }
    } catch (e) {
      // Silent fail on background launch check
      console.error('[updater] check failed:', e);
    }
  },

  async checkForUpdateManual() {
    try {
      const info = await invoke<UpdateInfo | null>('check_for_update');
      if (info) {
        pendingVersion = info.version;
        status = 'available';
        toast.info(`Tables ${info.version} is available`);
      } else {
        toast.success("You're up to date!");
      }
    } catch (e) {
      toast.error('Update check failed', { description: String(e) });
    }
  },

  async download() {
    status = 'downloading';
    downloadPercent = 0;
    try {
      await invoke('download_update');
      status = 'ready';
    } catch (e) {
      status = 'error';
      errorMessage = String(e);
      toast.error('Download failed', { description: String(e) });
    }
  },

  async install() {
    try {
      await invoke('install_update');
    } catch (e) {
      status = 'error';
      errorMessage = String(e);
      toast.error('Install failed', { description: String(e) });
    }
  },

  retry() {
    status = 'idle';
    errorMessage = null;
    pendingVersion = null;
    this.checkForUpdate();
  },
};
