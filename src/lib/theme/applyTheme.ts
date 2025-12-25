export type ThemeData = {
  ui: {
    background: {
      primary: string;
      secondary: string;
      tertiary: string;
      hover: string;
      active: string;
    };
    foreground: {
      primary: string;
      secondary: string;
      tertiary: string;
    };
    accent: {
      primary: string;
      hover: string;
      active: string;
    };
    border: {
      default: string;
      subtle: string;
      focus: string;
    };
  };
};

export function applyThemeStyles(themeJson: string): void {
  const data = JSON.parse(themeJson) as ThemeData;

  const root = document.documentElement;
  // Reset inline CSS variables before applying new ones
  root.style.cssText = "";

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
}
