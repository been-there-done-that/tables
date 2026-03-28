import type { ThemeData, ThemeRecord } from "./types";
import { preloadMonaco } from "../monaco/monaco-runtime";
import { updateMonacoTheme } from "../monaco/monaco-theme";


export function applyThemeStyles(themeJson: string): void {
  const data = JSON.parse(themeJson) as ThemeData;

  const root = document.documentElement;
  // Previously we used root.style.cssText = "" which caused transparency issues
  // Now we just overwrite the variables directly.


  const bg = data.ui.background;
  const fg = data.ui.foreground;
  const accent = data.ui.accent;
  const border = data.ui.border;


  root.style.setProperty("--theme-bg-primary", bg.primary);
  root.style.setProperty("--theme-bg-secondary", bg.secondary);
  root.style.setProperty("--theme-bg-tertiary", bg.tertiary);
  root.style.setProperty("--theme-bg-hover", bg.hover);
  root.style.setProperty("--theme-bg-active", bg.active);

  root.style.setProperty("--theme-fg-primary", fg.primary);
  root.style.setProperty("--theme-fg-secondary", fg.secondary);
  root.style.setProperty("--theme-fg-tertiary", fg.tertiary);

  root.style.setProperty("--theme-accent-primary", accent.primary);
  root.style.setProperty("--theme-accent-hover", accent.hover);
  root.style.setProperty("--theme-accent-active", accent.active);

  root.style.setProperty("--theme-border-default", border.default);
  root.style.setProperty("--theme-border-subtle", border.subtle);
  root.style.setProperty("--theme-border-focus", border.focus);

  // Auto-detect light vs dark from bg-primary lightness to set chip text colors
  const bgLightness = parseFloat(bg.primary.match(/(\d+(?:\.\d+)?)%/)?.[1] ?? "15");
  const isDark = bgLightness < 50;
  root.style.setProperty("--chip-file-color",   isDark ? "#93c5fd" : "#1d4ed8");
  root.style.setProperty("--chip-table-color",  isDark ? "#d8b4fe" : "#7c3aed");
  root.style.setProperty("--chip-result-color", isDark ? "#86efac" : "#15803d");
  root.style.setProperty("--chip-file-bg",      isDark ? "rgba(59,130,246,0.22)"  : "rgba(59,130,246,0.12)");
  root.style.setProperty("--chip-table-bg",     isDark ? "rgba(168,85,247,0.22)"  : "rgba(147,51,234,0.10)");
  root.style.setProperty("--chip-result-bg",    isDark ? "rgba(34,197,94,0.18)"   : "rgba(22,163,74,0.10)");
  root.style.setProperty("--chip-file-border",  isDark ? "rgba(59,130,246,0.45)"  : "rgba(59,130,246,0.35)");
  root.style.setProperty("--chip-table-border", isDark ? "rgba(168,85,247,0.45)"  : "rgba(147,51,234,0.35)");
  root.style.setProperty("--chip-result-border",isDark ? "rgba(34,197,94,0.4)"    : "rgba(22,163,74,0.35)");

  // Sync Monaco theme
  preloadMonaco().then((m) => updateMonacoTheme(m, data));
}



export function applyTheme(theme: ThemeRecord | undefined, useTransition = true) {
  if (!theme) return;
  const run = () => applyThemeStyles(theme.theme_data);

  if (useTransition && typeof document !== "undefined" && "startViewTransition" in document) {
    try {
      // Call as method to preserve Document context; swallow aborts
      (document as any)
        .startViewTransition(() => run())
        ?.finished?.catch(() => { });
      return;
    } catch (err) {
      console.warn("View transition failed, falling back:", err);
    }
  }

  run();
}