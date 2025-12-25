<script lang="ts">
  import Button from "$lib/components/Button.svelte";
  import Select, { type Option } from "$lib/components/Select.svelte";
  import FloatingWindow from "$lib/components/FloatingWindow.svelte";
  import { cn } from "$lib/utils";

  const selectOptions: Option[] = [
    { value: "", label: "(none)" },
    { value: "alpha", label: "Alpha" },
    { value: "bravo", label: "Bravo" },
    { value: "charlie", label: "Charlie" },
  ];

  let btnVariant = $state<"solid" | "ghost" | "outline" | "subtle">("solid");
  let btnRadius = $state<"none" | "sm" | "md" | "lg" | "full">("md");
  let btnHeight = $state<"8" | "10" | "12">("10");
  let selectRadius = $state<"sm" | "md" | "lg">("md");
  let selectHeight = $state<"sm" | "md" | "lg">("md");
  let selected = $state("alpha");
  let windowOpen = $state(false);
</script>

<svelte:head>
  <title>Components Demo</title>
</svelte:head>

<div class="p-6 space-y-6 text-(--theme-fg-primary) bg-(--theme-bg-primary)">
  <section class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <h2 class="font-semibold text-sm">Button</h2>
      <div class="flex items-center gap-2 text-xs">
        <label class="flex items-center gap-1">
          <span>Variant</span>
          <Select
            options={["solid", "ghost", "outline", "subtle"]}
            value={btnVariant}
            onCommit={(v: string) => (btnVariant = v as typeof btnVariant)}
            radius="sm"
            class="min-w-[130px]"
          />
        </label>
        <label class="flex items-center gap-1">
          <span>Radius</span>
          <Select
            options={[
              { value: "none", label: "None" },
              { value: "sm", label: "Sm" },
              { value: "md", label: "Md" },
              { value: "lg", label: "Lg" },
              { value: "full", label: "Full" },
            ]}
            value={btnRadius}
            onCommit={(v: string) => (btnRadius = v as typeof btnRadius)}
            radius="sm"
            class="min-w-[130px]"
          />
        </label>
        <label class="flex items-center gap-1">
          <span>Height</span>
          <Select
            options={["8", "10", "12"]}
            value={btnHeight}
            onCommit={(v: string) => (btnHeight = v as typeof btnHeight)}
            radius="sm"
            class="min-w-[90px]"
          />
        </label>
      </div>
    </div>
    <div class="flex items-center gap-3 flex-wrap">
      <Button variant={btnVariant} radius={btnRadius} height={btnHeight}>Primary</Button>
      <Button variant={btnVariant} radius={btnRadius} height={btnHeight} disabled>Disabled</Button>
      <Button as="a" href="#" variant={btnVariant} radius={btnRadius} height={btnHeight}>Anchor</Button>
    </div>
  </section>

  <section class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <h2 class="font-semibold text-sm">Select</h2>
      <div class="flex items-center gap-2 text-xs">
        <label class="flex items-center gap-1">
          <span>Radius</span>
          <Select
            options={selectOptions}
            value={selectRadius}
            onCommit={(v: string) => (selectRadius = v as typeof selectRadius)}
            radius="sm"
            class="min-w-[130px]"
          />
        </label>
        <label class="flex items-center gap-1">
          <span>Height</span>
          <Select
            options={["sm", "md", "lg"]}
            value={selectHeight}
            onCommit={(v: string) => (selectHeight = v as typeof selectHeight)}
            radius="sm"
            class="min-w-[110px]"
          />
        </label>
      </div>
    </div>
    <div class="flex items-center gap-3 flex-wrap">
      <label>
        <Select
          options={selectOptions}
          value={selected}
          radius={selectRadius}
          height={selectHeight}
          onCommit={(v: string) => (selected = v)}
        />
      </label>
      <div class="text-sm opacity-80">Selected: {selected || "(none)"}</div>
    </div>
  </section>

  <section class="space-y-3 p-4 rounded-lg border border-(--theme-border-default) bg-(--theme-bg-secondary)">
    <div class="flex items-center justify-between flex-wrap gap-3">
      <h2 class="font-semibold text-sm">Floating Window</h2>
      <Button variant="outline" height="8" radius="md" onClick={() => (windowOpen = true)}>Open Window</Button>
    </div>
    <p class="text-sm opacity-80">Draggable dialog with optional modal overlay; shortcuts can be passed via props.</p>
  </section>
</div>

<FloatingWindow
  title="Demo Window"
  bind:open={windowOpen}
  modal={true}
  class="max-w-xl"
  onClose={() => (windowOpen = false)}
  onMaximize={() => {}}
>
  <div class="p-4 space-y-3 text-sm">
    <p>This window is draggable by the header bar.</p>
    <p>Pass <code>openShortcut</code> / <code>closeShortcut</code> to toggle via keyboard, and <code>onClose</code> / <code>onMaximize</code> handlers as needed.</p>
  </div>
</FloatingWindow>
