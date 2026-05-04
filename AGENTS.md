# AGENTS.md

Guidance for Codex working in this repo. Keep this file short — codebase facts belong in code, README, or commit messages, not here. Only document non-obvious gotchas and conventions an agent can't infer in 30 seconds.

## What this is

BerryCode is a native IDE built specifically for the Bevy game engine. Same stack as the games it edits: **Bevy 0.18 + bevy_egui 0.39 + egui 0.33 + WGPU**. Two binaries ship from the same `berrycode` crate:

- `berrycode` (`src/main.rs`) — the production binary
- `berrycode-egui` (`src/bin/berrycode-egui.rs`) — a thinner harness used in dev / testing

Both wire up `BerryCodePlugin` from `src/bevy_plugin.rs`. **Add new app-level systems to the plugin, not to either bin.** Otherwise `berrycode-egui` silently misses behavior.

## Build / run

```bash
cargo run --bin berrycode               # debug
cargo build --release --bin berrycode   # release
cargo check -p berrycode                # fast iteration when iterating UI code
```

The dev build is large; `cargo check` is the right loop for editing egui UI. The app uses a PID lockfile at `~/Library/Caches/berrycode.lock` (macOS) — if the binary crashes mid-run, delete it before relaunching.

## Operating rules (read first)

These exist to stop the "止まった?" feedback loop. Follow them by default; only break them if the user explicitly asks.

### Never let the user lose the thread

- **Always announce intent before the first tool call.** When the user asks for something, the first thing in the reply is one line stating what you're about to do — not silence followed by tool calls. The user must never see "thinking…" with no follow-up text.
- **Every assistant turn must end with one of three things, never silence:** (a) a concrete result statement, (b) a single specific question that unblocks you, or (c) "next: <X>" when you're handing off to a tool that fires asynchronously. Never end with "確認待ちです" alone — pair it with the specific thing you need.
- **Don't send "ビルド待ち" / "monitoring" filler.** If you've armed a Monitor, the next message should come from the Monitor event, not from you. Filler messages create the appearance of stalls.

### Batching and momentum

- **Batch related edits.** When editing the same file 3+ times in a row, do it inside a single assistant turn — no narration between them. For >5 edits to one file or pervasive structural changes, prefer a single Write.
- **Until-done by default.** When the user says やって / 進めて / 全部直して / これも / 同じやり方で, complete edit → verify → rebuild → relaunch in one turn without checking in mid-flow.
- **One restart per change cycle.** Never `pkill + cargo run` more than once for the same logical change.

### Tool-call ergonomics

- **Monitor timeout default: 60 seconds.** Never use 120s+ unless the cargo build is genuinely slow (cold start, new dependency). Long timeouts produce dangling "Monitor timed out" notifications that read as stalls.
- **Trust `cargo check`, not rust-analyzer diagnostics.** rust-analyzer's `<new-diagnostics>` blocks are often stale right after an edit. Run `cargo check -p berrycode` to confirm; only react to its output.
- **Stale rust-analyzer warnings don't deserve a reply.** If `<new-diagnostics>` shows the same dead-code warnings you've already seen, ignore them silently. Don't quote them back.

### What not to do

- **No mid-flow recap.** Don't summarise what was just done unless the user asks; the diff and the working app are the report.
- **No "止まってない、続行中" reassurance posts.** If the user asks "止まった?" the answer should be a tool call (the next concrete step), not a paragraph explaining you weren't stopped.

## Gotchas you will hit

1. **`PrimaryEguiContext` is spawned exactly once — in the plugin.** `setup_primary_camera` in `bevy_plugin.rs` spawns the window-targeted `Camera2d` with `RenderLayers::none()`, `order: 100`, and `ClearColorConfig::None`. Do **not** also spawn one from `main.rs` — `setup_egui_fonts_and_style` queries with `With<PrimaryEguiContext>` and panics on multiple matches with `Multiple entities fit the query`.

2. **Bevy's default `ClearColor` is pure black.** The egui camera doesn't clear (`ClearColorConfig::None`), so any sub-pixel seam between `SidePanel`s shows through as a black band. The plugin sets `ClearColor` to `#191A1C` (`SIDEBAR_BG`) to hide this. Don't remove it.

3. **`SidePanel` width persists in egui memory by id.** If you change `default_width` / `width_range` / sizing logic for an existing panel, also bump the id (`"sidebar"` → `"sidebar_v2"`). Otherwise users will keep stale persisted widths that may sit outside the new range. Current ids in use: `sidebar_v2`, `ai_chat_panel_v2`, `activity_bar`.

4. **Don't read `ui.available_width()` back into the value driving `exact_width`.** That creates a feedback loop where the panel shrinks by `inner_margin` every frame until clamped. Use `default_width` + `width_range` for resizable panels.

5. **`SidePanel::left("activity_bar")` is `exact_width(48)`** — the leftmost icon strip. The next `SidePanel::left` (the file tree / search / git panel) sits to its right with id `sidebar_v2`. Both Explorer and Search render *inside* the same `sidebar_v2` panel, so they always share width by construction.

6. **Codicon glyph codepoints are NOT what their names suggest from memory.** `\u{eacd}` is `dashboard`, not `database`. `\u{eb24}` is `mute`, not `server-environment`. Always verify codepoints by extracting from `assets/codicon.ttf` via fonttools before adding a new icon. The verified pattern is in the v0.5.x history.

## Layout / file map

- `src/app/mod.rs` — central `BerryCodeApp` struct + `berry_ui_system` (the per-frame egui pass). Big file (~3000 lines); use Read with `offset`/`limit`.
- `src/app/{sidebar,ai_chat,header,editor,file_tree,search,git,...}.rs` — one panel each.
- `src/bevy_ide/inspector/` — BRP client (talks to a running Bevy app).
- `src/app/scene_editor/` — Unity-style scene editor (3D viewport, hierarchy, inspector, gizmos).
- `src/native/` — non-egui subsystems: search (Rayon), LSP client, etc.
- `src/syntax.rs` + `src/tree_sitter_engine.rs` — syntax highlighting.
- `src/buffer.rs` — Ropey-backed text buffer.
- `berry_api/` — separate workspace member; gRPC server for AI chat. Only needed for the AI features.

UI colors live in `app::ui_colors` (`mod.rs`). Reuse those constants instead of literal RGB.

Buttons go through `app::button_style` helpers (`primary_button`, `danger_button`, `icon_button`, `button_with_icon`) so the VS Code Dark+ palette stays consistent. Don't `egui::Button::new(...).fill(...)` ad-hoc.

## Conventions

- **Always run `cargo fmt --all` before committing.** Linux CI runs `cargo fmt --all -- --check` and rejects unformatted pushes. The repo ships a `.githooks/pre-push` that enforces this — activate it once per clone with `git config core.hooksPath .githooks`.
- **No `Co-Authored-By: Codex` / `Generated with Codex` lines in commit messages.** The user enforces this.
- **Prefer editing existing modules over creating new ones.** The repo already has a panel for everything; new files should be a last resort.
- **Don't leave breadcrumb comments** (`// removed by Codex`, `// TODO: was X`). Just delete the old code.
- **Run the binary before claiming a UI change is done.** `cargo check` doesn't catch layout regressions or texture upload issues.
