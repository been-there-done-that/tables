# Development Notes

Practical notes for working on this codebase — gotchas, workarounds, and non-obvious steps.

---

## Git: Untracking Large Binary Files

### When does this happen?

The Tauri sidecar harness binary (`src-tauri/binaries/harness-aarch64-apple-darwin`) is built locally and must **not** be committed to git. It is covered by `.gitignore` (`src-tauri/binaries/`), but if it ever gets accidentally staged and committed, `.gitignore` stops working for that file because git tracks ignored files once they are part of history.

Symptoms:
- `git status` hangs indefinitely or is very slow
- `git status` shows the binary as a modified/tracked file even though it's in `.gitignore`

### Why `git rm --cached` fails

The standard fix for untracking a file is:

```bash
git rm --cached src-tauri/binaries/harness-aarch64-apple-darwin
```

This **will hang or be killed (exit 137)** on large binaries (~59MB+). The reason is that `git rm --cached` internally loads the full object from the object store into memory to verify it before removing the index entry. On a memory-constrained system, this causes the process to get OOM-killed.

### The fix: `git update-index --force-remove`

This command removes the entry directly from the git index without reading the object content — it is much faster and memory-safe:

```bash
# 1. Clear any stale lock file left by prior killed commands
rm -f .git/index.lock

# 2. Remove from index without loading the object
git update-index --force-remove src-tauri/binaries/harness-aarch64-apple-darwin
```

After this, `.gitignore` takes effect and the file is no longer tracked. The binary on disk is **not** deleted.

### Preventing it in future

The binary is already covered by `.gitignore`:

```
src-tauri/binaries/
```

Never use `git add -A` or `git add .` — always stage files by name to avoid accidentally picking up build outputs.
