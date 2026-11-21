# Agent guide

Rules for AI coding agents (Claude Code, Codex, etc.) working on Ferric. People are welcome to follow them too.

## Before you start

1. You were given a task file: `docs/tasks/Txx-*.md`. Read all of it, then [docs/CONTRACTS.md](docs/CONTRACTS.md) and [docs/DESIGN.md](docs/DESIGN.md).
2. Check the task's **Depends on** list. Every dependency must already be merged to `main`. If one isn't, stop and say so.
3. Create the branch named in the task, from the latest `main`. Work in your own git worktree if other agents share this checkout.

## Rules for working side by side

Several agents work on this repo at once. These rules keep them from colliding:

- **Edit only the paths listed under "Owns"** in your task. You may read anything.
- **Never edit `crates/ferric-core`** unless your task owns it. It holds the contracts every other task builds against. If you need a contract change, stop, and describe the change you need in your PR description or final report.
- **Never edit another task's owned paths**, even to fix a small bug. Note it in your report instead.
- Shared files that several tasks need to change (`Cargo.toml` workspace deps, `ui/package.json`, `src-tauri/src/main.rs`): add **only** the lines your task needs, keep the list in alphabetical order to reduce merge conflicts, and don't reformat anything else.
- Don't add dependencies the task doesn't list without saying why in the PR.
- Don't commit secrets, client IDs or tokens. Use `.env.example` for placeholders.

## Definition of done

A task is done when:

- every acceptance criterion in the task file is met, and
- the PR checklist in [CONTRIBUTING.md](CONTRIBUTING.md) passes locally, and
- a PR is open against `main`, titled in Conventional Commit form, with a body containing:
  - `Closes task Txx`
  - what was built, and how you verified each acceptance criterion
  - anything you couldn't verify (for example, it needs Spotify running or Premium)
  - any follow-ups or contract changes you need

## Conventions

- Rust 2021 edition, `thiserror` for library errors, `anyhow` only in `src-tauri`.
- Async: `tokio`. Don't block inside an async context; use `spawn_blocking` for WinRT calls that block.
- Logging: `tracing`. No `println!`.
- Windows-only code goes behind `#[cfg(windows)]`, with a stub for other platforms so `cargo check` passes on every OS.
- UI: Svelte 5 + TypeScript in `ui/`. Types for IPC are **generated** from Rust (`ts-rs`) into `ui/src/lib/bindings/`. Never hand-edit that folder.
- Tests: unit tests next to the code; anything that needs Spotify or the network goes behind `#[ignore]`, with a comment saying how to run it.
