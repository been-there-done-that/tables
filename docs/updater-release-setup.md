# Updater Release Setup

Before the auto-updater works in production, three one-time setup steps are required.
None of them require code changes — just key generation, config, and GitHub secrets.

---

## Step 1 — Generate the Ed25519 signing keypair

Tauri signs every update package with an Ed25519 keypair so the app can verify the
download hasn't been tampered with. This is separate from Apple notarization.

Run this once on your machine (it never needs to be run again):

```bash
pnpm tauri signer generate -w ~/.tauri/tables.key
```

You'll be asked for an optional passphrase. You can leave it empty or set one —
just remember your choice for Step 3.

The command prints something like:

```
Your public key:
dW50cnVzdGVkIGNvbW1lbnQ6IHRhdXJpIHNlY3JldCBrZXkKUldSWlhBQ...

Your private key has been saved to /Users/you/.tauri/tables.key
```

**Copy the public key** — you need it in Step 2.

> The private key file at `~/.tauri/tables.key` must stay secret and never be
> committed to the repo. Keep a backup somewhere safe (1Password, etc.).

---

## Step 2 — Put the public key and repo owner into the config

Two files need to be updated on the `feature/updater` branch:

### `src-tauri/tauri.conf.json`

Replace `PLACEHOLDER_REPLACE_WITH_REAL_KEY` with the public key from Step 1,
and replace `OWNER` with your GitHub username or org:

```json
"plugins": {
  "updater": {
    "pubkey": "<paste public key here>",
    "endpoints": [
      "https://github.com/been-there-done-that/tables-releases/releases/latest/download/latest.json"
    ]
  }
}
```

### `.github/workflows/release.yml`

Near the bottom, replace `OWNER` with your GitHub username or org:

```yaml
        with:
          ...
          owner: been-there-done-that   # ← replace OWNER with this
          repo: tables-releases
```

Commit and push these changes to `feature/updater`.

---

## Step 3 — Add secrets to the private source repo on GitHub

Go to your private source repo on GitHub:
**Settings → Secrets and variables → Actions → New repository secret**

Add these three secrets:

| Secret name | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Full contents of `~/.tauri/tables.key` (open the file, copy everything) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The passphrase you chose in Step 1, or an empty string if you skipped it |
| `RELEASES_REPO_TOKEN` | A GitHub Personal Access Token — see below |

### How to create the `RELEASES_REPO_TOKEN`

The release workflow publishes binaries to a separate **public** repo called
`tables-releases` (you need to create this repo on GitHub first if it doesn't exist).
It needs a token with write access to that repo.

1. Go to GitHub → **Settings → Developer settings → Personal access tokens → Fine-grained tokens**
2. Click **Generate new token**
3. Set expiration as you prefer (1 year is reasonable)
4. Under **Repository access**, select **Only select repositories** → pick `tables-releases`
5. Under **Permissions → Repository permissions**, set:
   - **Contents** → Read and write
   - **Metadata** → Read-only (auto-selected)
6. Click **Generate token**, copy it immediately (it's only shown once)
7. Paste it as the value for `RELEASES_REPO_TOKEN`

---

## Step 4 — Create the `tables-releases` public repo

If it doesn't exist yet:

1. Go to GitHub → **New repository**
2. Name it `tables-releases`
3. Set it to **Public**
4. Leave it empty (no README, no .gitignore)

The release workflow will populate it automatically when you push a `v*` tag.

---

## How to publish a release

Once setup is complete, publishing a new version is:

```bash
# 1. Bump version in both places
#    src-tauri/tauri.conf.json  → "version": "0.2.0"
#    src-tauri/Cargo.toml       → version = "0.2.0"

# 2. Commit
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "chore: bump version to 0.2.0"

# 3. Tag and push — this triggers the release workflow
git tag v0.2.0
git push origin main --tags
```

GitHub Actions will build for macOS (arm64 + x64), Linux, and Windows,
sign the packages, upload them to `tables-releases`, and generate the
`latest.json` manifest that the app checks for updates.

---

## Checklist

- [ ] Run `pnpm tauri signer generate -w ~/.tauri/tables.key`
- [ ] Replace `PLACEHOLDER_REPLACE_WITH_REAL_KEY` in `tauri.conf.json`
- [ ] Replace `OWNER` in `tauri.conf.json` and `release.yml`
- [ ] Create `tables-releases` public repo on GitHub
- [ ] Add `TAURI_SIGNING_PRIVATE_KEY` secret
- [ ] Add `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secret
- [ ] Add `RELEASES_REPO_TOKEN` secret (fine-grained PAT with write access to `tables-releases`)
- [ ] Merge `feature/updater` into main
- [ ] Tag a release: `git tag v0.x.x && git push --tags`
