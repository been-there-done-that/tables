import { commandClient } from './client';
import { listen } from "@tauri-apps/api/event";
import type {
  Theme,
  Connection,
  AwsProfile,
  PluginInfo,
  CommandResponse
} from './types';

/**
 * Configurable delays for smooth UX transitions
 * Adjust these values to control loading timing across the application
 */
export const DELAYS = {
  THEME_LOAD: 300,
  THEME_ACTIVE: 200,
  CONNECTION_LOAD: 250,
  AWS_LOAD: 200,
  PLUGIN_LOAD: 200,
  THEME_TRANSITION: 500,
} as const;

/**
 * Reactive stores for managing application state with command integration
 */

// Theme Store
class ThemeStore {
  themes = $state<Theme[]>([]);
  activeId = $state<string>('');
  loading = $state<boolean>(true);
  error = $state<string>('');

  get state() {
    return {
      themes: this.themes,
      activeId: this.activeId,
      loading: this.loading,
      error: this.error,
      active: this.themes.find((t: Theme) => t.id === this.activeId)
    };
  }

  async loadThemes() {
    this.loading = true;
    this.error = '';

    // Add artificial delay for smooth UX
    await new Promise(resolve => setTimeout(resolve, DELAYS.THEME_LOAD));

    const response = await commandClient.getAllThemes();
    if (response.success && response.data) {
      this.themes = response.data;
    } else {
      this.error = response.error || 'Failed to load themes';
    }
    this.loading = false;
  }

  async loadActiveTheme() {
    // Add artificial delay for smooth UX
    await new Promise(resolve => setTimeout(resolve, DELAYS.THEME_ACTIVE));

    const response = await commandClient.getActiveTheme();
    if (response.success && response.data) {
      this.activeId = response.data.id;
    }
  }

  async setActiveTheme(themeId: string) {
    if (themeId === this.activeId) return;

    const response = await commandClient.setActiveTheme(themeId);
    if (response.success) {
      this.activeId = themeId;
    } else {
      this.error = response.error || 'Failed to set active theme';
      throw new Error(this.error);
    }
  }

  async refresh() {
    await Promise.all([this.loadThemes(), this.loadActiveTheme()]);
  }

  init() {

    let unlisten: () => void;

    // Initial load is handled by refresh(), but we set up the listener here
    import("@tauri-apps/api/event").then(async ({ listen }) => {
      // Need to import applyTheme dynamically or ensure it's imported at top
      const { applyTheme } = await import("$lib/theme/applyTheme");

      unlisten = await listen<Theme>("current-theme", (event) => {
        console.log("[ThemeStore] Theme change event received:", event.payload);
        const theme = event.payload;

        // Update local state if needed
        if (this.activeId !== theme.id) {
          console.log("[ThemeStore] Syncing store with backend theme:", theme.id);
          this.activeId = theme.id;
          // If the theme is not in our list, we might need to reload themes, but usually it is.
        }

        applyTheme(theme);
      });
    });

    return () => {
      if (unlisten) unlisten();
    };
  }
}

// Connection Store
class ConnectionStore {
  connections = $state<Connection[]>([]);
  loading = $state<boolean>(false);
  error = $state<string>('');
  selectedId = $state<string>('');

  get state() {
    return {
      connections: this.connections,
      loading: this.loading,
      error: this.error,
      selected: this.connections.find((c: Connection) => c.id === this.selectedId),
      favorites: this.connections.filter((c: Connection) => c.is_favorite),
      selectedId: this.selectedId
    };
  }

  async loadConnections() {
    this.loading = true;
    this.error = '';

    // Add artificial delay for smooth UX
    await new Promise(resolve => setTimeout(resolve, DELAYS.CONNECTION_LOAD));

    const response = await commandClient.listConnections();
    if (response.success && response.data) {
      this.connections = response.data;
    } else {
      this.error = response.error || 'Failed to load connections';
    }
    this.loading = false;
  }

  async createConnection(connection: Omit<Connection, 'id' | 'created_at' | 'updated_at' | 'last_connected_at' | 'connection_count'>, credentials: any) {
    const response = await commandClient.createConnection({ connection, credentials });
    if (response.success) {
      await this.loadConnections(); // Refresh list
    } else {
      this.error = response.error || 'Failed to create connection';
      throw new Error(this.error);
    }
  }

  async updateConnection(id: string, updates: Partial<Connection>, credentials?: any) {
    const response = await commandClient.updateConnection({ id, connection: updates, credentials });
    if (response.success) {
      await this.loadConnections(); // Refresh list
    } else {
      this.error = response.error || 'Failed to update connection';
      throw new Error(this.error);
    }
  }

  async deleteConnection(id: string) {
    const response = await commandClient.deleteConnection(id);
    if (response.success) {
      await this.loadConnections(); // Refresh list
    } else {
      this.error = response.error || 'Failed to delete connection';
      throw new Error(this.error);
    }
  }

  async testConnection(connection: Connection, credentials?: any) {
    const response = await commandClient.testConnection(connection, credentials);
    if (response.success) {
      return response.data;
    } else {
      this.error = response.error || 'Connection test failed';
      throw new Error(this.error);
    }
  }

  async searchConnections(query: string) {
    const response = await commandClient.searchConnections({ query });
    if (response.success && response.data) {
      this.connections = response.data;
    } else {
      this.error = response.error || 'Search failed';
    }
  }

  selectConnection(id: string) {
    this.selectedId = id;
  }

  async refresh() {
    await this.loadConnections();
  }

  init() {

    let unlisten: () => void;
    listen<void>("connections-changed", () => {
      console.log("[ConnectionStore] Connections changed event received, reloading...");
      this.loadConnections();
    }).then(u => unlisten = u);

    return () => {
      if (unlisten) unlisten();
    };
  }
}

// AWS Profile Store
class AwsStore {
  profiles = $state<AwsProfile[]>([]);
  loading = $state<boolean>(false);
  error = $state<string>('');

  get state() {
    return {
      profiles: this.profiles,
      loading: this.loading,
      error: this.error,
      validProfiles: this.profiles.filter((p: AwsProfile) => p.is_valid)
    };
  }

  async loadProfiles() {
    this.loading = true;
    this.error = '';

    // Add artificial delay for smooth UX
    await new Promise(resolve => setTimeout(resolve, DELAYS.AWS_LOAD));

    const response = await commandClient.getAvailableAwsProfiles();
    if (response.success && response.data) {
      this.profiles = response.data;
    } else {
      this.error = response.error || 'Failed to load AWS profiles';
    }
    this.loading = false;
  }

  async testProfile(profile: AwsProfile) {
    const response = await commandClient.testAwsProfile(profile);
    if (response.success) {
      // Update profile validity in the list
      const index = this.profiles.findIndex(p => p.name === profile.name);
      if (index >= 0) {
        this.profiles[index] = { ...profile, is_valid: response.data || false };
      }
    } else {
      this.error = response.error || 'Profile test failed';
      throw new Error(this.error);
    }
  }

  async refresh() {
    await this.loadProfiles();
  }
}

// Plugin Store
class PluginStore {
  plugins = $state<PluginInfo[]>([]);
  loading = $state<boolean>(false);
  error = $state<string>('');

  get state() {
    return {
      plugins: this.plugins,
      loading: this.loading,
      error: this.error,
      enabledPlugins: this.plugins.filter((p: PluginInfo) => p.enabled),
      disabledPlugins: this.plugins.filter((p: PluginInfo) => !p.enabled)
    };
  }

  async loadPlugins() {
    this.loading = true;
    this.error = '';

    // Add artificial delay for smooth UX
    await new Promise(resolve => setTimeout(resolve, DELAYS.PLUGIN_LOAD));

    const response = await commandClient.getAvailablePlugins();
    if (response.success && response.data) {
      this.plugins = response.data;
    } else {
      this.error = response.error || 'Failed to load plugins';
    }
    this.loading = false;
  }

  async enablePlugin(name: string) {
    const response = await commandClient.enablePlugin(name);
    if (response.success) {
      await this.loadPlugins(); // Refresh list
    } else {
      this.error = response.error || 'Failed to enable plugin';
      throw new Error(this.error);
    }
  }

  async disablePlugin(name: string) {
    const response = await commandClient.disablePlugin(name);
    if (response.success) {
      await this.loadPlugins(); // Refresh list
    } else {
      this.error = response.error || 'Failed to disable plugin';
      throw new Error(this.error);
    }
  }

  async refresh() {
    await this.loadPlugins();
  }
}

// Create store instances
export const themeStore = new ThemeStore();
export const connectionStore = new ConnectionStore();
export const awsStore = new AwsStore();
export const pluginStore = new PluginStore();

// Initialize stores
export async function initializeStores() {
  await Promise.all([
    themeStore.refresh(),
    connectionStore.refresh(),
    awsStore.refresh(),
    pluginStore.refresh()
  ]);
}
