<script lang="ts">
  import { commandClient } from '$lib/commands/client';
  import { themeStore, connectionStore } from '$lib/commands/stores.svelte';
  import { onMount } from 'svelte';

  let testResults = $state<string[]>([]);
  let loading = $state(false);

  function addResult(message: string) {
    testResults = [...testResults, `${new Date().toLocaleTimeString()}: ${message}`];
  }

  async function testCommands() {
    loading = true;
    testResults = [];
    
    try {
      addResult('Testing theme commands...');
      
      // Test get all themes
      const themesResponse = await commandClient.getAllThemes();
      addResult(`Get all themes: ${themesResponse.success ? 'SUCCESS' : 'FAILED'} - ${themesResponse.data?.length || 0} themes`);
      
      // Test get active theme
      const activeThemeResponse = await commandClient.getActiveTheme();
      addResult(`Get active theme: ${activeThemeResponse.success ? 'SUCCESS' : 'FAILED'} - ${activeThemeResponse.data?.name || 'None'}`);
      
      addResult('Testing connection commands...');
      
      // Test list connections
      const connectionsResponse = await commandClient.listConnections();
      addResult(`List connections: ${connectionsResponse.success ? 'SUCCESS' : 'FAILED'} - ${connectionsResponse.data?.length || 0} connections`);
      
      addResult('Testing stores...');
      
      // Test stores
      await themeStore.refresh();
      addResult(`Theme store refresh: SUCCESS - ${themeStore.state.themes.length} themes loaded`);
      
      await connectionStore.refresh();
      addResult(`Connection store refresh: SUCCESS - ${connectionStore.state.connections.length} connections loaded`);
      
      addResult('All tests completed successfully!');
      
    } catch (error) {
      addResult(`ERROR: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    addResult('Test page loaded - ready to test command interface');
  });
</script>

<div class="p-6 max-w-4xl mx-auto">
  <h1 class="text-2xl font-bold mb-6">Command Interface Test</h1>
  
  <div class="space-y-4">
    <button 
      onclick={testCommands} 
      disabled={loading}
      class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
    >
      {loading ? 'Testing...' : 'Run Command Tests'}
    </button>
    
    <div class="border rounded-lg p-4 bg-gray-50 dark:bg-gray-800">
      <h2 class="font-semibold mb-2">Test Results:</h2>
      <div class="space-y-1 font-mono text-sm">
        {#each testResults as result}
          <div class={result.includes('SUCCESS') ? 'text-green-600' : result.includes('ERROR') || result.includes('FAILED') ? 'text-red-600' : 'text-gray-600'}>
            {result}
          </div>
        {/each}
      </div>
    </div>
    
    <div class="border rounded-lg p-4">
      <h2 class="font-semibold mb-2">Store State:</h2>
      <div class="grid grid-cols-2 gap-4 text-sm">
        <div>
          <h3 class="font-medium">Themes</h3>
          <p>Total: {themeStore.state.themes.length}</p>
          <p>Active: {themeStore.state.active?.name || 'None'}</p>
          <p>Loading: {themeStore.state.loading ? 'Yes' : 'No'}</p>
        </div>
        <div>
          <h3 class="font-medium">Connections</h3>
          <p>Total: {connectionStore.state.connections.length}</p>
          <p>Favorites: {connectionStore.state.favorites.length}</p>
          <p>Loading: {connectionStore.state.loading ? 'Yes' : 'No'}</p>
        </div>
      </div>
    </div>
  </div>
</div>
