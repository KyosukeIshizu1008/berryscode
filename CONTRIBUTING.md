# Contributing to BerryCode

Thanks for your interest in helping. BerryCode is a small project run by a
single maintainer right now; the bar for contributions is "**will this
make the IDE noticeably better for Bevy users?**" — not "is this
technically clean."

## TL;DR

```bash
git clone https://github.com/KyosukeIshizu1008/berryscode
cd berryscode
git config core.hooksPath .githooks   # one-time, enables the pre-push fmt check
cargo run --bin berrycode
```

Open an issue before doing significant work. Small fixes can go straight
to a PR.

## Where to ask questions

- **Discord** — https://discord.gg/u5VYs7za, the fastest path. Ping in
  `#help` or `#dev`.
- **GitHub Discussions** — for design conversations that should be
  searchable later.
- **Issues** — for actual bugs and concrete feature requests.

## Setting up the dev environment

### Prerequisites

- Rust 1.75 or newer (`rustup`).
- macOS / Linux / Windows are all supported. On Linux you'll additionally
  need `libx11-dev libasound2-dev libudev-dev libpipewire-0.3-dev` (or
  your distro's equivalent).

### First run

```bash
cargo run --bin berrycode               # debug build, slower but iterative
cargo build --release --bin berrycode   # release build, ~5–10 min cold
cargo check -p berrycode                # fast loop while iterating egui code
```

The dev binary is large; `cargo check` is the right loop for editing UI
code. The release build is what you should use for screen recording and
performance testing.

### Useful shortcuts

The `~/Library/Caches/berrycode.lock` PID file is created on launch. If
the binary crashed mid-run, delete that file before relaunching.

## Project layout

| Directory | What's in it |
|-----------|--------------|
| `berrycode/src/app/` | Bevy + egui UI (panels, editor, git, terminal, …) |
| `berrycode/src/bevy_ide/` | Bevy-specific subsystems (BRP inspector, scene preview) |
| `berrycode/src/bevy_plugin.rs` | The plugin that wires up everything |
| `berrycode/src/native/` | Non-egui subsystems (search, LSP, fs) |
| `berrycode/src/syntax.rs` + `tree_sitter_engine.rs` | Syntax highlighting |
| `berrycode/src/buffer.rs` | Ropey-backed text buffer |
| `berrycode/src/godot_import/` | Read-only Godot project loader |

Bigger files (e.g. `app/mod.rs`) are >2k lines — use Read with
`offset` / `limit` rather than reading the whole thing.

## Code style

### Formatting

`cargo fmt --all` before every commit. The Linux CI runs
`cargo fmt --all -- --check` and rejects unformatted pushes.

The repo ships a pre-push hook at `.githooks/pre-push`. Activate it once
per clone:

```bash
git config core.hooksPath .githooks
```

### Linting

We don't run clippy in CI yet. If you fix a clippy warning, do it in a
focused commit so it's easy to review.

### Comments

Comments should explain **why** non-obvious code exists, not **what** it
does. The diff already shows what changed; the comment is for the reader
in two years who wonders why a workaround is shaped a particular way.

If a piece of code is the workaround for a specific upstream bug, link
the issue. If it's the workaround for a known platform quirk
(e.g. macOS IME edge case), say so.

### Tests

- `cargo test -p berrycode` for unit tests.
- New behaviour should ship with at least one test that would catch the
  regression next time someone refactors the area.
- Tests that depend on filesystem fixtures (e.g. `/tmp/godot_sample`)
  should `return` early if the fixture isn't present, so CI doesn't
  flake on environments without it.

## Commit message conventions

We use a Conventional Commits-style prefix:

| Prefix | When to use |
|--------|-------------|
| `feat:` / `feat(scope):` | A user-visible new feature |
| `fix:` / `fix(scope):` | A user-visible bug fix |
| `chore:` | Tooling, repo metadata, anything that doesn't change behaviour |
| `docs:` | README / CHANGELOG / inline doc comments |
| `test:` | Tests-only commits |
| `ci:` | GitHub Actions workflow changes |

Version bumps live in the commit subject when relevant:

```
feat: v0.8.0 — Godot project read-only viewer (Migration & interop)
```

The body should explain the **why**, not just restate the title. See
recent commits in `git log` for examples of the level of detail
expected on substantial features.

**Do not** include `Co-Authored-By: Claude` or
`Generated with Claude Code` lines. AI tooling is used freely in this
repo, but commit attribution stays human.

## Pull request process

1. **Discuss before large work.** For anything >100 lines or that
   reshapes a subsystem, open an issue or post in Discord first. It's
   embarrassing to merge two competing implementations of the same
   feature.
2. **Keep PRs focused.** One concern per PR. If you find an unrelated
   bug while working, fix it in a separate PR.
3. **CI must pass.** macOS / Linux / Windows checks run on every PR.
   `cargo fmt --all -- --check` is enforced; the rest are advisory but
   should be green before requesting review.
4. **Describe what to verify.** PR description should include "to
   verify, do X, Y, Z" — not just the diff. Reviewers don't have time
   to figure out the mental model from the patch alone.
5. **Respond to review.** Even a quick "addressed" or "intentional
   because Z" comment is better than silence.

## What we're NOT looking for

- **Drive-by reformatting.** If your PR's diff is 80% whitespace and 20%
  substantive, we'll reject it and ask for the substantive part alone.
- **Stylistic Rust nitpicking.** If the existing code has been in the
  tree for a while and works, we generally don't have the bandwidth to
  re-bikeshed it.
- **New language support.** The IDE is "Bevy-specialised" by design;
  adding C# / Python / TypeScript LSP integration would dilute that
  position. (GDScript syntax highlighting is the exception because it's
  read-only Godot project support, not a development environment.)
- **Cosmetic theme PRs without a reason.** Colours have been picked to
  read consistently with VS Code Dark+; one-off palette changes are
  unlikely to merge unless they fix a contrast / accessibility problem.

## Licensing

BerryCode is dual-licensed under MIT and Apache-2.0. By submitting a
contribution you agree your work can be used under either license. We
don't require a CLA today; if that ever changes, all current
contributors will be notified.

## Code of Conduct

Be kind, be specific, assume good faith. If something feels off — a
review tone, a Discord exchange, anything — DM the maintainer
(`@KyosukeIshizu1008`) on Discord or email
ishizu@oracleberry.co.jp.

We'll add a more formal Code of Conduct (Contributor Covenant or
similar) once the contributor base grows. Until then, the rule above
is the rule.
