import { getContext, setContext } from "svelte";
import type { ThemeRecord } from "$lib/theme/types";

export type ThemeState = {
  themes: ThemeRecord[];
  activeId: string;
  loading: boolean;
  error: string;
};

export type ThemeContext = {
  subscribe: (fn: (state: ThemeState) => void) => () => void;
  setActive: (id: string) => Promise<void>;
};

export const THEME_CONTEXT = Symbol("theme-context");

export function setThemeContext(ctx: ThemeContext) {
  setContext(THEME_CONTEXT, ctx);
}

export function getThemeContext(): ThemeContext {
  return getContext<ThemeContext>(THEME_CONTEXT);
}
