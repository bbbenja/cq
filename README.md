# cq — commit quick

A Rust TUI tool that lets you type your commit message while pre-commit hooks run in the background. No more waiting.

```
┌─ Commit message ─────────────────────────────┐
│                                               │
│  fix: resolve race condition in worker pool   │
│                                               │
├─ Pre-commit hook ─────────────────────────────┤
│  ✅ Passed (1.3s)                             │
│                                               │
└───────────────────────────────────────────────┘
  Ctrl+S: commit   Ctrl+C: abort
```

## Install

```bash
cargo install --path .
```

### Set up the git alias

```bash
cq install    # git commit now calls cq
cq uninstall  # restore default git commit
```

## Usage

Stage your changes, then commit:

```bash
git add -p
git commit    # opens cq if the alias is installed
# or run directly:
cq
```

The TUI opens immediately. Start typing your commit message — the pre-commit hook is already running in the background.

### Keybindings

| Key | Action |
|---|---|
| **Ctrl+S** / **Ctrl+Enter** | Submit commit |
| **Esc** / **Ctrl+C** | Abort |

### Hook status

- **⏳ Running...** — hook is executing, with live output
- **✅ Passed** — hook succeeded, ready to commit
- **❌ Failed** — hook failed, commit blocked (output shown in red)

If you submit while the hook is still running, cq waits for it to finish and commits automatically on success.

If no pre-commit hook is found, cq commits directly.

## Requirements

- macOS or Linux
- Rust stable toolchain
- Git
