# Tailwind Theming Rules

## ❌ Forbidden in UI Code

* bg-[var(--theme-*)]
* text-[var(--theme-*)]
* border-[var(--theme-*)]
* bg-(--theme-*)
* text-(--theme-*)
* Any Tailwind arbitrary color that references theme variables

These are internal runtime variables and must never be used directly.

## ✅ Required Pattern

All UI colors must come from Tailwind semantic utilities:

* bg-background
* bg-surface
* bg-muted
* text-foreground
* text-foreground-muted
* border-border
* border-border-focus
* bg-primary / hover:bg-primary-hover

If a needed semantic token does not exist, it must be added to `@theme`.

## ✅ Allowed Exceptions

Arbitrary values are allowed ONLY when:

* The value is truly dynamic per instance (e.g. bg-(--row-bg))
* Integrating with editors or plugins (Monaco, Ace, etc.)
* The value is non-color (layout, calc, animation math)

## Design Principle

UI code expresses *intent*, not implementation. CSS variables are implementation details.
