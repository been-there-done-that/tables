# Auto-Updater Design

**Date:** 2026-03-29
**Status:** Approved

## Overview

Add automatic update checking and installation to Tables using `tauri-plugin-updater` backed by GitHub Releases. Updates are checked on launch (background, non-blocking) and on manual user request. The user is prompted before any download begins. Download progress is shown inline in the titlebar. No Apple notarization for now — Tauri's Ed25519 signing is sufficient.

## Decisions Made

| Question | Decision |
|---|---|
| Distribution | GitHub Releases (public releases repo, source stays private) |
| Update check trigger | On launch (3s delay, non-blocking) + manual from settings |
| User flow | Confirm download → progress in titlebar → restart to install |
| Chip style | Full pill (`border-radius: 999px`) |
| Chip placement | Titlebar.svelte, before all right-side icon buttons |
| Progress bar | Thin 2px line at bottom of titlebar, full width |
| Toast on launch | Yes — svelte-sonner, auto-dismisses, non-blocking |
| Apple signing | Not yet — Ed25519 only for now |

## Update Flow

```
App launch
  └─ 3s delay (non-blocking)
       └─ check_for_update
            ├─ null → idle (nothing shown)
            └─ update found
                 ├─ toast: "Tables vX.Y.Z is available" (auto-dismisses)
                 └─ pill appears in titlebar: "vX.Y.Z available"
                      └─ user clicks pill
                           └─ confirmation dialog: "Download update?" [Cancel] [Download]
                                └─ user confirms
                                     └─ pill → "Downloading… N%" + progress bar
                                          └─ download complete
                                               └─ pill → "Restart to install" (green)
                                                    └─ user clicks
                                                         └─ install + relaunch

Settings page: "Check for updates" button
  └─ same flow from "check_for_update" onward
```

## Backend

### New file: `src-tauri/src/commands/update_commands.rs`

Three Tauri commands:

**`check_for_update`**
Calls `tauri_plugin_updater`. Returns `Option<UpdateInfo>` where `UpdateInfo = { version: String, body: Option<String>, date: Option<String> }`. Stores the pending `Update` handle in `APP_UPDATER: Mutex<Option<Update>>`.

**`download_update`**
Takes the stored `Update` handle, streams the download, emits `update://progress` events: `{ downloaded: u64, total: Option<u64> }`. Stores the downloaded update in `PENDING_INSTALL: Mutex<Option<PendingInstall>>`.

**`install_update`**
Calls `.install()` on the stored `PendingInstall`, which relaunches the app. Non-recoverable — app restarts.

### Registration

Add to `lib.rs` plugin setup:
```rust
.plugin(tauri_plugin_updater::Builder::new().build())
```

Add commands to `invoke_handler`.

### `tauri.conf.json` updater block

```json
"plugins": {
  "updater": {
    "pubkey": "<ED25519_PUBLIC_KEY>",
    "endpoints": [
      "https://github.com/<owner>/tables-releases/releases/latest/download/latest.json"
    ]
  }
}
```

## Frontend

### New store: `src/lib/stores/updater.svelte.ts`

```ts
type UpdaterStatus =
  | 'idle'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'error'

// State
let status: UpdaterStatus
let pendingVersion: string | null
let downloadPercent: number        // 0–100
let errorMessage: string | null

// Methods
checkForUpdate()   // invoke check_for_update, show toast if found
download()         // invoke download_update, listen to update://progress events
install()          // invoke install_update
```

Listens to `update://progress` events from Rust to update `downloadPercent`.
On launch, `checkForUpdate()` is called from `+layout.svelte` after a 3s `setTimeout`.

### New component: `src/lib/components/UpdateChip.svelte`

Full pill chip rendered in `Titlebar.svelte` before the right-side icon group. Hidden when `status === 'idle'`. Four visual states:

| Status | Color tokens | Content |
|---|---|---|
| `available` | `--theme-accent-primary` + `color-mix(…15%)` bg | `● vX.Y.Z available` |
| `downloading` | `--theme-accent-primary` | `⟳ Downloading… N%` (spinner) |
| `ready` | `--chip-result-color/bg/border` | `● Restart to install` |
| `error` | red-400 / red-500 | `⚠ Update failed` |

Clicking `available` → opens a confirmation dialog (uses existing `ConfirmationModal.svelte`) then calls `updaterStore.download()`.
Clicking `ready` → calls `updaterStore.install()`.
Clicking `error` → retries `checkForUpdate()`.

### Progress bar

2px line at the bottom edge of the Titlebar div, full width. Visible only during `downloading`. Fill uses `--theme-accent-primary`, track uses `--theme-border-subtle`.

### Settings page addition

In `src/routes/settings/+page.svelte`, add an "Updates" section:
- Current version (from `__APP_VERSION__` Vite constant)
- "Check for updates" button — calls `updaterStore.checkForUpdate()`
- Status text: "Up to date" / "vX.Y.Z available — click the titlebar chip to install"

## GitHub Releases Setup

### One-time keypair generation
```bash
pnpm tauri signer generate -w ~/.tauri/tables.key
# Copy public key → tauri.conf.json plugins.updater.pubkey
# Copy private key → GitHub secret TAURI_SIGNING_PRIVATE_KEY
```

### New repo: `<owner>/tables-releases` (public)
Stores only compiled binaries and the `latest.json` manifest. Source stays in the private repo.

### New file: `.github/workflows/release.yml`
Triggers on `v*` tags pushed to the private repo. Steps:
1. Checkout, install pnpm, install Rust
2. `pnpm tauri build` with `TAURI_SIGNING_PRIVATE_KEY` env var
3. Upload `.dmg` / `.app.tar.gz` / `.msi` / `.AppImage` to `tables-releases` GitHub Release
4. Generate and upload `latest.json` manifest

`latest.json` format (Tauri v2 updater):
```json
{
  "version": "0.3.0",
  "notes": "Release notes here",
  "pub_date": "2026-03-29T00:00:00Z",
  "platforms": {
    "darwin-aarch64": { "signature": "...", "url": "https://..." },
    "darwin-x86_64":  { "signature": "...", "url": "https://..." },
    "windows-x86_64": { "signature": "...", "url": "https://..." },
    "linux-x86_64":   { "signature": "...", "url": "https://..." }
  }
}
```

## Error Handling

| Scenario | Behaviour |
|---|---|
| No internet / GitHub unreachable | Silent fail — status stays `idle`, nothing shown to user |
| Update check fails | Silent fail on launch; show error toast on manual check |
| Download interrupted | Chip shows `error` state with retry on click |
| Signature verification fails | Tauri plugin rejects the update — show error toast |
| User dismisses confirmation | Status reverts to `available`, chip stays visible |

## Files Changed / Created

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | Add `tauri-plugin-updater = "2"` |
| `src-tauri/tauri.conf.json` | Add `plugins.updater` block |
| `src-tauri/src/lib.rs` | Register plugin + update commands |
| `src-tauri/src/commands/update_commands.rs` | New — 3 commands |
| `src/lib/stores/updater.svelte.ts` | New — updater state store |
| `src/lib/components/UpdateChip.svelte` | New — pill chip component |
| `src/lib/Titlebar.svelte` | Add `UpdateChip` before right-side icons |
| `src/routes/settings/+page.svelte` | Add Updates section |
| `.github/workflows/release.yml` | New — build + release workflow |

## Out of Scope

- Apple notarization / hardened runtime (future, requires paid Apple Developer account)
- Staged rollouts / beta channel (can add a custom update server later)
- Delta/differential updates
- Rollback
