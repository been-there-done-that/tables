// src/lib/themes/types.ts

export interface Theme {
  id: string;
  name: string;
  author?: string;
  description?: string;
  theme_data: string; // JSON string containing full theme data
  is_builtin: boolean;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface ThemeData {
  id: string;
  name: string;
  author?: string;
  description?: string;
  version?: string;
  ui: {
    background: ColorScale;
    foreground: ColorScale;
    accent: AccentColors;
    border: BorderColors;
    scrollbar?: ScrollbarColors;
    selection?: SelectionColors;
    input?: InputColors;
  };
  syntax: SyntaxColors;
  editor?: EditorColors;
}

export interface ColorScale {
  primary: string;
  secondary: string;
  tertiary: string;
  hover: string;
  active: string;
  disabled?: string;
}

export interface AccentColors {
  primary: string;
  hover: string;
  active: string;
  subtle?: string;
}

export interface BorderColors {
  default: string;
  subtle: string;
  focus: string;
}

export interface SyntaxColors {
  keyword: string;
  string: string;
  number: string;
  comment: string;
  function: string;
  variable: string;
  operator: string;
  type: string;
  constant?: string;
  property?: string;
  tag?: string;
  attribute?: string;
}

export interface EditorColors {
  background?: string;
  foreground?: string;
  selection?: string;
  selectionMatch?: string;
  lineNumber?: string;
  lineNumberActive?: string;
  gutter?: string;
  cursor?: string;
  bracket?: string;
  bracketMismatch?: string;
}

export interface ScrollbarColors {
  track: string;
  thumb: string;
  thumbHover: string;
}

export interface InputColors {
  background: string;
  foreground: string;
  placeholder: string;
  border: string;
  borderFocus: string;
  backgroundFocus: string;
}

export interface SelectionColors {
  background: string;
  foreground?: string;
}

export interface ThemeExport {
  version: string;
  exported_at: string;
  themes: Array<{
    id: string;
    name: string;
    author?: string;
    description?: string;
    theme_data: string;
  }>;
}

export interface ThemeImport {
  version: string;
  exported_at: string;
  themes: Array<{
    id: string;
    name: string;
    author?: string;
    description?: string;
    theme_data: string;
  }>;
}

export interface ImportResult {
  imported: number;
  failed: number;
  total: number;
}