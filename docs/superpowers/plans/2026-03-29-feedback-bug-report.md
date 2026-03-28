# Feedback / Bug Report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to submit bug reports, feature requests, and general feedback from the app, creating GitHub Issues via a Cloudflare Worker proxy.

**Architecture:** A new Tauri window at `/feedback` hosts a three-tab form. On submit, a Rust command POSTs a structured JSON payload to a Cloudflare Worker, which formats the GitHub issue markdown and calls the GitHub API using a server-side token. Success emits a Tauri event that triggers a toast in the main window.

**Tech Stack:** Rust (`reqwest`, `sysinfo`, `tauri`), SvelteKit (Svelte 5 runes), Cloudflare Workers (plain JS, Wrangler CLI), GitHub Issues API.

---

## File Map

**New files:**
- `feedback-worker/index.js` — Cloudflare Worker: validates payload, formats GitHub issue, calls GitHub API
- `feedback-worker/wrangler.toml` — Wrangler config (name, compatibility date, GITHUB_REPO var)
- `feedback-worker/package.json` — dev dependency on wrangler
- `src-tauri/src/commands/feedback_commands.rs` — `get_system_info` and `submit_feedback` Tauri commands
- `src/routes/feedback/+page.svelte` — The feedback form UI (three tabs, system info, submit logic)

**Modified files:**
- `src-tauri/src/commands/mod.rs` — add `pub mod feedback_commands; pub use feedback_commands::*;`
- `src-tauri/src/commands/window_commands.rs` — add `open_feedback_window` command
- `src-tauri/src/plugins/core.rs` — register `get_system_info`, `submit_feedback`, `open_feedback_window` in `aggregate_plugin_commands!`
- `src/lib/Titlebar.svelte` — add feedback button; add `"feedback-window"` to toolbar exclusion list
- `src/routes/+layout.svelte` — listen for `feedback://submitted` event, fire success toast

---

## Task 1: Cloudflare Worker

**Files:**
- Create: `feedback-worker/index.js`
- Create: `feedback-worker/wrangler.toml`
- Create: `feedback-worker/package.json`

- [ ] **Step 1: Create `feedback-worker/package.json`**

```json
{
  "name": "tables-feedback-worker",
  "version": "1.0.0",
  "private": true,
  "devDependencies": {
    "wrangler": "^3.0.0"
  },
  "scripts": {
    "deploy": "wrangler deploy",
    "dev": "wrangler dev"
  }
}
```

- [ ] **Step 2: Create `feedback-worker/wrangler.toml`**

```toml
name = "tables-feedback"
main = "index.js"
compatibility_date = "2024-09-23"

[vars]
GITHUB_REPO = "OWNER/tables-releases"
```

Replace `OWNER` with the actual GitHub username/org before deploying.

- [ ] **Step 3: Create `feedback-worker/index.js`**

```js
export default {
  async fetch(request, env) {
    const corsHeaders = {
      "Access-Control-Allow-Origin": "tauri://localhost",
      "Access-Control-Allow-Methods": "POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type",
    };

    if (request.method === "OPTIONS") {
      return new Response(null, { headers: corsHeaders });
    }

    if (request.method !== "POST") {
      return respond(405, { error: "Method not allowed" }, corsHeaders);
    }

    let payload;
    try {
      payload = await request.json();
    } catch {
      return respond(400, { error: "Invalid JSON" }, corsHeaders);
    }

    const { type, title, body, steps, systemInfo } = payload;

    if (!["bug", "feature", "feedback"].includes(type)) {
      return respond(400, { error: "Invalid feedback type" }, corsHeaders);
    }
    if (!body || typeof body !== "string" || !body.trim()) {
      return respond(400, { error: "Body is required" }, corsHeaders);
    }

    const issueTitle = (title && title.trim()) || autoTitle(type, body);
    const issueBody = formatBody(type, body, steps, systemInfo);
    const labels = labelFor(type);

    const ghResponse = await fetch(
      `https://api.github.com/repos/${env.GITHUB_REPO}/issues`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${env.GITHUB_TOKEN}`,
          "Content-Type": "application/json",
          "User-Agent": "tables-feedback-worker",
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        },
        body: JSON.stringify({ title: issueTitle, body: issueBody, labels }),
      }
    );

    if (!ghResponse.ok) {
      const errText = await ghResponse.text();
      console.error("GitHub API error:", ghResponse.status, errText);
      return respond(500, { error: "Submission failed. Try again later." }, corsHeaders);
    }

    const issue = await ghResponse.json();
    return respond(200, { issue_url: issue.html_url }, corsHeaders);
  },
};

function respond(status, body, corsHeaders) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...corsHeaders },
  });
}

function autoTitle(type, body) {
  const prefix =
    type === "bug" ? "[Bug]" : type === "feature" ? "[Feature]" : "[Feedback]";
  const snippet = body.trim().slice(0, 60);
  return `${prefix} ${snippet}${body.trim().length > 60 ? "…" : ""}`;
}

function labelFor(type) {
  if (type === "bug") return ["bug"];
  if (type === "feature") return ["enhancement"];
  return ["feedback"];
}

function formatBody(type, body, steps, systemInfo) {
  if (type === "bug") {
    let md = `## What happened\n\n${body.trim()}`;
    md += `\n\n## Steps to reproduce\n\n${steps && steps.trim() ? steps.trim() : "_Not provided_"}`;
    if (systemInfo) {
      md += `\n\n---\n**System info**\n| Field | Value |\n|-------|-------|\n`;
      md += `| Version | ${systemInfo.version} |\n`;
      md += `| OS | ${systemInfo.os} |\n`;
      md += `| Arch | ${systemInfo.arch} |\n`;
      md += `| Memory | ${systemInfo.memory_gb} GB |`;
    }
    return md;
  }
  if (type === "feature") {
    return `## Why would this be useful?\n\n${body.trim()}`;
  }
  return body.trim();
}
```

> **Note:** `GITHUB_TOKEN` is a secret set via `wrangler secret put GITHUB_TOKEN` — it is NOT in wrangler.toml. It must be a fine-grained PAT with `issues: write` on the `tables-releases` repo.

- [ ] **Step 4: Verify worker runs locally**

```bash
cd feedback-worker
npm install
npx wrangler dev
```

Expected: Worker starts on `http://localhost:8787`. Test with:
```bash
curl -X POST http://localhost:8787 \
  -H "Content-Type: application/json" \
  -d '{"type":"bug","body":"test bug"}'
```
Expected response (with local dev, GITHUB_TOKEN won't be set so it will 500 from GitHub — that's fine): `{"error":"Submission failed. Try again later."}` — confirms routing and validation work.

Test validation:
```bash
curl -X POST http://localhost:8787 \
  -H "Content-Type: application/json" \
  -d '{"type":"bad","body":"x"}'
```
Expected: `{"error":"Invalid feedback type"}`

- [ ] **Step 5: Commit**

```bash
git add feedback-worker/
git commit -m "feat: add Cloudflare Worker for feedback submission"
```

---

## Task 2: Rust types and `get_system_info` command

**Files:**
- Create: `src-tauri/src/commands/feedback_commands.rs`

- [ ] **Step 1: Create `src-tauri/src/commands/feedback_commands.rs`** with types and `get_system_info`

```rust
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tauri::AppHandle;

/// Returned by get_system_info and embedded in FeedbackPayload for bug reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub memory_gb: u64,
}

/// Payload sent from the frontend to submit_feedback.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackPayload {
    pub feedback_type: String,
    pub title: Option<String>,
    pub body: String,
    pub steps: Option<String>,
    pub system_info: Option<SystemInfo>,
}

/// URL of the Cloudflare Worker that proxies submissions to GitHub Issues.
/// Replace with the real deployed Worker URL before shipping.
const FEEDBACK_WORKER_URL: &str = "https://tables-feedback.OWNER.workers.dev";

#[tauri::command]
pub async fn get_system_info(app: AppHandle) -> Result<SystemInfo, String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let version = app.package_info().version.to_string();
    let os = format!(
        "{} {}",
        System::name().unwrap_or_else(|| "Unknown OS".to_string()),
        System::os_version().unwrap_or_else(|| String::new())
    )
    .trim()
    .to_string();
    let arch = std::env::consts::ARCH.to_string();
    let memory_gb = sys.total_memory() / (1024 * 1024 * 1024);

    Ok(SystemInfo {
        version,
        os,
        arch,
        memory_gb,
    })
}

#[tauri::command]
pub async fn submit_feedback(payload: FeedbackPayload) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(FEEDBACK_WORKER_URL)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                "Could not connect. Check your internet connection.".to_string()
            } else {
                format!("Request failed: {}", e)
            }
        })?;

    if response.status().is_success() {
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Invalid response: {}", e))?;
        Ok(body["issue_url"]
            .as_str()
            .unwrap_or("")
            .to_string())
    } else {
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "Submission failed. Try again later.".to_string())?;
        Err(body["error"]
            .as_str()
            .unwrap_or("Submission failed. Try again later.")
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_payload_serializes_correctly() {
        let payload = FeedbackPayload {
            feedback_type: "bug".to_string(),
            title: Some("Test bug".to_string()),
            body: "Something went wrong".to_string(),
            steps: None,
            system_info: Some(SystemInfo {
                version: "0.1.0".to_string(),
                os: "macOS 15.4".to_string(),
                arch: "aarch64".to_string(),
                memory_gb: 16,
            }),
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["feedback_type"], "bug");
        assert_eq!(json["title"], "Test bug");
        assert_eq!(json["body"], "Something went wrong");
        assert!(json["steps"].is_null());
        assert_eq!(json["system_info"]["memory_gb"], 16);
    }

    #[test]
    fn feedback_payload_optional_fields_nullable() {
        let payload = FeedbackPayload {
            feedback_type: "feedback".to_string(),
            title: None,
            body: "Great app".to_string(),
            steps: None,
            system_info: None,
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert!(json["title"].is_null());
        assert!(json["system_info"].is_null());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

```bash
cd src-tauri && cargo test feedback_commands
```

Expected output:
```
test commands::feedback_commands::tests::feedback_payload_serializes_correctly ... ok
test commands::feedback_commands::tests::feedback_payload_optional_fields_nullable ... ok
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/feedback_commands.rs
git commit -m "feat: add feedback_commands with get_system_info and submit_feedback"
```

---

## Task 3: Register commands and add `open_feedback_window`

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/commands/window_commands.rs`
- Modify: `src-tauri/src/plugins/core.rs`

- [ ] **Step 1: Add `feedback_commands` to `src-tauri/src/commands/mod.rs`**

Add these two lines after the `ddl_commands` block (end of file, before `#[cfg(test)]`):

```rust
pub mod feedback_commands;
pub use feedback_commands::*;
```

- [ ] **Step 2: Add `open_feedback_window` to `src-tauri/src/commands/window_commands.rs`**

Add this function at the end of the file, before the closing brace. Copy the exact pattern from `open_appearance_window` but with a fixed 560×580 size and `resizable(false)`:

```rust
#[tauri::command]
pub async fn open_feedback_window(app: tauri::AppHandle) -> Result<(), String> {
    const LABEL: &str = "feedback-window";

    if let Some(existing) = app.get_webview_window(LABEL) {
        existing
            .unminimize()
            .map_err(|e| format!("Failed to unminimize: {}", e))?;
        existing
            .show()
            .map_err(|e| format!("Failed to show: {}", e))?;
        existing
            .set_focus()
            .map_err(|e| format!("Failed to focus: {}", e))?;
        return Ok(());
    }

    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        LABEL,
        tauri::WebviewUrl::App("feedback".into()),
    )
    .title("Send Feedback")
    .inner_size(560.0, 580.0)
    .resizable(false)
    .decorations(cfg!(target_os = "macos"))
    .transparent(true)
    .focused(true)
    .title_bar_style(tauri::TitleBarStyle::Overlay)
    .hidden_title(true)
    .build()
    .map_err(|e| format!("Failed to create feedback window: {}", e))?;

    Ok(())
}
```

- [ ] **Step 3: Register the three new commands in `src-tauri/src/plugins/core.rs`**

Find the `aggregate_plugin_commands!` macro. Locate the window commands section (look for `open_datasource_window`, `open_appearance_window`, `create_new_window`). Add the three new commands in that same block:

```rust
open_feedback_window,
get_system_info,
submit_feedback,
```

- [ ] **Step 4: Verify it compiles**

```bash
cd src-tauri && cargo build 2>&1 | head -50
```

Expected: no errors. If there are unused import warnings for `FeedbackPayload` or `SystemInfo`, add `#[allow(unused_imports)]` or check that the derive macros are correct.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/mod.rs \
        src-tauri/src/commands/window_commands.rs \
        src-tauri/src/plugins/core.rs
git commit -m "feat: register feedback commands and add open_feedback_window"
```

---

## Task 4: Frontend `/feedback` page

**Files:**
- Create: `src/routes/feedback/+page.svelte`

- [ ] **Step 1: Create `src/routes/feedback/+page.svelte`**

```svelte
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
      {#each ([["bug", "🐛 Bug Report"], ["feature", "✨ Feature Request"], ["feedback", "💬 General Feedback"]] as const) as [tab, label]}
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
                📎 System info will be attached
              </span>
              {#if includeSystemInfo}
                <button
                  class="text-xs text-red-400 hover:text-red-300 transition-colors"
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
        <span class="text-xs text-red-400">{errorMessage}</span>
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
```

- [ ] **Step 2: Run type check**

```bash
cd /path/to/project && pnpm check 2>&1 | grep -A3 "feedback"
```

Expected: no errors in the feedback page. (If you see `Cannot find name 'Math'` or similar in `updater.svelte.ts` — those are pre-existing, unrelated.)

- [ ] **Step 3: Commit**

```bash
git add src/routes/feedback/
git commit -m "feat: add feedback form page at /feedback route"
```

---

## Task 5: Titlebar button and layout event listener

**Files:**
- Modify: `src/lib/Titlebar.svelte`
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: Add import for the feedback icon in `src/lib/Titlebar.svelte`**

At the top of the `<script>` block, add this import alongside the other Tabler icon imports:

```typescript
import IconMessageReport from "@tabler/icons-svelte/icons/message-report";
```

- [ ] **Step 2: Add `"feedback-window"` to the toolbar exclusion list in `Titlebar.svelte`**

Find this line (appears twice in the file — the outer `{#if}` block and the inner one):
```svelte
{#if !["datasource-window", "appearance-window"].includes(windowState.label)}
```

Update **both occurrences** to:
```svelte
{#if !["datasource-window", "appearance-window", "feedback-window"].includes(windowState.label)}
```

- [ ] **Step 3: Add the feedback button in `Titlebar.svelte`**

Find the settings button block. The settings button ends with `</button>`. After it and before the AI Assistant button (`<!-- AI Assistant -->`), add:

```svelte
<!-- Feedback -->
<button
  class={cn(
    "h-6 w-6 flex items-center justify-center rounded-md border transition-all",
    "hover:bg-(--theme-bg-hover) border-transparent",
  )}
  onclick={async () => {
    try {
      await invoke("open_feedback_window");
    } catch (e) {
      console.error("Failed to open feedback window:", e);
    }
  }}
  title="Send Feedback"
>
  <IconMessageReport class="size-5" />
</button>
```

- [ ] **Step 4: Add `feedback://submitted` listener in `src/routes/+layout.svelte`**

First, add the `toast` import alongside the existing imports at the top of `<script>`:
```typescript
import { toast } from "svelte-sonner";
```

Then inside the `setup()` function in `onMount`, after the existing `windowState.init()` and `schemaStore.initialize()` calls, add:

```typescript
const unlistenFeedback = await listen<{ issue_url: string }>(
  "feedback://submitted",
  (event) => {
    toast.success("Feedback submitted!", {
      description: "Thank you — your issue has been created.",
      action: {
        label: "View",
        onClick: () => {
          window.open(event.payload.issue_url, "_blank");
        },
      },
    });
  }
);
```

And in the cleanup `return () => { ... }` block, add:
```typescript
unlistenFeedback();
```

- [ ] **Step 5: Run type check**

```bash
pnpm check 2>&1 | grep -E "error|Error" | head -20
```

Expected: no new errors introduced.

- [ ] **Step 6: Commit**

```bash
git add src/lib/Titlebar.svelte src/routes/+layout.svelte
git commit -m "feat: add feedback button to titlebar and submitted toast"
```

---

## Task 6: Final compile and smoke test

- [ ] **Step 1: Full Rust build**

```bash
cd src-tauri && cargo build 2>&1 | tail -20
```

Expected: `Finished` with no errors.

- [ ] **Step 2: Run all Rust tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -20
```

Expected: `test result: ok. N passed; 0 failed`.

- [ ] **Step 3: Start dev mode and manual smoke test**

```bash
pnpm tauri dev
```

Manual checks:
1. Click the `MessageReport` icon in the titlebar — feedback window opens at 560×580
2. Titlebar in the feedback window shows only the macOS traffic lights (no toolbar buttons)
3. Switch tabs — form fields reset, system info only visible on Bug Report tab
4. Click "Remove" on system info — it collapses; "Add back" restores it
5. Submit with empty fields — inline error appears, window stays open
6. Submit with valid fields — button shows spinner (Worker URL placeholder will 500, shows error message inline)

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete feedback/bug report feature"
```

---

## Post-Implementation: Deploy the Worker

Before the feature is usable end-to-end:

1. Update `FEEDBACK_WORKER_URL` in `feedback_commands.rs` with the real deployed Worker URL
2. Update `GITHUB_REPO` in `feedback-worker/wrangler.toml` with the real `OWNER/tables-releases`
3. Run `cd feedback-worker && npm install && npx wrangler secret put GITHUB_TOKEN` and paste the fine-grained PAT
4. Run `npx wrangler deploy`
5. Test end-to-end: submit a real bug report, verify the GitHub Issue appears in `tables-releases`

> The fine-grained PAT needs `issues: write` on `tables-releases` only. Create it at GitHub → Settings → Developer settings → Fine-grained tokens.
