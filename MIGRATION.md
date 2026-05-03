# Migration guide: v0.5 → v0.7

What changed across the 0.5 → 0.7 line and how to update an existing
BerryCode project. Bug-fix patch releases (0.5.18, 0.7.1) are skipped
when their changes are transparent.

## TL;DR

1. Re-save every `.bscene` once after upgrading. Codegen output
   gained new `pub` types and serde-default fields; older `.bscene`
   files load fine but the generated `src/scenes/` won't get the
   new shape until you save.
2. If your `main.rs` referenced any `crate::scenes::scene::*`
   helpers, switch to `crate::scenes::*` for the project-wide types
   (`PlayerController`, `AnimatorParams`, `AnimatorRuntime`,
   `TouchInputZone`, …). Per-scene-prefixed types
   (`<Pascal>PendingGlbAnim`, `<Pascal>GlbAnimGraphs`) are still
   per-scene.
3. Add `avian3d = "0.6"` to your project's `Cargo.toml` and
   register `PhysicsPlugins::default()` in `App::new()` if you
   use `Collider` / `RigidBody` from the editor (since v0.5.5).
4. For mobile builds, install the rustup target for your platform
   — see [`MOBILE.md`](./MOBILE.md).

---

## v0.5.x → v0.5.6 — PlayerController + Animator FSM

- New `ComponentData::PlayerController { speed, jump_velocity,
  run_multiplier, turn_speed }` (v0.5.7 added `turn_speed`).
- New `ComponentData::Animator { controller_path }` reads a
  `.banimator` file at codegen time and emits the FSM.

**Action**: replace any hand-rolled `Player` `CustomScript` with
the built-in `PlayerController`. Move animation switching out of
`main.rs` and into a `.banimator` file authored via the Animator
Editor (Inspector → Animator → Open Editor).

## v0.5.5 — avian3d physics

- `Collider` / `RigidBody` components now codegen to real
  `avian3d::prelude::*` components instead of comment stubs.

**Action**:
```toml
# Cargo.toml
[dependencies]
avian3d = "0.6"
```
```rust
// main.rs
use avian3d::prelude::*;
App::new().add_plugins(PhysicsPlugins::default());
```

## v0.5.9 — `pub` GLB anim runtime types

- `<Pascal>PendingGlbAnim`, `<Pascal>GlbAnimEntry`,
  `<Pascal>GlbAnimGraphs` are emitted with `pub` so user code can
  reach into the runtime animation state.

**Action**: nothing required. Open up if you want a custom
clip-switcher in `main.rs`.

## v0.5.16 — Physic Material asset

- `Collider.physic_material_path: Option<String>` references a
  `.bphysmat` file (RON `( friction: f, restitution: f )`).
  Codegen reads the file at save time and emits its values into
  avian's `Friction::new` / `Restitution::new`.

**Action**: optional. Older Colliders still emit their inline
values when no path is set.

## v0.6.0 — Mobile display preview

- Scene View toolbar gains a `DisplayProfile` selector
  (iPhone / iPad / Android Phone / Tablet).
- Safe-area bands overlay the notch + home indicator.

**Action**: nothing required — it's a viewport-only feature.

## v0.6.1 — TouchInputZone

- New `ComponentData::TouchInputZone { x, y, w, h, parameter_name,
  action_kind, label }`. Codegen emits a runtime
  `touch_input_evaluate` system that reads `bevy::input::touch::Touches`
  + the mouse cursor and updates `AnimatorParams`.

**Action**: nothing required. To use, add the component in the
Inspector, point it at an Animator parameter name, and let the
runtime drive the FSM transition.

## v0.7.0 — iOS / Android cargo build dispatch

- Mobile `Platform` selections in Build Settings no longer error;
  they spawn `cargo build --target <triple>` via
  `app::mobile::packager`.

**Action**: install the rustup targets:
```bash
rustup target add aarch64-apple-ios-sim aarch64-apple-ios aarch64-linux-android
```

## v0.7.2 — AAB / IPA / Play Console scaffolds

- `BuildSettings` gained iOS + Android signing fields:
  `ios_bundle_id`, `ios_team_id`, `android_package_name`,
  `android_keystore_path`, `android_key_alias`,
  `play_console_service_account_path`. All default to empty;
  fill them via `File → Build Settings`.
- `mobile::play_console::upload_aab` / `upload_ipa` return
  copy-paste curl / `xcrun altool` commands until the full
  OAuth2 + multipart flow lands.

**Action**: nothing forced. Set the fields when you're ready
to ship.

## v0.7.3 — Project skeleton emitters

- `mobile::project_emit::emit_xcode_project` writes a minimal
  Swift / `Info.plist` / `project.yml` tree.
- `mobile::project_emit::emit_gradle_project` writes
  `settings.gradle`, `build.gradle`, `AndroidManifest.xml`.
- Both are idempotent — user edits to existing files are
  preserved.

**Action**: nothing required. Run them via Build Settings when
you want a starter project.

## v0.7.4 — Build All cross-platform

- `mobile::build_all::build_all(root, &platforms)` now wires
  desktop + Web through `cargo build` too. One call ships
  every artifact.

**Action**: optional — your existing per-platform Build flow
still works.

## v0.7.5 — `examples/hello_mobile/`

- New reference sample at `examples/hello_mobile/` shows the
  runtime shape BerryCode emits (PlayerController +
  AnimatorParams + TouchInputZone + FollowCamera) without
  pulling in the editor.

**Action**: optional. Use as a starting template:
`cp -r examples/hello_mobile <your-project-name>`.

---

## Bevy version

Targeting Bevy `0.18` for the entire 0.5–0.7 line. Upgrading to
Bevy 0.19 lands in v1.0 along with the Bevy upgrade guide.

## avian3d version

Targeting avian3d `0.6`. The 0.7 release will track whichever
avian release pairs with Bevy 0.19.
