/** Tailwind theming guardrail: blocks direct --theme-* color usage */
module.exports = {
  plugins: ["regexp"],
  rules: {
    /**
     * ❌ Disallow theme variables in Tailwind classes
     */
    "regexp/no-unused-capturing-group": "off",
    "regexp/no-misleading-capturing-group": "off",
    "regexp/require-unicode-regexp": "off",
    "regexp/prefer-character-class": "off",
    "regexp/no-super-linear-backtracking": "off",
    "regexp/prefer-lookaround": "off",

    /**
     * Core rule
     */
    "regexp/forbid": [
      "error",
      [
        {
          // bg-[var(--theme-*)]
          pattern: "bg-\\[var\\(--theme-[^\\]]+\\)\\]",
          message:
            "Do not use --theme-* variables directly. Use semantic Tailwind utilities (bg-background, bg-muted, etc).",
        },
        {
          // bg-(--theme-*)
          pattern: "bg-\\(--theme-[^)]+\\)",
          message:
            "Do not use canonical arbitrary theme variables. Use semantic Tailwind utilities instead.",
        },
        {
          pattern: "text-\\[var\\(--theme-[^\\]]+\\)\\]",
          message:
            "Text colors must use semantic Tailwind tokens from @theme.",
        },
        {
          pattern: "text-\\(--theme-[^)]+\\)",
          message:
            "Text colors must use semantic Tailwind tokens from @theme.",
        },
        {
          pattern: "border-\\[var\\(--theme-[^\\]]+\\)\\]",
          message:
            "Border colors must use semantic Tailwind tokens from @theme.",
        },
        {
          pattern: "border-\\(--theme-[^)]+\\)",
          message:
            "Border colors must use semantic Tailwind tokens from @theme.",
        },
      ],
    ],
  },
};
