<!-- src/lib/components/ui/ThemeSwitcher.svelte -->
<script lang="ts">
  import { themeRegistry } from '$lib/themes/registry';
  import type { Theme } from '$lib/themes/types';

  let isOpen = $state(false);
  let searchQuery = $state('');
  let showBuiltIn = $state(true);
  let showCustom = $state(true);

  // Filter themes based on search and category
  const filteredThemes = $derived(() => {
    const query = searchQuery.toLowerCase();
    return themeRegistry.allThemes.filter(theme => {
      const matchesSearch =
        theme.name.toLowerCase().includes(query) ||
        theme.description?.toLowerCase().includes(query) ||
        theme.id.toLowerCase().includes(query);

      const matchesCategory =
        (showBuiltIn && theme.is_builtin) ||
        (showCustom && !theme.is_builtin);

      return matchesSearch && matchesCategory;
    });
  });

  async function handleThemeSelect(theme: Theme) {
    try {
      await themeRegistry.setTheme(theme.id);
      isOpen = false;
    } catch (error) {
      console.error('Failed to switch theme:', error);
      alert('Failed to switch theme. Please try again.');
    }
  }

  function handleClose() {
    isOpen = false;
  }

  // Close on escape key
  $effect(() => {
    if (isOpen) {
      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          isOpen = false;
        }
      };

      document.addEventListener('keydown', handleKeyDown);
      return () => document.removeEventListener('keydown', handleKeyDown);
    }
  });
</script>

<div class="relative">
  <!-- Theme button -->
  <button
    onclick={() => isOpen = !isOpen}
    class="flex items-center gap-2 px-3 py-2 rounded bg-bg-secondary hover:bg-bg-tertiary transition-colors border border-border-subtle"
  >
    <div
      class="w-4 h-4 rounded border border-border-subtle"
      style="background: var(--theme-accent-primary)"
    />
    <span class="text-sm font-medium">
      {themeRegistry.activeTheme?.name || 'Loading...'}
    </span>
    <svg class="w-4 h-4 transition-transform" class:rotate-180={isOpen} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  <!-- Dropdown -->
  {#if isOpen}
    <div class="absolute top-full left-0 mt-2 w-80 rounded-lg shadow-2xl overflow-hidden z-50 border border-border-subtle">
      <!-- Search -->
      <div class="p-3 border-b border-border-subtle bg-bg-primary">
        <input
          bind:value={searchQuery}
          placeholder="Search themes..."
          class="w-full px-3 py-2 rounded bg-bg-secondary border border-border text-fg-primary placeholder:text-fg-tertiary outline-none focus:border-accent-primary"
        />

        <!-- Filters -->
        <div class="flex gap-4 mt-2">
          <label class="flex items-center gap-2 text-sm text-fg-secondary">
            <input type="checkbox" bind:checked={showBuiltIn} />
            Built-in ({themeRegistry.builtinThemes.length})
          </label>
          <label class="flex items-center gap-2 text-sm text-fg-secondary">
            <input type="checkbox" bind:checked={showCustom} />
            Custom ({themeRegistry.userThemes.length})
          </label>
        </div>
      </div>

      <!-- Theme list -->
      <div class="max-h-96 overflow-y-auto bg-bg-primary">
        {#if filteredThemes().length === 0}
          <div class="p-4 text-center text-fg-tertiary">
            No themes found
          </div>
        {/if}

        {#each filteredThemes() as theme (theme.id)}
          <div
            class="flex items-center gap-3 px-3 py-2 hover:bg-bg-tertiary cursor-pointer transition-colors {themeRegistry.isActive(theme.id) ? 'bg-accent-primary bg-opacity-10 border-r-2 border-accent-primary' : ''}"
            onclick={() => handleThemeSelect(theme)}
          >
            <!-- Preview swatch -->
            <div
              class="w-8 h-8 rounded border border-border-subtle flex-shrink-0"
              style="background: {(() => {
                try {
                  const themeData = JSON.parse(theme.theme_data);
                  return themeData.ui?.accent?.primary || '#666';
                } catch {
                  return '#666';
                }
              })()}"
            />

            <!-- Theme info -->
            <div class="flex-1 min-w-0">
              <div class="font-medium text-fg-primary truncate flex items-center gap-2">
                {theme.name}
                {#if theme.is_builtin}
                  <span class="text-xs px-1.5 py-0.5 rounded bg-accent-primary bg-opacity-20 text-accent-primary">
                    Built-in
                  </span>
                {/if}
                {#if themeRegistry.isActive(theme.id)}
                  <span class="text-xs px-1.5 py-0.5 rounded bg-accent-primary text-white">
                    Active
                  </span>
                {/if}
              </div>
              <div class="text-sm text-fg-tertiary truncate">
                {theme.description || theme.author || 'No description'}
              </div>
            </div>
          </div>
        {/each}
      </div>

      <!-- Footer -->
      <div class="p-3 border-t border-border-subtle bg-bg-secondary flex gap-2">
        <button
          onclick={() => { /* TODO: Open theme editor */ }}
          class="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded bg-bg-primary hover:bg-bg-tertiary transition-colors text-sm"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          Create custom theme
        </button>
      </div>
    </div>
  {/if}

  <!-- Click outside to close -->
  {#if isOpen}
    <div class="fixed inset-0 z-40" onclick={handleClose} />
  {/if}
</div>