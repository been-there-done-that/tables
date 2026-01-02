import type * as monaco from 'monaco-editor';
import type { ThemeData } from '../theme/types';

export const MONACO_THEME_NAME = 'app-theme';

/**
 * Initializes the app-theme with default values.
 */
export function defineMonacoTheme(m: typeof monaco) {
    m.editor.defineTheme(MONACO_THEME_NAME, {
        base: 'vs-dark',
        inherit: true,
        rules: [],
        colors: {
            'editor.background': '#1e1e1e',
        }
    });
}

/**
 * Updates the app-theme dynamically based on application theme data.
 * This should be called whenever the global application theme changes.
 */
export function updateMonacoTheme(m: typeof monaco, data: ThemeData) {
    const bg = data.ui.background;
    const fg = data.ui.foreground;
    const accent = data.ui.accent;
    const border = data.ui.border;

    const colors: { [key: string]: string } = {
        'editor.background': bg.primary,
        'editor.foreground': fg.primary,
        'editor.lineHighlightBackground': bg.hover + '44',
        'editorCursor.foreground': accent.primary,
        'editorLineNumber.foreground': fg.tertiary,
        'editorLineNumber.activeForeground': fg.secondary,
        'editorGutter.background': bg.primary,             // Same as editor, border comes from decorations
        'editorLineNumber.dimmedForeground': fg.tertiary + '88',
        'editorIndentGuide.background': border.subtle,
        'editorIndentGuide.activeBackground': border.default,
        'editor.selectionBackground': accent.primary + '33',
        'editor.inactiveSelectionBackground': accent.primary + '11',
        'editorBracketMatch.background': accent.primary + '22',
        'editorBracketMatch.border': accent.primary + '66',
        'editorWidget.background': bg.secondary,
        'editorWidget.border': border.default,
        'editorSuggestWidget.background': bg.secondary,
        'editorSuggestWidget.border': border.default,
        'editorSuggestWidget.foreground': fg.primary,
        'editorSuggestWidget.highlightForeground': accent.primary,
        'editorSuggestWidget.selectedBackground': bg.active,
        'editorSuggestWidget.selectedForeground': fg.primary,
        'editorSuggestWidget.focusHighlightForeground': accent.primary,
        'list.activeSelectionBackground': bg.active,
        'list.activeSelectionForeground': fg.primary,
        'list.hoverBackground': bg.hover,
        'list.hoverForeground': fg.primary,
        'scrollbarSlider.background': fg.tertiary + '33',
        'scrollbarSlider.hoverBackground': fg.tertiary + '55',
        'scrollbarSlider.activeBackground': fg.tertiary + '77',
    };

    // Use specific editor overrides if they exist in theme data
    if (data.editor) {
        if (data.editor.background) colors['editor.background'] = data.editor.background;
        if (data.editor.foreground) colors['editor.foreground'] = data.editor.foreground;
        if (data.editor.cursor) colors['editorCursor.foreground'] = data.editor.cursor;
        if (data.editor.selection) colors['editor.selectionBackground'] = data.editor.selection;
    }

    const rules: monaco.editor.ITokenThemeRule[] = [];
    if (data.syntax) {
        if (data.syntax.keyword) rules.push({ token: 'keyword', foreground: data.syntax.keyword });
        if (data.syntax.string) rules.push({ token: 'string', foreground: data.syntax.string });
        if (data.syntax.number) rules.push({ token: 'number', foreground: data.syntax.number });
        if (data.syntax.comment) rules.push({ token: 'comment', foreground: data.syntax.comment });
        if (data.syntax.function) rules.push({ token: 'function', foreground: data.syntax.function });
        if (data.syntax.variable) rules.push({ token: 'variable', foreground: data.syntax.variable });
        if (data.syntax.operator) rules.push({ token: 'operator', foreground: data.syntax.operator });
        if (data.syntax.type) rules.push({ token: 'type', foreground: data.syntax.type });
    }

    // Heuristic for base theme (dark vs light)
    // We could do a lumen check, but usually apps are dark-first.
    // Let's check if the background is light.
    const base: monaco.editor.BuiltinTheme = 'vs-dark';

    m.editor.defineTheme(MONACO_THEME_NAME, {
        base,
        inherit: true,
        rules,
        colors
    });

    // Globally set the theme for all editors using this theme name
    m.editor.setTheme(MONACO_THEME_NAME);
}
