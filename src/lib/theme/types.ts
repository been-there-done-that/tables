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
  syntax?: {
    keyword?: string;
    string?: string;
    number?: string;
    comment?: string;
    function?: string;
    variable?: string;
    operator?: string;
    type?: string;
  };
  editor?: {
    background?: string;
    foreground?: string;
    selection?: string;
    cursor?: string;
    bracket?: string;
  };
};

export type ThemeRecord = {
  id: string;
  name: string;
  author?: string;
  description?: string;
  theme_data: string;
  is_builtin: boolean;
  is_active: boolean;
};
