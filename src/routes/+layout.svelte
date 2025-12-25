<script lang="ts">
    let { children } = $props();
    import "../app.css";
    

    import { listen } from "@tauri-apps/api/event";
    import { applyTheme } from "$lib/theme/applyTheme";
    import type { ThemeRecord } from "$lib/theme/types";

//     listen<ThemeRecord>("current-theme", (event) => {
//     const theme = event.payload;
//     activeId = theme.id;
//     applyTheme(theme);
//     const idx = themes.findIndex((t) => t.id === theme.id);
//     if (idx >= 0) {
//       themes[idx] = theme;
//     }
//   });

</script>

{@render children()}
