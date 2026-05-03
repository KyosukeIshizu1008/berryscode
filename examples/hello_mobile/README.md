# Hello Mobile

Reference sample for BerryCode v0.7.5+. A minimal Bevy 0.18 + avian3d
project that demonstrates the runtime shape BerryCode emits into a
real game.

## What's in here

- **Player capsule** with `RigidBody::Dynamic` + `Collider::capsule`,
  `LockedAxes::ROTATION_LOCKED` so it stays upright.
- **PlayerController** component with `speed` / `jump_velocity` /
  `run_multiplier` / `turn_speed` — same shape the editor's
  Inspector exposes.
- **AnimatorParams** receiving:
  - `isMoving` bool (driven by arrow-key input via
    `drive_animator_params`)
  - `jump` Trigger (driven by either the Space key or the
    on-screen virtual button via `touch_input_evaluate`)
- **TouchInputZone** in the bottom-right corner — Trigger action so
  each tap fires the `jump` parameter once. Works with mouse on
  desktop (left-click + hold) and with real touch on mobile.
- **FollowCamera** that mirrors the v0.6.x display profile preview
  — the same top-down framing is what the iPhone / iPad display
  profiles draw inside the editor.

## Run it

```bash
cargo run -p hello_mobile
```

Arrow keys to move, Shift to run, Space or click the bottom-right
button to jump.

## Open it in BerryCode

`File → Open Folder → /Users/<you>/berryscode/examples/hello_mobile`.
The editor will:

1. Discover `src/scenes/*.bscene` files (none yet — this sample
   spawns its scene from `main.rs` to stay self-contained;
   normally BerryCode generates `src/scenes/scene.rs` from the
   .bscene file).
2. Show the `PlayerController` / `TouchInputZone` /
   `AnimatorParams` types in the Inspector when an entity carries
   them.
3. Letterbox the Scene View to your chosen mobile profile so you
   can verify the `TouchInputZone` rect lands inside the safe
   area.

## Build for mobile

See the top-level [`MOBILE.md`](../../MOBILE.md) for the full
recipe. tl;dr:

```bash
rustup target add aarch64-apple-ios-sim aarch64-apple-ios aarch64-linux-android
cargo build -p hello_mobile --target aarch64-apple-ios-sim --release
cargo build -p hello_mobile --target aarch64-linux-android --release
```

iOS / Android wrapping (Xcode / Gradle project trees) lands in v0.7.6+.
