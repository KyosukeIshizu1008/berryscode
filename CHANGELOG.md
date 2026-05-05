# Changelog

All notable changes to BerryCode are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once
v1.0 ships — until then, **breaking changes can land on minor bumps** and
will be called out explicitly under the relevant version below.

For older releases (pre-v0.7.11), see the
[GitHub Releases](https://github.com/KyosukeIshizu1008/berryscode/releases)
page — every release on Releases ships with a tagged binary, signature, and
release notes.

## [Unreleased]

## [0.8.0] — 2026-05-05

> **Note:** the Windows artifact in this release is unsigned because the
> SignPath organization configuration is in flux. Windows users may see
> an "Unknown publisher" SmartScreen warning. Code-signing will return
> in v0.8.1.

### Added

- Read-only Godot project support (v0.8.x Migration & interop track):
  - `godot_import` module parses `project.godot` and `.tscn` files into a
    typed `GodotProject` / `GodotScene` tree, recovering section header
    attributes (`name=`, `type=`, `parent=`) that
    `godot-properties-parser` discards.
  - "Godot Scene" floating panel auto-opens when the active editor tab is
    a `.tscn` file. Tree on top, properties below. Cached against
    `(path, mtime)` so the parser doesn't re-run every frame.
  - Syntax highlighting for `.gd` (GDScript: `extends`, `@annotations`,
    SCREAMING_SNAKE constants) and `.tscn` / `.tres` / `project.godot`
    (section headers, `Vector2(...)` / `ExtResource(...)` calls,
    semicolon line comments).
  - File-tree icons: `.gd` blue, `.tscn` purple, `.tres` teal,
    `project.godot` gear icon.
  - "● Godot Project" status-bar badge whenever the workspace contains a
    `project.godot`.
- `AGENTS.md` mirrors the operating rules already in `CLAUDE.md` for
  agents that follow the Codex documentation contract.
- Activity Bar panel visibility settings (`Settings → Workbench → Activity
  Bar`). Per-panel checkboxes hide panels the user doesn't need; choices
  persist to `~/.berrycode/panels.json`. Database, Docker, and OracleBerry
  default to off so the left strip stays compact for new users.
- Opt-out `ai` Cargo feature (default on). `cargo build --no-default-features`
  now produces a binary with no AI chat / OracleBerry / agent code, for
  users who want a leaner build. (#21, contributor: @pragma-twice)

### Changed

- UI palette consolidated into `app::ui_colors`. Ad-hoc `Color32` literals
  across the header, editor tabs, dock, status bar, search, scene editor
  hierarchy, and settings now route through named constants
  (`ACTIVITY_BAR_BG`, `STATUS_BAR_BG`, `HOVER_BG`, `ACTIVE_BG`, `ACCENT`,
  `ACCENT_HOVER`, `FOCUS_BORDER`, …) so the VS Code Dark+ palette is
  sourced from one place. Also tightens chrome text sizing to VS Code
  defaults (13px body, 12px controls, 11px small, 16px headings) and
  rebuilds the search-option toggles (`Aa` / `ab` / `.*`) as
  custom-painted chips. (#20)
- README roadmap section restructured. The old per-version bullet list
  (~185 lines) is replaced with a 4-row phase table, a compact "Shipped"
  summary, the current v0.8 focus split into already-in-main vs v0.8.x
  roadmap, the new "Migration & interop" parallel track, and a 6-row
  "Future" table. Roughly one-third the size.
- README install tables now match the actual release artefacts. macOS
  ships as `BerryCode-<version>-macOS-universal.dmg` (not `.tar.gz` as
  the previous table claimed); Linux / Windows artefact names made
  version-explicit.
- Build instructions no longer reference the (removed) `berry_api/`
  subcrate. AI features are now built into the binary; bring your own
  key in Settings (`Cmd+,`).
- AI chat panel pulls a couple of magic colour values up to
  `ui_colors::SIDEBAR_BG` / `ui_colors::TEXT_MUTED` so theme drift is
  harder to introduce.
- Workspace `version` aligned to `0.8.0` (was stale at `0.2.0`); Snap
  and Flatpak manifests refreshed to v0.7.11 sources with correct
  sha256.
- Flatpak app-id renamed `com.berrycode.BerryCode` →
  `dev.berrycode.BerryCode` so the manifest, the README install
  instructions, and the filename all agree.

### Fixed

- Mobile Toolchain panel no longer freezes the UI. The previous code
  re-spawned `cargo mobile --version` on every render frame (~100–500ms
  per spawn). The probe result is now cached in `Option<bool>` and
  invalidated only on Refresh / one-click session end.
- Search panic when the input contained an unfinished regex character
  class; UI freezes triggered by long-running synchronous work in the
  status bar and AI chat panels. (P1 + P2 fixes from external review.)
- Seven additional issues from external code review (P1 + P2 + P3) and
  the clippy regression gate restored to green.

## [0.7.11] — 2026-05-03

Last release before the v0.8 line. The full v0.7 cycle is summarised in
the README's "✅ Shipped" section; per-release notes live on
[GitHub Releases](https://github.com/KyosukeIshizu1008/berryscode/releases).

### Highlights from the v0.7 cycle

- **v0.7.0** — iOS / Android build pipeline scaffold (Phase E start).
- **v0.7.2 / 0.7.3 / 0.7.4** — Android AAB / Play Console scaffolds,
  Xcode + Gradle project skeleton emitters, Build All wiring.
- **v0.7.5** — `examples/hello_mobile/` reference sample.
- **v0.7.6** — MIGRATION.md + MOBILE.md docs polish.
- **v0.7.7** — backfill mobile-test gaps from v0.6.x.
- **v0.7.8** — one-click iOS / Android run via cargo-mobile2.
- **v0.7.9** — Show Colliders toggle in Scene View toolbar.
- **v0.7.10 / 0.7.11** — font fallback for Japanese on
  Windows + Linux, FileWatcher live-refresh, `rest_client` realigned
  to berry-core-api 0.1.0, Windows ZIP code-signing via SignPath, and
  a Windows compile fix in the font lookup loop.

## [0.6.x] — 2026-05-03

- **v0.6.0** — mobile / tablet display preview in Scene View.
- **v0.6.1** — `TouchInputZone` component for mobile virtual buttons.

## [0.5.x] — 2026-04 → 2026-05

The v0.5 cycle landed the Inspector + Animator + asset-import depth.
See [GitHub Releases](https://github.com/KyosukeIshizu1008/berryscode/releases?q=v0.5)
for per-release notes.

### Highlights

- **v0.5.5** — emit avian3d physics from Collider / RigidBody components.
- **v0.5.6 / 0.5.7** — `PlayerController` component + console panel
  consolidation; `turn_speed` + `move_speed` parameters.
- **v0.5.10** — Animator runtime FSM (Unity-style).
- **v0.5.12** — drag-and-drop assets onto Inspector path fields.
- **v0.5.13 / 0.5.14** — Animator `OnComplete` transition; blend tree
  first-clip fallback in animator codegen.
- **v0.5.15** — Scene / Game view toggle in the Scene Editor.
- **v0.5.16** — Physic Material asset reference on Collider.
- **v0.5.17** — VS Code-style Inspector pass.

## Earlier releases

For everything before v0.5, see
[GitHub Releases](https://github.com/KyosukeIshizu1008/berryscode/releases).

[Unreleased]: https://github.com/KyosukeIshizu1008/berryscode/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/KyosukeIshizu1008/berryscode/releases/tag/v0.8.0
[0.7.11]: https://github.com/KyosukeIshizu1008/berryscode/releases/tag/v0.7.11
