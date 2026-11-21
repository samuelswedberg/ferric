<!-- Title: Conventional Commit form, e.g. "feat(media): watch Spotify via SMTC" -->

## What

<!-- What this PR does, in a sentence or two. If it completes a task: "Closes task Txx". -->

## How I verified it

<!-- One line per acceptance criterion from the task file, or what you tested by hand. -->

- [ ]

## Not verified / needs follow-up

<!-- Anything you couldn't test (needs Premium, needs Spotify running, other display scaling), contract changes you need, bugs spotted in other areas. -->

## Checklist

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `npm run check --prefix ui` and `npm run build --prefix ui`
- [ ] Only touched paths this change owns (see `docs/tasks/README.md`)
