<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import ThemeSwitcher from '../lib/components/ui/ThemeSwitcher.svelte';
  import { themeRegistry } from '../lib/themes/registry';

  let name = $state("");
  let greetMsg = $state("");

  async function greet(event: Event) {
    event.preventDefault();
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsg = await invoke("greet", { name });
  }
</script>

<main class="container mx-auto p-8 max-w-4xl">
  <header class="flex items-center justify-between mb-8">
    <h1 class="text-3xl font-bold">Tables IDE</h1>
    <ThemeSwitcher />
  </header>

  <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
    <!-- Theme demo section -->
    <section class="bg-bg-secondary p-6 rounded-lg border border-border-subtle">
      <h2 class="text-xl font-semibold mb-4 text-accent-primary">Theme System Demo</h2>

      <div class="space-y-4">
        <div class="flex gap-4">
          <button class="px-4 py-2 bg-accent-primary text-white rounded hover:bg-accent-hover transition-colors">
            Primary Button
          </button>
          <button class="px-4 py-2 bg-bg-tertiary text-fg-primary rounded hover:bg-bg-active transition-colors">
            Secondary Button
          </button>
        </div>

        <input
          type="text"
          placeholder="Sample input field"
          class="w-full px-3 py-2 border border-input-border rounded focus:border-input-border-focus focus:bg-input-bg-focus transition-colors"
        />

        <div class="bg-bg-tertiary p-3 rounded border border-border-subtle">
          <p class="text-fg-secondary mb-2">Sample text with different colors:</p>
          <p>
            <span class="text-fg-primary">Primary text</span> •
            <span class="text-fg-secondary">Secondary text</span> •
            <span class="text-fg-tertiary">Tertiary text</span>
          </p>
        </div>

        <div class="space-y-1">
          <div class="text-syntax-keyword">function keyword()</div>
          <div class="text-syntax-string">"string literal"</div>
          <div class="text-syntax-number">42</div>
          <div class="text-syntax-comment">// This is a comment</div>
          <div class="text-syntax-function">functionName()</div>
          <div class="text-syntax-variable">variableName</div>
        </div>
      </div>
    </section>

    <!-- Original demo section -->
    <section class="bg-bg-secondary p-6 rounded-lg border border-border-subtle">
      <h2 class="text-xl font-semibold mb-4">Tauri Demo</h2>

      <div class="flex gap-4 mb-4">
        <a href="https://vite.dev" target="_blank" class="hover:opacity-75 transition-opacity">
          <img src="/vite.svg" class="w-12 h-12" alt="Vite Logo" />
        </a>
        <a href="https://tauri.app" target="_blank" class="hover:opacity-75 transition-opacity">
          <img src="/tauri.svg" class="w-12 h-12" alt="Tauri Logo" />
        </a>
        <a href="https://svelte.dev" target="_blank" class="hover:opacity-75 transition-opacity">
          <img src="/svelte.svg" class="w-12 h-12" alt="SvelteKit Logo" />
        </a>
      </div>

      <p class="text-fg-secondary mb-4">Click on the logos to learn more about the technologies.</p>

      <form onsubmit={greet} class="space-y-4">
        <input
          id="greet-input"
          placeholder="Enter a name..."
          bind:value={name}
          class="w-full px-3 py-2 border border-input-border rounded focus:border-input-border-focus focus:bg-input-bg-focus transition-colors"
        />
        <button
          type="submit"
          class="px-4 py-2 bg-accent-primary text-white rounded hover:bg-accent-hover transition-colors"
        >
          Greet
        </button>
      </form>

      {#if greetMsg}
        <p class="mt-4 p-3 bg-accent-primary bg-opacity-10 border border-accent-primary rounded text-fg-primary">
          {greetMsg}
        </p>
      {/if}
    </section>
  </div>

  <!-- Theme info -->
  <section class="mt-8 p-4 bg-bg-tertiary rounded border border-border-subtle">
    <h3 class="font-semibold mb-2">Active Theme Info</h3>
    <div class="text-sm text-fg-secondary">
      <p><strong>Theme:</strong> {themeRegistry.activeTheme?.name || 'Loading...'}</p>
      <p><strong>Author:</strong> {themeRegistry.activeTheme?.author || 'Unknown'}</p>
      <p><strong>Built-in:</strong> {themeRegistry.activeTheme?.is_builtin ? 'Yes' : 'No'}</p>
      <p><strong>Total themes:</strong> {themeRegistry.allThemes.length}</p>
    </div>
  </section>
</main>
