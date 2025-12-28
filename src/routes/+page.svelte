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
        alert(`Connection Successful!\nVersion: ${testResult.data.version}`);
      } else {
        alert(`Connection Failed:\n${testResult.error}`);
      }
    } catch (err) {
      console.error("Test failed:", err);
      alert(`Test failed: ${String(err)}`);
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
