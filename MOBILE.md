# Mobile build pipeline

End-to-end notes for shipping a BerryCode project to the App Store and
Google Play. Status reflects v0.7.2; gaps are tracked in the editor's
Build Settings panel.

## Targets

| Platform | Target triple | Status |
|----------|---------------|--------|
| iOS Simulator | `aarch64-apple-ios-sim` | ✅ `cargo build` wired |
| iOS Device | `aarch64-apple-ios` | ✅ `cargo build` wired, IPA codesign in v0.7.3 |
| Android | `aarch64-linux-android` | ✅ `cargo build` wired, AAB + signing in v0.7.3 |
| visionOS | `aarch64-apple-visionos` | 🟡 v0.9.x |
| Meta Quest | `aarch64-linux-android` (Quest mode) | 🟡 v0.9.x |

Install the rustup target before building:

```bash
rustup target add aarch64-apple-ios-sim aarch64-apple-ios aarch64-linux-android
```

## Build Settings

Open `File → Build Settings` and fill the relevant rows:

- **iOS** — `Bundle ID` (e.g. `com.example.myapp`), `Team ID` (10-char
  Apple developer ID).
- **Android** — `Package name`, `Keystore` path, `Key alias`.
- **Play Console** — `Service account JSON` path
  (Play Console → API access).

Settings are stored in `<project>/build_settings.ron` and committed with
the project so teammates don't re-enter them.

## Workflow

1. **Author the scene** in BerryCode's Scene Editor. Any
   `TouchInputZone` components drive `AnimatorParams` from the user's
   touch / mouse position.
2. **Preview the layout** by switching the Scene View toolbar's display
   profile to iPhone / iPad / Android — safe-area bands appear at the
   notch / home-indicator regions.
3. **Cmd+S** to save → BerryCode regenerates `src/scenes/` with the
   shared `PlayerController`, `AnimatorRuntime`, `TouchInputZone`, and
   `AnimatorParams` types in `mod.rs`.
4. **Build** via `Build Settings → Build`:
   - Desktop: produces a release binary in `target/release/`.
   - iOS Simulator / Device: produces a binary at
     `target/<triple>/release/`. Until v0.7.3 wraps `xcodebuild`,
     drag the binary into a manual Xcode shell project to assemble
     the `.app` bundle for the simulator or to sign for distribution.
   - Android: produces a `.so` at `target/aarch64-linux-android/release/`.
     v0.7.3 wraps Gradle to package it into an AAB; until then run
     `cargo apk build --release` from the project root (requires
     `cargo install cargo-apk`).
5. **Ship**:
   - **App Store** — `xcrun altool --upload-app --type ios --file
     YourApp.ipa --username APPLE_ID --password APP_SPECIFIC_PW`.
     `play_console::upload_ipa` returns this command verbatim with
     your fields filled in for now.
   - **Play Console** — see the curl recipe `play_console::upload_aab`
     emits, or upload manually via the Play Console UI.

## Compatibility cheat-sheet

| Scene component | Mobile-ready? | Notes |
|-----------------|---------------|-------|
| `TouchInputZone` | ✅ | Falls back to mouse on desktop. |
| `PlayerController` | ✅ | `turn_speed` keeps the avatar facing the input direction. |
| `Animator` (`.banimator`) | ✅ | FSM evaluates each frame. |
| `Collider` + `RigidBody` | ✅ | avian3d ships on every target. |
| `MeshFromFile` (GLB) | ✅ | Animation clips auto-loaded. |
| `Skybox` | 🟡 | HDR loads on desktop; mobile GPU support varies. |
| `ParticleEmitter` | 🟡 | Performance not yet tuned for mobile. |

## Roadmap (post-v0.7.2)

- v0.7.3 — Xcode project + Gradle wrapper auto-emit. Real `xcrun altool`
  + Play Console REST upload (no more curl recipes).
- v0.7.4 — Cross-platform "Build All" wires the desktop pipeline into
  `mobile::build_all` so one click produces every artifact.
- v0.7.5 — `examples/hello_mobile/` complete walking-fox sample with a
  PlayerController, Animator, and on-screen joystick.
- v0.8.x — Asset variants (`@2x` / `_low` automatic resampling) + per-
  platform physics simulation rate presets.
- v0.9.x — visionOS / Quest, OpenXR scene preview.
- v1.0 — Full migration guide + Bevy 0.18 → 0.19 upgrade notes.
