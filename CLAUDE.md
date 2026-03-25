# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**cq** (commit quick) — a Rust TUI that lets you type your commit message while pre-commit hooks run in the background. Replaces `git commit` via a global git alias.

## Build & Test Commands

```bash
cargo build              # build
cargo check              # fast type-check
cargo test               # run all tests
cargo fmt --check        # check formatting
cargo clippy -- -D warnings  # lint
cargo install --path .   # install the `cq` binary locally
```

Pre-commit hooks (via cargo-husky) run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` on every commit.

## Architecture

The app is a single-binary async TUI (tokio + ratatui + crossterm).

**Flow:** `main.rs` parses CLI (clap) → `app::run()` does preflight checks (in repo? staged changes?) → spawns the pre-commit hook in a background task → runs the TUI event loop → commits with `--no-verify` (since hooks already ran manually).

### Key modules

- **`app.rs`** — TUI event loop, state machine (`HookStatus`: NoHook → Running → Passed/Failed, with Waiting for submit-while-running). Owns the `App` struct and keyboard handling.
- **`hook.rs`** — Finds the pre-commit hook (checks `core.hooksPath` first for Husky/lefthook support, falls back to `.git/hooks/`). Spawns it async and streams stdout/stderr as `HookEvent`s via an unbounded mpsc channel.
- **`ui.rs`** — Stateless rendering. Three-panel layout: commit message textarea, hook status panel (with spinner/output), footer with keybindings.
- **`git.rs`** — Thin wrappers around git commands (repo check, staged changes check, commit with `--no-verify`).

### Design decisions

- Commit uses `--no-verify` because cq already ran the hook itself — avoids running hooks twice.
- Hook output streams in real-time via async line readers on both stdout and stderr.
- If the user submits while the hook is still running, the app enters a `Waiting` state and auto-commits when the hook passes (or blocks if it fails).
