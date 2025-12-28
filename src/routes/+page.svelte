<script lang="ts">
  import {
    IconDatabase,
    IconPlus,
    IconRefresh,
    IconPlugConnected,
    IconTrash,
  } from "@tabler/icons-svelte";
  import { onMount } from "svelte";
  import {
    listConnections,
    testConnectionById,
    deleteConnection,
  } from "$lib/commands/client";
  import type { Connection } from "$lib/commands/types";
  import Button from "$lib/components/Button.svelte";
  import ConnectionModal from "$lib/components/datasource/ConnectionModal.svelte";
  import ConfirmationModal from "$lib/components/ConfirmationModal.svelte";

  let connections = $state<Connection[]>([]);
  let isLoading = $state(true);
  let showModal = $state(false);
  let testingId = $state<string | null>(null);

  // Delete confirmation state
  let showDeleteConfirm = $state(false);
  let connectionToDelete = $state<string | null>(null);
  let isDeleting = $state(false);

  // Test result dialog state
  let showTestResult = $state(false);
  let testResultMessage = $state('');
  let testResultType = $state<'success' | 'error'>('success');

  async function loadConnections() {
    isLoading = true;
    try {
      const result = await listConnections();
      if (result.success) {
        connections = result.data || [];
      } else {
        console.error("Failed to load connections:", result.error);
      }
    } catch (err) {
      console.error("Error loading connections:", err);
    } finally {
      isLoading = false;
    }
  }

  function confirmDelete(id: string) {
    connectionToDelete = id;
    showDeleteConfirm = true;
  }

  async function handleConfirmDelete() {
    if (!connectionToDelete) return;

    isDeleting = true;
    try {
      const result = await deleteConnection(connectionToDelete);
      if (result.success) {
        showDeleteConfirm = false;
        await loadConnections();
      } else {
        alert(`Failed to delete connection: ${result.error}`);
      }
    } catch (err) {
      console.error("Delete failed:", err);
      alert(`Delete failed: ${String(err)}`);
    } finally {
      isDeleting = false;
    }
  }

  function handleCancelDelete() {
    showDeleteConfirm = false;
    connectionToDelete = null;
  }

  async function handleTestStoredConnection(id: string) {
    if (testingId) return; // Prevent concurrent tests
    testingId = id;
    try {
      // 1. Test connection directly using ID (securely on backend)
      const testResult = await testConnectionById(id);

      if (testResult.success && testResult.data) {
        testResultMessage = 'Connection test successful!';
        testResultType = 'success';
        showTestResult = true;
      } else {
        testResultMessage = `Connection test failed: ${testResult.error}`;
        testResultType = 'error';
        showTestResult = true;
      }
    } catch (err) {
      console.error("Test failed:", err);
      testResultMessage = `Test failed: ${String(err)}`;
      testResultType = 'error';
      showTestResult = true;
    } finally {
      testingId = null;
    }
  }

  onMount(() => {
    loadConnections();
  });
</script>

<div
  class="h-full flex flex-col bg-[--theme-bg-primary] text-[--theme-fg-primary]"
>
  <!-- Header -->
  <div
    class="h-14 shrink-0 border-b border-[--theme-border-default] px-6 flex items-center justify-between"
  >
    <div class="flex items-center gap-3">
      <div
        class="p-1.5 rounded-lg bg-[--theme-accent-primary]/10 text-[--theme-accent-primary]"
      >
        <IconDatabase class="size-5" />
      </div>
      <h1 class="font-medium">Database Connections</h1>
    </div>

    <div class="flex items-center gap-3">
      <button
        onclick={loadConnections}
        class="p-2 text-[--theme-fg-tertiary] hover:text-[--theme-fg-primary] hover:bg-[--theme-bg-secondary] rounded-md transition-colors"
        title="Refresh List"
      >
        <IconRefresh class={`size-4 ${isLoading ? "animate-spin" : ""}`} />
      </button>
      <Button onClick={() => (showModal = true)}>
        <IconPlus class="size-4" />
        New Connection
      </Button>
    </div>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto p-6">
    {#if isLoading && connections.length === 0}
      <div
        class="flex items-center justify-center h-48 text-[--theme-fg-tertiary]"
      >
        <span class="flex items-center gap-2">
          <IconRefresh class="size-4 animate-spin" />
          Loading connections...
        </span>
      </div>
    {:else if connections.length === 0}
      <div
        class="flex flex-col items-center justify-center h-64 text-center border-2 border-dashed border-[--theme-border-default] rounded-xl bg-[--theme-bg-secondary]/10 m-4"
      >
        <div class="p-4 rounded-full bg-[--theme-bg-secondary] mb-4">
          <IconDatabase class="size-8 text-[--theme-fg-tertiary]" />
        </div>
        <h3 class="text-lg font-medium mb-1">No connections yet</h3>
        <p class="text-[--theme-fg-secondary] text-sm max-w-xs mb-6">
          Add your first database connection to start exploring your data.
        </p>
        <Button onClick={() => (showModal = true)}>
          <IconPlus class="size-4" />
          Add Connection
        </Button>
      </div>
    {:else}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each connections as conn}
          <div
            class="p-4 rounded-lg border border-[--theme-border-default] bg-[--theme-bg-secondary]/30 hover:bg-[--theme-bg-secondary]/50 transition-colors group flex flex-col h-full"
          >
            <div class="flex items-start justify-between mb-2">
              <div class="flex items-center gap-2">
                <div
                  class="size-8 rounded flex items-center justify-center bg-[--theme-bg-tertiary] text-[--theme-fg-secondary]"
                >
                  <IconDatabase class="size-5" />
                </div>
                <div>
                  <div class="font-medium text-sm text-[--theme-fg-primary]">
                    {conn.name}
                  </div>
                  <div class="text-xs text-[--theme-fg-tertiary] capitalize">
                    {conn.engine}
                  </div>
                </div>
              </div>

              {#if conn.is_favorite}
                <div class="size-1.5 rounded-full bg-amber-400"></div>
              {/if}
            </div>

            <div
              class="mt-4 pt-3 border-t border-[--theme-border-default] flex justify-between items-center text-xs text-[--theme-fg-tertiary]"
            >
              <span>{conn.host || "Local"}</span>
              <span>{conn.port || "-"}</span>
            </div>

            <div
              class="mt-3 pt-3 border-t border-[--theme-border-default] grow"
            >
              <div class="max-h-32 overflow-y-auto custom-scrollbar">
                <pre
                  class="text-[10px] leading-tight text-[--theme-fg-tertiary] p-2 bg-[--theme-bg-tertiary]/50 rounded">{JSON.stringify(
                    conn,
                    null,
                    2,
                  )}</pre>
              </div>
            </div>

            <div
              class="mt-4 pt-3 border-t border-[--theme-border-default] flex justify-between items-center"
            >
              <button
                onclick={() => confirmDelete(conn.id)}
                class="p-1.5 text-[--theme-fg-tertiary] hover:text-red-500 hover:bg-red-500/10 rounded-md transition-colors"
                title="Delete Connection"
              >
                <IconTrash class="size-4" />
              </button>

              <button
                onclick={() => handleTestStoredConnection(conn.id)}
                class="text-xs flex items-center gap-1.5 px-2.5 py-1.5 rounded-md bg-[--theme-bg-tertiary] hover:bg-[--theme-bg-hover] text-[--theme-fg-secondary] hover:text-[--theme-fg-primary] border border-[--theme-border-default] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                disabled={testingId === conn.id}
              >
                {#if testingId === conn.id}
                  <IconRefresh class="size-3.5 animate-spin" />
                  Testing...
                {:else}
                  <IconPlugConnected class="size-3.5" />
                  Test Connection
                {/if}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<ConnectionModal bind:open={showModal} />

<ConfirmationModal
  bind:open={showDeleteConfirm}
  title="Delete Connection"
  message="Are you sure you want to delete this connection? This action is irreversible and requires you to re-enter credentials if you want to add it back."
  confirmText="Delete Connection"
  variant="danger"
  isLoading={isDeleting}
  onConfirm={handleConfirmDelete}
  onCancel={handleCancelDelete}
/>

<!-- Test Result Modal -->
{#if showTestResult}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="bg-[--theme-bg-primary] rounded-lg p-6 max-w-md w-full mx-4 border border-[--theme-border-default]">
      <div class="flex items-center gap-3 mb-4">
        {#if testResultType === 'success'}
          <div class="w-8 h-8 rounded-full bg-green-500/20 flex items-center justify-center">
            <svg class="w-5 h-5 text-green-500" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
            </svg>
          </div>
          <h3 class="text-lg font-semibold text-[--theme-fg-primary]">Success</h3>
        {:else}
          <div class="w-8 h-8 rounded-full bg-red-500/20 flex items-center justify-center">
            <svg class="w-5 h-5 text-red-500" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
            </svg>
          </div>
          <h3 class="text-lg font-semibold text-[--theme-fg-primary]">Error</h3>
        {/if}
      </div>
      
      <p class="text-[--theme-fg-secondary] mb-6">{testResultMessage}</p>
      
      <div class="flex justify-end">
        <button
          onclick={() => showTestResult = false}
          class="px-4 py-2 bg-[--theme-bg-tertiary] hover:bg-[--theme-bg-hover] text-[--theme-fg-secondary] hover:text-[--theme-fg-primary] rounded-md border border-[--theme-border-default] transition-colors"
        >
          OK
        </button>
      </div>
    </div>
  </div>
{/if}
