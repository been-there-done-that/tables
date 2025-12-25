// src/lib/themes/registry.ts
import { invoke } from '@tauri-apps/api/core';
import type { Theme, ThemeExport, ThemeImport, ImportResult } from './types';
import { applyThemeToDOM, parseThemeData } from './generator';

class ThemeRegistry {
  // All available themes
  private _themes: Map<string, Theme> = new Map();

  // Currently active theme ID
  private _activeThemeId: string = '';

  // Event listeners for reactive updates
  private listeners: Array<() => void> = [];

  // Public getters (reactive in Svelte components)
  get themes(): Map<string, Theme> {
    return this._themes;
  }

  get activeThemeId(): string {
    return this._activeThemeId;
  }

  // Computed active theme object
  get activeTheme(): Theme | undefined {
    return this._themes.get(this._activeThemeId);
  }

  // Computed parsed active theme data
  get activeThemeData() {
    const theme = this.activeTheme;
    if (!theme) return null;

    try {
      return parseThemeData(theme.theme_data);
    } catch (error) {
      console.error('Failed to parse theme data:', error);
      return null;
    }
  }

  // All themes sorted by name
  get allThemes(): Theme[] {
    return Array.from(this._themes.values()).sort((a, b) => a.name.localeCompare(b.name));
  }

  // Built-in themes only
  get builtinThemes(): Theme[] {
    return this.allThemes.filter(theme => theme.is_builtin);
  }

  // User themes only
  get userThemes(): Theme[] {
    return this.allThemes.filter(theme => !theme.is_builtin);
  }

  // Subscribe to theme changes
  subscribe(callback: () => void) {
    this.listeners.push(callback);
    return () => {
      this.listeners = this.listeners.filter(l => l !== callback);
    };
  }

  // Notify listeners of changes
  private notifyListeners() {
    this.listeners.forEach(callback => callback());
  }

  /**
   * Initialize theme registry by loading from database
   */
  async init(): Promise<void> {
    try {
      const themes = await invoke<Theme[]>('get_all_themes');

      const themesMap = new Map<string, Theme>();
      let activeThemeId = '';

      themes.forEach(theme => {
        themesMap.set(theme.id, theme);

        if (theme.is_active) {
          activeThemeId = theme.id;
        }
      });

      this._themes = themesMap;
      this._activeThemeId = activeThemeId;
      this.notifyListeners();

      // Apply the active theme
      const themeData = this.activeThemeData;
      if (themeData) {
        applyThemeToDOM(themeData);
      }

      console.log(`Loaded ${themes.length} themes, active: ${activeThemeId}`);
    } catch (error) {
      console.error('Failed to initialize theme registry:', error);
      throw error;
    }
  }

  /**
   * Set active theme
   */
  async setTheme(themeId: string): Promise<void> {
    try {
      await invoke('set_active_theme', { themeId });
      this._activeThemeId = themeId;
      this.notifyListeners();

      // Apply the new theme
      const themeData = this.activeThemeData;
      if (themeData) {
        applyThemeToDOM(themeData);
      }
    } catch (error) {
      console.error('Failed to set active theme:', error);
      throw error;
    }
  }

  /**
   * Create a new custom theme
   */
  async createTheme(themeData: string): Promise<Theme> {
    try {
      const newTheme = await invoke<Theme>('create_theme', { themeData });

      // Add to local registry
      this._themes.set(newTheme.id, newTheme);
      this.notifyListeners();

      return newTheme;
    } catch (error) {
      console.error('Failed to create theme:', error);
      throw error;
    }
  }

  /**
   * Update an existing theme
   */
  async updateTheme(themeId: string, themeData: string): Promise<Theme> {
    try {
      const updatedTheme = await invoke<Theme>('update_theme', {
        themeId,
        themeData
      });

      // Update local registry
      this._themes.set(updatedTheme.id, updatedTheme);
      this.notifyListeners();

      // If this was the active theme, reapply it
      if (this._activeThemeId === themeId) {
        const themeData = this.activeThemeData;
        if (themeData) {
          applyThemeToDOM(themeData);
        }
      }

      return updatedTheme;
    } catch (error) {
      console.error('Failed to update theme:', error);
      throw error;
    }
  }

  /**
   * Delete a theme
   */
  async deleteTheme(themeId: string): Promise<void> {
    try {
      await invoke('delete_theme', { themeId });

      // Remove from local registry
      this._themes.delete(themeId);
      this.notifyListeners();

      // If we deleted the active theme, switch to monokai
      if (this._activeThemeId === themeId) {
        await this.setTheme('monokai');
      }
    } catch (error) {
      console.error('Failed to delete theme:', error);
      throw error;
    }
  }

  /**
   * Search themes by name or description
   */
  async searchThemes(query: string): Promise<Theme[]> {
    try {
      return await invoke<Theme[]>('search_themes', { query });
    } catch (error) {
      console.error('Failed to search themes:', error);
      throw error;
    }
  }

  /**
   * Export all user themes for device sync
   */
  async exportThemes(): Promise<ThemeExport> {
    try {
      return await invoke<ThemeExport>('export_themes');
    } catch (error) {
      console.error('Failed to export themes:', error);
      throw error;
    }
  }

  /**
   * Import themes from device sync
   */
  async importThemes(importData: ThemeImport): Promise<ImportResult> {
    try {
      const result = await invoke<Record<string, string>>('import_themes', { importData });

      // Reload themes from database to get updated data
      await this.init();

      // Convert result to structured format
      const imported = Object.values(result).filter(status => status === 'success').length;
      const failed = Object.values(result).filter(status => status !== 'success').length;
      const total = Object.keys(result).length;

      return { imported, failed, total };
    } catch (error) {
      console.error('Failed to import themes:', error);
      throw error;
    }
  }

  /**
   * Get a theme by ID
   */
  getTheme(themeId: string): Theme | undefined {
    return this._themes.get(themeId);
  }

  /**
   * Check if a theme is active
   */
  isActive(themeId: string): boolean {
    return this._activeThemeId === themeId;
  }
}

export const themeRegistry = new ThemeRegistry();