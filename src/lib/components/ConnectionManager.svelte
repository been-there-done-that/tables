<script lang="ts">
  import { connectionStore } from '$lib/commands/stores.svelte';
  import type { Connection, CreateConnectionRequest } from '$lib/commands/types';
  import { onMount } from 'svelte';

  let showCreateForm = $state(false);
  let newConnectionName = $state('');
  let selectedEngine = $state('postgresql');

  onMount(() => {
    connectionStore.refresh();
  });

  async function handleCreateConnection() {
    if (!newConnectionName.trim()) return;

    try {
      const connectionData: Omit<Connection, 'id' | 'created_at' | 'updated_at' | 'last_connected_at' | 'connection_count'> = {
        name: newConnectionName,
        engine: selectedEngine,
        host: '',
        port: selectedEngine === 'postgresql' ? 5432 : selectedEngine === 'mysql' ? 3306 : undefined,
        database: '',
        username: '',
        auth_type: 'password',
        ssl_enabled: false,
        ssh_tunnel_enabled: false,
        connection_params: {},
        is_favorite: false,
        color_tag: ''
      };

      await connectionStore.createConnection(connectionData, {});
      newConnectionName = '';
      showCreateForm = false;
    } catch (error) {
      console.error('Failed to create connection:', error);
    }
  }

  async function handleDeleteConnection(id: string) {
    if (confirm('Are you sure you want to delete this connection?')) {
      try {
        await connectionStore.deleteConnection(id);
      } catch (error) {
        console.error('Failed to delete connection:', error);
      }
    }
  }

  async function handleTestConnection(connection: Connection) {
    try {
      const result = await connectionStore.testConnection(connection);
      alert(`Connection test: ${result?.connected ? 'Success' : 'Failed'}`);
    } catch (error) {
      console.error('Connection test failed:', error);
      alert('Connection test failed');
    }
  }
</script>

<div class="connection-manager">
  <div class="header">
    <h2>Connections</h2>
    <button onclick={() => showCreateForm = true}>Create Connection</button>
  </div>

  {#if connectionStore.state.loading}
    <p>Loading connections...</p>
  {:else if connectionStore.state.error}
    <p class="error">Error: {connectionStore.state.error}</p>
  {:else}
    <div class="connections-list">
      {#each connectionStore.state.connections as connection}
        <div class="connection-item">
          <div class="connection-info">
            <h3>{connection.name}</h3>
            <p>Engine: {connection.engine}</p>
            {#if connection.host}
              <p>Host: {connection.host}:{connection.port}</p>
            {/if}
            <p>Connections: {connection.connection_count}</p>
          </div>
          <div class="connection-actions">
            <button onclick={() => handleTestConnection(connection)}>Test</button>
            <button onclick={() => handleDeleteConnection(connection.id)}>Delete</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if showCreateForm}
    <div class="modal">
      <div class="modal-content">
        <h3>Create New Connection</h3>
        <form onsubmit|preventDefault={handleCreateConnection}>
          <div>
            <label for="name">Connection Name:</label>
            <input id="name" bind:value={newConnectionName} required />
          </div>
          <div>
            <label for="engine">Database Engine:</label>
            <select id="engine" bind:value={selectedEngine}>
              <option value="postgresql">PostgreSQL</option>
              <option value="mysql">MySQL</option>
              <option value="sqlite">SQLite</option>
              <option value="mongodb">MongoDB</option>
              <option value="redis">Redis</option>
            </select>
          </div>
          <div class="form-actions">
            <button type="submit">Create</button>
            <button type="button" onclick={() => showCreateForm = false}>Cancel</button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>

<style>
  .connection-manager {
    padding: 20px;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  .connections-list {
    display: grid;
    gap: 15px;
  }

  .connection-item {
    border: 1px solid #ccc;
    border-radius: 8px;
    padding: 15px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .connection-info h3 {
    margin: 0 0 10px 0;
  }

  .connection-info p {
    margin: 5px 0;
    font-size: 0.9em;
    color: #666;
  }

  .connection-actions {
    display: flex;
    gap: 10px;
  }

  .modal {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-content {
    background: white;
    padding: 30px;
    border-radius: 8px;
    width: 400px;
  }

  .modal-content h3 {
    margin-top: 0;
  }

  .modal-content div {
    margin-bottom: 15px;
  }

  .modal-content label {
    display: block;
    margin-bottom: 5px;
  }

  .modal-content input,
  .modal-content select {
    width: 100%;
    padding: 8px;
    border: 1px solid #ccc;
    border-radius: 4px;
  }

  .form-actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }

  .error {
    color: red;
  }

  button {
    padding: 8px 16px;
    border: 1px solid #ccc;
    border-radius: 4px;
    background: #f0f0f0;
    cursor: pointer;
  }

  button:hover {
    background: #e0e0e0;
  }
</style>
