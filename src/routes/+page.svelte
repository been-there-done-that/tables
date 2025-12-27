<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let showDatasource = $state(false);

  const openSettingsWindow = async () => {
    try {
      await invoke("open_appearance_window");
    } catch (e) {
      console.error("Failed to open appearance window:", e);
    }
  };
</script>
