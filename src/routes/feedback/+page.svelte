<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { cn } from "$lib/utils";
  import FormInput from "$lib/components/FormInput.svelte";

  type FeedbackType = "bug" | "feature" | "feedback";

  interface SystemInfo {
    version: string;
    os: string;
    arch: string;
    memory_gb: number;
  }

  let activeTab = $state<FeedbackType>("bug");
  let title = $state("");
  let body = $state("");
  let steps = $state("");
  let systemInfo = $state<SystemInfo | null>(null);
  let includeSystemInfo = $state(true);
  let isSubmitting = $state(false);
  let errorMessage = $state<string | null>(null);

  onMount(async () => {
    try {
      systemInfo = await invoke<SystemInfo>("get_system_info");
    } catch (e) {
      console.error("[Feedback] Failed to get system info", e);
      includeSystemInfo = false;
    }
  });

  function switchTab(tab: FeedbackType) {
    activeTab = tab;
    title = "";
    body = "";
    steps = "";
    errorMessage = null;
  }

  function validate(): string | null {
    if (!body.trim()) return "Please describe your feedback.";
    if (activeTab !== "feedback" && !title.trim()) return "Please add a title.";
    return null;
  }

  async function handleSubmit() {
    const validationError = validate();
    if (validationError) {
      errorMessage = validationError;
      return;
    }

    isSubmitting = true;
    errorMessage = null;

    try {
      const payload = {
        feedback_type: activeTab,
        title: title.trim() || null,
        body: body.trim(),
        steps: steps.trim() || null,
        system_info:
          activeTab === "bug" && includeSystemInfo ? systemInfo : null,
      };

      const issueUrl = await invoke<string>("submit_feedback", { payload });
      await emit("feedback://submitted", { issue_url: issueUrl });
      await getCurrentWindow().close();
    } catch (e) {
      errorMessage =
        typeof e === "string" ? e : "Submission failed. Please try again.";
    } finally {
      isSubmitting = false;
    }
  }

  async function handleCancel() {
    await getCurrentWindow().close();
  }
</script>

<div
  class="flex h-screen flex-col overflow-hidden"
  style="background: var(--theme-bg-primary); color: var(--theme-fg-primary);"
>
  <!-- Titlebar spacer (accounts for macOS traffic lights overlay) -->
  <div class="h-8 shrink-0" aria-hidden="true"></div>

  <!-- Scrollable body -->
  <div class="flex-1 overflow-y-auto px-5 py-4 flex flex-col gap-4">
    <!-- Tab selector -->
    <div class="flex gap-1.5">
      {#each ([["bug", "🐛 Bug Report"], ["feature", "✨ Feature Request"], ["feedback", "💬 General Feedback"]] as [FeedbackType, string][]) as [tab, label]}
        <button
          class={cn(
            "flex-1 rounded-md border px-2 py-1.5 text-xs font-medium transition-all",
            activeTab === tab
              ? "border-(--theme-accent-primary) bg-(--theme-accent-primary)/10 text-(--theme-accent-primary)"
              : "border-(--theme-border-default) text-(--theme-fg-secondary) hover:bg-(--theme-bg-hover) hover:text-(--theme-fg-primary)"
          )}
          onclick={() => switchTab(tab)}
        >
          {label}
        </button>
      {/each}
    </div>

    <!-- Bug Report fields -->
    {#if activeTab === "bug"}
      <div class="flex flex-col gap-3">
        <FormInput
          label="Title"
          inputId="bug-title"
          bind:value={title}
          placeholder="Short description of the bug…"
        />
        <div class="flex flex-col gap-1">
          <label for="bug-body" class="text-xs font-medium text-(--theme-fg-secondary)">
            What happened?
          </label>
          <textarea
            id="bug-body"
            bind:value={body}
            placeholder="Describe what went wrong…"
            rows={5}
            class={cn(
              "w-full resize-none rounded-md border px-3 py-2 text-sm leading-relaxed",
              "border-(--theme-border-default) bg-(--theme-bg-primary) text-(--theme-fg-primary)",
              "placeholder:text-(--theme-fg-tertiary)",
              "focus:border-(--theme-accent-primary) focus:outline-none"
            )}
          ></textarea>
        </div>
        <div class="flex flex-col gap-1">
          <label for="bug-steps" class="text-xs font-medium text-(--theme-fg-secondary)">
            Steps to reproduce
            <span class="font-normal text-(--theme-fg-tertiary)">(optional)</span>
          </label>
          <textarea
            id="bug-steps"
            bind:value={steps}
            placeholder="1. Open a connection…"
            rows={3}
            class={cn(
              "w-full resize-none rounded-md border px-3 py-2 text-sm leading-relaxed",
              "border-(--theme-border-default) bg-(--theme-bg-primary) text-(--theme-fg-primary)",
              "placeholder:text-(--theme-fg-tertiary)",
              "focus:border-(--theme-accent-primary) focus:outline-none"
            )}
          ></textarea>
        </div>

        <!-- System info -->
        {#if systemInfo}
          <div
            class="rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) overflow-hidden"
          >
            <div class="flex items-center justify-between px-3 py-2">
              <span class="flex items-center gap-1.5 text-xs font-medium text-(--theme-fg-secondary)">
                📎 {includeSystemInfo ? "System info will be attached" : "System info removed"}
              </span>
              {#if includeSystemInfo}
                <button
                  class="text-xs text-(--theme-fg-tertiary) hover:text-(--theme-fg-secondary) transition-colors"
                  onclick={() => (includeSystemInfo = false)}
                >Remove</button>
              {:else}
                <button
                  class="text-xs text-(--theme-accent-primary) hover:opacity-80 transition-opacity"
                  onclick={() => (includeSystemInfo = true)}
                >Add back</button>
              {/if}
            </div>
            {#if includeSystemInfo}
              <div class="px-3 pb-2.5 flex flex-col gap-1">
                {#each [["Version", systemInfo.version], ["OS", systemInfo.os], ["Arch", systemInfo.arch], ["Memory", `${systemInfo.memory_gb} GB`]] as [key, val]}
                  <div class="flex gap-3 text-xs">
                    <span class="w-16 shrink-0 text-(--theme-fg-tertiary)">{key}</span>
                    <span class="font-mono text-(--theme-fg-secondary)">{val}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Feature Request fields -->
    {#if activeTab === "feature"}
      <div class="flex flex-col gap-3">
        <FormInput
          label="Feature title"
          inputId="feature-title"
          bind:value={title}
          placeholder="What would you like to see?"
        />
        <div class="flex flex-col gap-1">
          <label for="feature-body" class="text-xs font-medium text-(--theme-fg-secondary)">
            Why would this be useful?
          </label>
          <textarea
            id="feature-body"
            bind:value={body}
            placeholder="Describe your use case…"
            rows={6}
            class={cn(
              "w-full resize-none rounded-md border px-3 py-2 text-sm leading-relaxed",
              "border-(--theme-border-default) bg-(--theme-bg-primary) text-(--theme-fg-primary)",
              "placeholder:text-(--theme-fg-tertiary)",
              "focus:border-(--theme-accent-primary) focus:outline-none"
            )}
          ></textarea>
        </div>
      </div>
    {/if}

    <!-- General Feedback fields -->
    {#if activeTab === "feedback"}
      <div class="flex flex-col gap-3">
        <div class="flex flex-col gap-1">
          <label for="feedback-body" class="text-xs font-medium text-(--theme-fg-secondary)">
            Your message
          </label>
          <textarea
            id="feedback-body"
            bind:value={body}
            placeholder="Share anything on your mind…"
            rows={8}
            class={cn(
              "w-full resize-none rounded-md border px-3 py-2 text-sm leading-relaxed",
              "border-(--theme-border-default) bg-(--theme-bg-primary) text-(--theme-fg-primary)",
              "placeholder:text-(--theme-fg-tertiary)",
              "focus:border-(--theme-accent-primary) focus:outline-none"
            )}
          ></textarea>
        </div>
      </div>
    {/if}
  </div>

  <!-- Footer -->
  <div
    class="shrink-0 border-t px-5 py-3 flex items-center justify-between gap-3"
    style="border-color: var(--theme-border-default);"
  >
    <span class="text-xs text-(--theme-fg-tertiary)">
      Creates a GitHub issue in tables-releases
    </span>

    <div class="flex items-center gap-2">
      {#if errorMessage}
        <span class="text-xs text-destructive">{errorMessage}</span>
      {/if}
      <button
        class={cn(
          "h-7 rounded-md border px-3 text-xs font-medium transition-all",
          "border-(--theme-border-default) text-(--theme-fg-secondary)",
          "hover:bg-(--theme-bg-hover) hover:text-(--theme-fg-primary)"
        )}
        onclick={handleCancel}
        disabled={isSubmitting}
      >
        Cancel
      </button>
      <button
        class={cn(
          "h-7 rounded-md px-3 text-xs font-medium transition-all",
          "bg-(--theme-accent-primary) text-white",
          "hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed",
          "flex items-center gap-1.5"
        )}
        onclick={handleSubmit}
        disabled={isSubmitting}
      >
        {#if isSubmitting}
          <svg class="size-3 animate-spin" viewBox="0 0 24 24" fill="none">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
          </svg>
          Submitting…
        {:else}
          Submit
        {/if}
      </button>
    </div>
  </div>
</div>
