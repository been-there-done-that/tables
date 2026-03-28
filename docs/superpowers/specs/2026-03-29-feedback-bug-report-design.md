# Feedback / Bug Report Feature — Design Spec

**Date:** 2026-03-29
**Branch:** to be implemented on a feature branch off `main`

---

## Overview

Allow users to submit bug reports, feature requests, and general feedback directly from the app. Submissions create GitHub Issues in the `tables-releases` repo via a Cloudflare Worker proxy that keeps the GitHub token out of the app.

---

## Architecture

```
User (titlebar button)
    → invoke("open_feedback_window")
    → Tauri creates WebviewWindow at /feedback

User submits form
    → invoke("submit_feedback", payload)       [Rust]
    → reqwest POST to Cloudflare Worker        [Rust, existing dep]

Cloudflare Worker
    → validates + formats GitHub issue markdown
    → POST to GitHub Issues API with secret token
    → returns { issue_url } or { error }

Rust command
    → returns Ok(issue_url) | Err(message) to frontend
    → Frontend shows success toast / inline error
```

**Why Rust submits (not frontend fetch):** Avoids CORS complexity and keeps the Worker URL out of the frontend JS bundle (in the Rust binary instead).

---

## Tauri Window

- **Label:** `feedback-window`
- **Route:** `/feedback`
- **Size:** 560 × 580, not resizable
- **macOS:** `decorations: true`, `titleBarStyle: Overlay`, `hiddenTitle: true`, `transparent: true`
- **Other platforms:** `decorations: true`, `transparent: true`
- **Titlebar behavior:** `windowState.label === "feedback-window"` is added to the exclusion list in `Titlebar.svelte`, hiding all the normal toolbar buttons (same pattern as `datasource-window` and `appearance-window`)
- **Entry point:** New button in `Titlebar.svelte` right-side actions area (between the settings icon and the existing reload/AI buttons). Uses a `MessageReport` or `MessageCircle` Tabler icon. Only shown when label is not a sub-window.
- **Window command:** `open_feedback_window` in `window_commands.rs`, same singleton pattern as `open_datasource_window` (check if exists → focus; else create).

---

## Frontend: `/feedback` Route

Single file: `src/routes/feedback/+page.svelte`

### Form Types

**Bug Report**
- Title (text input, required)
- What happened? (textarea, tall, required)
- Steps to reproduce (textarea, optional)
- System info section (see below)

**Feature Request**
- Feature title (text input, required)
- Why would this be useful? (textarea, tall, required)

**General Feedback**
- Message (textarea, taller, required)

### System Info Section (Bug Report only)

Displayed as a collapsible/expandable section below the form fields. Collected via `invoke("get_system_info")` on mount.

Fields shown: App version, OS name + version, Architecture, Memory (GB).

User can click "Remove" to exclude it from the submission. If removed, system info is omitted from the payload entirely.

### Submission State

- Submit button shows a spinner while in-flight
- On success: close the window + emit a Tauri event that triggers a toast in the main window ("Feedback submitted — [view issue](url)")
- On error: show an inline error message below the footer (does not close window), button re-enables

### UI Components

Use existing app components throughout — no custom CSS:
- Tabs: button group styled with `bg-(--theme-bg-active)` for active, `hover:bg-(--theme-bg-hover)` for inactive
- Labels: `text-xs font-medium text-(--theme-fg-secondary)`
- Inputs: `FormInput` component
- Textareas: `<textarea>` with `border border-(--theme-border-default) bg-(--theme-bg-primary) rounded-md px-3 py-2 text-sm` + `focus:border-(--theme-accent-primary) focus:outline-none`
- Buttons: existing `Button` component — `variant="outline"` for Cancel, `variant="solid"` for Submit
- System info box: `bg-(--theme-bg-secondary) border border-(--theme-border-default) rounded-md`

---

## Rust Layer

### New file: `src-tauri/src/commands/feedback_commands.rs`

**Types:**

```rust
#[derive(Serialize, Deserialize)]
pub struct SystemInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub memory_gb: u64,
}

#[derive(Serialize, Deserialize)]
pub struct FeedbackPayload {
    pub feedback_type: String,     // "bug" | "feature" | "feedback"
    pub title: Option<String>,
    pub body: String,
    pub steps: Option<String>,
    pub system_info: Option<SystemInfo>,
}
```

**Commands:**

```rust
#[tauri::command]
pub async fn get_system_info(app: AppHandle) -> Result<SystemInfo, String>
// Uses `sysinfo` crate (already in Cargo.toml) + app.package_info().version

#[tauri::command]
pub async fn submit_feedback(payload: FeedbackPayload) -> Result<String, String>
// POSTs to FEEDBACK_WORKER_URL (compile-time env var or hardcoded constant)
// Returns issue URL on success, error message on failure
```

**Worker URL:** Defined as a `const FEEDBACK_WORKER_URL: &str` in `feedback_commands.rs`. Not a secret — just a URL.

### Registration

- `mod feedback_commands; pub use feedback_commands::*;` in `commands/mod.rs`
- `get_system_info`, `submit_feedback` added to `aggregate_plugin_commands!()` in `plugins/core.rs`

---

## Cloudflare Worker

**Location:** `feedback-worker/` at repo root (not compiled by Tauri build, deployed manually)

**Files:**
- `feedback-worker/index.js` — worker entry point
- `feedback-worker/wrangler.toml` — Cloudflare config
- `feedback-worker/package.json`

**Request format (POST /):**
```json
{
  "type": "bug" | "feature" | "feedback",
  "title": "optional string",
  "body": "required string",
  "steps": "optional string",
  "systemInfo": {
    "version": "0.1.0",
    "os": "macOS 15.4",
    "arch": "aarch64",
    "memory_gb": 16
  }
}
```

**Validation:**
- `type` must be one of the three values
- `body` must be a non-empty string
- Returns 400 with `{ error }` on validation failure

**CORS:**
- `Access-Control-Allow-Origin: tauri://localhost`
- OPTIONS preflight handled

**GitHub issue formatting:**

*Bug Report:*
```
## What happened
{body}

## Steps to reproduce
{steps or "Not provided"}

---
**System info**
| Field | Value |
|-------|-------|
| Version | {version} |
| OS | {os} |
| Arch | {arch} |
| Memory | {memory_gb} GB |
```

*Feature Request:*
```
## Feature request
{title}

## Why would this be useful?
{body}
```

*General Feedback:*
```
{body}
```

Issue labels: `bug` for bug reports, `enhancement` for feature requests, `feedback` for general.

**Worker secret:** `GITHUB_TOKEN` env var set in Cloudflare dashboard (fine-grained PAT with `issues: write` on `tables-releases` repo only).

**Response:**
- `200 { issue_url: "https://github.com/..." }` on success
- `4xx/5xx { error: "message" }` on failure

---

## Success Toast in Main Window

On successful submit from the feedback window:
- Feedback window emits a Tauri event `feedback://submitted` with `{ issue_url }`
- Main window layout (`+layout.svelte`) listens for this event and fires a `toast.success("Feedback submitted", { action: { label: "View", onClick: () => open(issue_url) } })`

---

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Worker unreachable | Inline error: "Could not connect. Check your internet connection." |
| Worker returns 4xx | Inline error: show Worker's error message |
| GitHub API failure | Worker returns 500; inline error: "Submission failed. Try again later." |
| Empty required fields | Client-side validation before submit; highlight empty fields |

---

## Files Changed / Created

**New files:**
- `src/routes/feedback/+page.svelte`
- `src-tauri/src/commands/feedback_commands.rs`
- `feedback-worker/index.js`
- `feedback-worker/wrangler.toml`
- `feedback-worker/package.json`

**Modified files:**
- `src-tauri/src/commands/mod.rs` — add `feedback_commands`
- `src-tauri/src/commands/window_commands.rs` — add `open_feedback_window`
- `src-tauri/src/plugins/core.rs` — register new commands
- `src/lib/Titlebar.svelte` — add feedback button + add `feedback-window` to exclusion list
- `src/routes/+layout.svelte` — listen for `feedback://submitted` event

---

## Out of Scope

- Email-based feedback (GitHub Issues only)
- Attachment / screenshot upload
- Offline queuing of submissions
- Analytics / tracking of submission rates
