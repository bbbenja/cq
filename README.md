# cq — commit quick

A Rust TUI tool that lets you type your commit message while pre-commit hooks run in the background. No more waiting.

```
┌─ Staged files (2) ──────────┬─ Recent commits ──────────────┐
│  M  src/main.rs             │  a2d7437 feat: show 5 recent  │
│  A  src/utils.rs            │  fcd5def fix: amend pre-fills  │
├─ Commit message ────────────┤  2b5a701 feat: add Ctrl+T     │
│                             │  010283f feat: retry failed    │
│  fix: resolve race in pool  │  24642cb feat: scrollable     │
│                             ├─ Pre-commit hook ─────────────┤
│                             │  ✅ Passed (1.3s)             │
└─────────────────────────────┴───────────────────────────────┘
  Ctrl+S: commit   Ctrl+T: type   Ctrl+C/Esc: abort
```

## Install

### Quick install (prebuilt binary)

```bash
curl -fsSL https://raw.githubusercontent.com/bbbenja/cq/main/install.sh | sh
```

### From source

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

### CLI flags

| Flag | Description |
|---|---|
| `--amend` | Amend the previous commit (pre-fills the message) |
| `-s` / `--signoff` | Add Signed-off-by trailer |
| `-a` / `--all` | Stage all modified/deleted files before committing |
| `--author AUTHOR` | Override the commit author |
| `--date DATE` | Override the author date |
| `--allow-empty` | Allow empty commits |
| `-c` / `--conventional` | Start with the conventional commit type selector |

### Keybindings

| Key | Action |
|---|---|
| **Ctrl+S** / **Ctrl+Enter** | Submit commit |
| **Ctrl+T** | Open conventional commit type selector |
| **Ctrl+R** | Retry failed hook |
| **Alt+Up** / **Alt+Down** | Scroll hook output |
| **Esc** / **Ctrl+C** | Abort |

### Layout

The TUI is split into two columns:

- **Left** — staged files list (color-coded: green=added, yellow=modified, red=deleted) and the commit message editor
- **Right** — last 5 commits for context, then live pre-commit hook output with scrollable log

### Hook status

- **⏳ Running...** — hook is executing, with live output
- **✅ Passed** — hook succeeded, ready to commit
- **❌ Failed** — hook failed, commit blocked (press Ctrl+R to retry)

If you submit while the hook is still running, cq waits for it to finish and commits automatically on success.

If no pre-commit hook is found, cq commits directly.

### Conventional commits

Press **Ctrl+T** (or start with `cq -c`) to open the type selector:

1. Pick a type (feat, fix, chore, refactor, docs, test, style, ci, perf, build)
2. Optionally enter a scope
3. The prefix is inserted into the message editor (e.g. `feat(api): `)

### Hook manager support

cq supports hooks managed by [Husky](https://typicode.github.io/husky/), [lefthook](https://github.com/evilmartians/lefthook), and any tool that sets `core.hooksPath`. It also reads `commit.template` from git config.

## Releasing

```bash
./scripts/release.sh 0.2.0   # bumps Cargo.toml, commits, tags
git push && git push --tags   # triggers GitHub Actions build + release
```

The release workflow builds binaries for Linux (x86_64, aarch64) and macOS (x86_64, aarch64), then publishes them to GitHub Releases with SHA256 checksums.

## Requirements

- macOS or Linux
- Git

## License

MIT — see [LICENSE](LICENSE).
