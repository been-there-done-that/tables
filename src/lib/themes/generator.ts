// src/lib/themes/generator.ts
import type { ThemeData } from './types';

/**
 * Generate CSS custom properties from theme data
 */
export function generateCSSVariables(themeData: ThemeData): string {
  const vars: string[] = [];

  // UI Background colors
  vars.push(`--theme-bg-primary: ${themeData.ui.background.primary};`);
  vars.push(`--theme-bg-secondary: ${themeData.ui.background.secondary};`);
  vars.push(`--theme-bg-tertiary: ${themeData.ui.background.tertiary};`);
  vars.push(`--theme-bg-hover: ${themeData.ui.background.hover};`);
  vars.push(`--theme-bg-active: ${themeData.ui.background.active};`);
  vars.push(`--theme-bg-disabled: ${themeData.ui.background.disabled || themeData.ui.background.tertiary};`);

  // UI Foreground colors
  vars.push(`--theme-fg-primary: ${themeData.ui.foreground.primary};`);
  vars.push(`--theme-fg-secondary: ${themeData.ui.foreground.secondary};`);
  vars.push(`--theme-fg-tertiary: ${themeData.ui.foreground.tertiary};`);
  vars.push(`--theme-fg-disabled: ${themeData.ui.foreground.disabled || themeData.ui.foreground.tertiary};`);

  // Accent colors
  vars.push(`--theme-accent-primary: ${themeData.ui.accent.primary};`);
  vars.push(`--theme-accent-hover: ${themeData.ui.accent.hover};`);
  vars.push(`--theme-accent-active: ${themeData.ui.accent.active};`);
  vars.push(`--theme-accent-subtle: ${themeData.ui.accent.subtle || themeData.ui.accent.primary}aa;`);

  // Border colors
  vars.push(`--theme-border-default: ${themeData.ui.border.default};`);
  vars.push(`--theme-border-subtle: ${themeData.ui.border.subtle};`);
  vars.push(`--theme-border-focus: ${themeData.ui.border.focus};`);

  // Scrollbar colors
  if (themeData.ui.scrollbar) {
    vars.push(`--theme-scrollbar-track: ${themeData.ui.scrollbar.track};`);
    vars.push(`--theme-scrollbar-thumb: ${themeData.ui.scrollbar.thumb};`);
    vars.push(`--theme-scrollbar-thumb-hover: ${themeData.ui.scrollbar.thumbHover};`);
  }

  // Selection colors
  if (themeData.ui.selection) {
    vars.push(`--theme-selection-bg: ${themeData.ui.selection.background};`);
    vars.push(`--theme-selection-fg: ${themeData.ui.selection.foreground || 'inherit'};`);
  }

  // Input colors
  if (themeData.ui.input) {
    vars.push(`--theme-input-bg: ${themeData.ui.input.background};`);
    vars.push(`--theme-input-fg: ${themeData.ui.input.foreground};`);
    vars.push(`--theme-input-placeholder: ${themeData.ui.input.placeholder};`);
    vars.push(`--theme-input-border: ${themeData.ui.input.border};`);
    vars.push(`--theme-input-border-focus: ${themeData.ui.input.borderFocus};`);
    vars.push(`--theme-input-bg-focus: ${themeData.ui.input.backgroundFocus};`);
  }

  // Syntax highlighting colors
  vars.push(`--theme-syntax-keyword: ${themeData.syntax.keyword};`);
  vars.push(`--theme-syntax-string: ${themeData.syntax.string};`);
  vars.push(`--theme-syntax-number: ${themeData.syntax.number};`);
  vars.push(`--theme-syntax-comment: ${themeData.syntax.comment};`);
  vars.push(`--theme-syntax-function: ${themeData.syntax.function};`);
  vars.push(`--theme-syntax-variable: ${themeData.syntax.variable};`);
  vars.push(`--theme-syntax-operator: ${themeData.syntax.operator};`);
  vars.push(`--theme-syntax-type: ${themeData.syntax.type};`);
  vars.push(`--theme-syntax-constant: ${themeData.syntax.constant || themeData.syntax.number};`);
  vars.push(`--theme-syntax-property: ${themeData.syntax.property || themeData.syntax.variable};`);
  vars.push(`--theme-syntax-tag: ${themeData.syntax.tag || themeData.syntax.keyword};`);
  vars.push(`--theme-syntax-attribute: ${themeData.syntax.attribute || themeData.syntax.string};`);

  // Editor-specific colors
  if (themeData.editor) {
    vars.push(`--theme-editor-bg: ${themeData.editor.background || themeData.ui.background.primary};`);
    vars.push(`--theme-editor-fg: ${themeData.editor.foreground || themeData.ui.foreground.primary};`);
    vars.push(`--theme-editor-selection: ${themeData.editor.selection || themeData.ui.selection?.background};`);
    vars.push(`--theme-editor-selection-match: ${themeData.editor.selectionMatch || '#3E4451'};`);
    vars.push(`--theme-editor-line-number: ${themeData.editor.lineNumber || '#90908A'};`);
    vars.push(`--theme-editor-line-number-active: ${themeData.editor.lineNumberActive || '#F8F8F2'};`);
    vars.push(`--theme-editor-gutter: ${themeData.editor.gutter || themeData.ui.background.secondary};`);
    vars.push(`--theme-editor-cursor: ${themeData.editor.cursor || '#F8F8F2'};`);
    vars.push(`--theme-editor-bracket: ${themeData.editor.bracket || '#A6E22E'};`);
    vars.push(`--theme-editor-bracket-mismatch: ${themeData.editor.bracketMismatch || '#F92672'};`);
  }

  return vars.join('\n');
}

/**
 * Apply theme to DOM
 */
export function applyThemeToDOM(themeData: ThemeData): void {
  const cssVars = generateCSSVariables(themeData);
  const root = document.documentElement;

  // Remove old theme attribute
  root.removeAttribute('data-theme');

  // Apply CSS variables
  root.style.cssText = cssVars;

  // Set theme attribute for selectors
  root.setAttribute('data-theme', themeData.id);

  // Dispatch event for components listening
  root.dispatchEvent(new CustomEvent('themechange', { detail: themeData }));
}

/**
 * Parse theme data from JSON string
 */
export function parseThemeData(themeDataJson: string): ThemeData {
  return JSON.parse(themeDataJson);
}