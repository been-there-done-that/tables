<script lang="ts">
  import { cn } from "$lib/utils";
  import type { Snippet } from "svelte";

  type Variant = "solid" | "ghost" | "outline" | "subtle";
  type Radius = "none" | "sm" | "md" | "lg" | "full";
  type Height = "8" | "10" | "12";
  type ButtonType = "button" | "submit" | "reset";
  type ClickHandler = ((event: MouseEvent) => void) | null;

  interface Props {
    as?: string;
    type?: ButtonType;
    variant?: Variant;
    radius?: Radius;
    height?: Height;
    class?: string;
    disabled?: boolean;
    href?: string;
    children?: Snippet;
    onClick?: ClickHandler | undefined;
  }

  let {
    as: asTag = "button",
    type = "button" as ButtonType,
    variant = "solid" as Variant,
    radius = "sm" as Radius,
    height = "8" as Height,
    class: className = "",
    disabled = false,
    href = "",
    children,
    onClick = undefined,
  }: Props = $props();

  const radiusClass: Record<Radius, string> = {
    none: "rounded-none",
    sm: "rounded-sm",
    md: "rounded-md",
    lg: "rounded-lg",
    full: "rounded-full",
  };

  const heightClass: Record<Height, string> = {
    "8": "h-8 px-3 text-sm",
    "10": "h-10 px-4 text-sm",
    "12": "h-12 px-5 text-base",
  };

  const variantClass: Record<Variant, string> = {
    solid:
      "bg-(--theme-accent-primary) text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.45)] hover:bg-[color-mix(in_srgb,var(--theme-accent-primary)_78%,black_22%)] focus-visible:ring-offset-2 focus-visible:ring-offset-(--theme-bg-primary)",
    ghost:
      "bg-transparent text-(--theme-fg-primary) hover:bg-[color-mix(in_srgb,var(--theme-accent-primary)_15%,var(--theme-bg-secondary)_85%)]",
    outline:
      "border border-(--theme-border-default) text-(--theme-fg-primary) hover:border-(--theme-accent-primary) hover:bg-[color-mix(in_srgb,var(--theme-accent-primary)_12%,var(--theme-bg-secondary)_88%)]",
    subtle:
      "bg-[color-mix(in_srgb,var(--theme-bg-tertiary)_78%,transparent)] text-(--theme-fg-primary) hover:bg-[color-mix(in_srgb,var(--theme-bg-tertiary)_88%,transparent)]",
  };

</script>

{#if asTag === "button"}
  <button
    type={type}
    disabled={disabled}
    onclick={onClick ?? undefined}
    class={cn(
      "inline-flex items-center justify-center gap-2 font-medium transition focus:outline-none focus-visible:ring-2 focus-visible:ring-(--theme-accent-primary)",
      heightClass[height],
      radiusClass[radius],
      variantClass[variant],
      disabled && "opacity-50 cursor-not-allowed",
      className,
    )}
  >
    {@render children?.()}
  </button>
{:else if asTag === "a"}
  <a
    href={href}
    onclick={onClick}
    class={cn(
      "inline-flex items-center justify-center gap-2 font-medium transition focus:outline-none focus-visible:ring-2 focus-visible:ring-(--theme-accent-primary)",
      heightClass[height],
      radiusClass[radius],
      variantClass[variant],
      disabled && "opacity-50 cursor-not-allowed",
      className,
    )}
  >
    {@render children?.()}
  </a>
{:else}
  <svelte:element
    this={asTag}
    onclick={onClick ?? undefined}
    role="button"
    tabindex="0"
    class={cn(
      "inline-flex items-center justify-center gap-2 font-medium transition focus:outline-none focus-visible:ring-2 focus-visible:ring-(--theme-accent-primary)",
      heightClass[height],
      radiusClass[radius],
      variantClass[variant],
      disabled && "opacity-50 cursor-not-allowed",
      className,
    )}
  >
    {@render children?.()}
  </svelte:element>
{/if}
