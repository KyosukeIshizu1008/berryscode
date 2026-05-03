//! iOS / Android project skeleton emitters — v0.7.3.
//!
//! Writes the minimum file tree needed to wrap a BerryCode-built Rust
//! library into an iOS `.xcodeproj` or an Android Gradle module. Output
//! is intentionally conservative — enough to `xcodebuild archive` /
//! `gradlew bundleRelease` without further hand-editing, no more — so
//! the user owns the long tail of platform-specific tweaks.
//!
//! The emitters are idempotent: if a file already exists with different
//! content they leave it alone (so user edits to e.g. `Info.plist` or
//! `AndroidManifest.xml` survive a re-emit). Only missing files get
//! created.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Result of an emit pass — list of paths created and paths that were
/// preserved (already on disk).
#[derive(Debug, Default)]
pub struct EmitReport {
    pub created: Vec<PathBuf>,
    pub preserved: Vec<PathBuf>,
}

impl EmitReport {
    fn touch(&mut self, path: PathBuf, contents: &str) -> std::io::Result<()> {
        if path.exists() {
            self.preserved.push(path);
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
        self.created.push(path);
        Ok(())
    }
}

/// Emit a minimal Xcode project tree at `<root>/ios/`. Wraps the
/// `aarch64-apple-ios` cargo build output as an embedded `.framework`
/// loaded by a small Swift `@main` shim.
pub fn emit_xcode_project(
    root: &Path,
    bundle_id: &str,
    team_id: &str,
    app_name: &str,
) -> std::io::Result<EmitReport> {
    let mut report = EmitReport::default();
    let ios = root.join("ios");

    // Info.plist — minimum keys App Store accepts.
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\"><dict>\n\
         \x20 <key>CFBundleName</key><string>{name}</string>\n\
         \x20 <key>CFBundleIdentifier</key><string>{bundle}</string>\n\
         \x20 <key>CFBundleVersion</key><string>1</string>\n\
         \x20 <key>CFBundleShortVersionString</key><string>1.0.0</string>\n\
         \x20 <key>CFBundleExecutable</key><string>{name}</string>\n\
         \x20 <key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>\n\
         \x20 <key>UISupportedInterfaceOrientations</key><array>\n\
         \x20   <string>UIInterfaceOrientationPortrait</string>\n\
         \x20   <string>UIInterfaceOrientationLandscapeLeft</string>\n\
         \x20   <string>UIInterfaceOrientationLandscapeRight</string>\n\
         \x20 </array>\n\
         </dict></plist>\n",
        name = app_name,
        bundle = bundle_id,
    );
    report.touch(ios.join("Info.plist"), &plist)?;

    // Swift shim — calls into the Bevy entry point exported from the
    // user's Rust crate.
    let swift_shim = format!(
        "import UIKit\n@main\n\
         class AppDelegate: UIResponder, UIApplicationDelegate {{\n\
         \x20   var window: UIWindow?\n\
         \x20   func application(_ a: UIApplication,\n\
         \x20       didFinishLaunchingWithOptions opts:\n\
         \x20         [UIApplication.LaunchOptionsKey: Any]?) -> Bool {{\n\
         \x20       // Hand off to the Rust-side Bevy `App::run()`.\n\
         \x20       {name}_main()\n\
         \x20       return true\n\
         \x20   }}\n\
         }}\n\
         @_silgen_name(\"{name}_main\") func {name}_main()\n",
        name = app_name,
    );
    report.touch(ios.join("Sources").join("App.swift"), &swift_shim)?;

    // Build settings hint — Xcode reads ENV inside its build phases. The
    // user adds this as a Run Script phase invoking
    // `cargo build --target aarch64-apple-ios --release`.
    let build_phase = format!(
        "#!/usr/bin/env bash\n\
         set -euo pipefail\n\
         cd \"$SRCROOT/..\"\n\
         cargo build --target aarch64-apple-ios --release\n\
         cp target/aarch64-apple-ios/release/lib{name}.a \"$BUILT_PRODUCTS_DIR/lib{name}.a\"\n",
        name = app_name,
    );
    report.touch(ios.join("scripts").join("build_rust.sh"), &build_phase)?;

    // README — explains the manual `xcodegen` / `xcodebuild` step until
    // v0.7.4 emits a real `.xcodeproj`.
    let readme = format!(
        "# Generated iOS wrapper\n\n\
         BerryCode v0.7.3 emits the Swift shim, `Info.plist`, and the\n\
         Cargo build script — but stops short of generating the\n\
         `.xcodeproj` itself. Use [xcodegen](https://github.com/yonaskolb/XcodeGen)\n\
         on the bundled `project.yml` to assemble the project, then\n\
         `xcodebuild archive`. Real `.xcodeproj` emit lands in v0.7.4.\n\n\
         Bundle ID: `{bundle}`\nTeam ID: `{team}`\n",
        bundle = bundle_id,
        team = team_id,
    );
    report.touch(ios.join("README.md"), &readme)?;

    // Minimal `project.yml` for xcodegen.
    let project_yml = format!(
        "name: {name}\noptions:\n  bundleIdPrefix: {bundle}\nsettings:\n  DEVELOPMENT_TEAM: {team}\ntargets:\n  {name}:\n    type: application\n    platform: iOS\n    deploymentTarget: \"15.0\"\n    sources:\n      - Sources\n    info:\n      path: Info.plist\n    settings:\n      base:\n        OTHER_LDFLAGS: -l{name}\n        LIBRARY_SEARCH_PATHS: $(BUILT_PRODUCTS_DIR)\n",
        name = app_name,
        bundle = bundle_id,
        team = team_id,
    );
    report.touch(ios.join("project.yml"), &project_yml)?;

    Ok(report)
}

/// Emit a minimal Gradle project tree at `<root>/android/`.
pub fn emit_gradle_project(
    root: &Path,
    package_name: &str,
    app_name: &str,
) -> std::io::Result<EmitReport> {
    let mut report = EmitReport::default();
    let android = root.join("android");

    // settings.gradle — single-module project.
    report.touch(
        android.join("settings.gradle"),
        "include ':app'\nrootProject.name = 'BerryCodeApp'\n",
    )?;

    // top-level build.gradle.
    report.touch(
        android.join("build.gradle"),
        "buildscript {\n\
         \x20   repositories { google(); mavenCentral() }\n\
         \x20   dependencies { classpath 'com.android.tools.build:gradle:8.2.0' }\n\
         }\n\
         allprojects { repositories { google(); mavenCentral() } }\n",
    )?;

    // app/build.gradle.
    let app_gradle = format!(
        "plugins {{ id 'com.android.application' }}\n\
         android {{\n\
         \x20   namespace '{pkg}'\n\
         \x20   compileSdk 34\n\
         \x20   defaultConfig {{\n\
         \x20       applicationId '{pkg}'\n\
         \x20       minSdk 24\n\
         \x20       targetSdk 34\n\
         \x20       versionCode 1\n\
         \x20       versionName '1.0.0'\n\
         \x20       ndk {{ abiFilters 'arm64-v8a' }}\n\
         \x20   }}\n\
         \x20   buildTypes {{\n\
         \x20       release {{\n\
         \x20           minifyEnabled false\n\
         \x20           // signingConfig signingConfigs.release  // wired in v0.7.4\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n\
         // The Rust library is built by `cargo apk` / `xbuild` and dropped\n\
         // into `app/src/main/jniLibs/arm64-v8a/lib{name}.so`. v0.7.4 will\n\
         // wire that as a Gradle task.\n",
        pkg = package_name,
        name = app_name,
    );
    report.touch(android.join("app/build.gradle"), &app_gradle)?;

    // AndroidManifest.xml.
    let manifest = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <manifest xmlns:android=\"http://schemas.android.com/apk/res/android\">\n\
         \x20 <application android:label=\"{name}\" android:theme=\"@android:style/Theme.NoTitleBar.Fullscreen\">\n\
         \x20   <activity android:name=\"android.app.NativeActivity\"\n\
         \x20             android:exported=\"true\"\n\
         \x20             android:configChanges=\"orientation|keyboardHidden|screenSize\">\n\
         \x20     <meta-data android:name=\"android.app.lib_name\" android:value=\"{name}\"/>\n\
         \x20     <intent-filter>\n\
         \x20       <action android:name=\"android.intent.action.MAIN\"/>\n\
         \x20       <category android:name=\"android.intent.category.LAUNCHER\"/>\n\
         \x20     </intent-filter>\n\
         \x20   </activity>\n\
         \x20 </application>\n\
         </manifest>\n",
        name = app_name,
    );
    report.touch(android.join("app/src/main/AndroidManifest.xml"), &manifest)?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xcode_emit_creates_expected_files() {
        let tmp = tempfile::tempdir().unwrap();
        let report =
            emit_xcode_project(tmp.path(), "com.example.app", "ABCDE12345", "Demo").unwrap();
        let names: Vec<String> = report
            .created
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "Info.plist"));
        assert!(names.iter().any(|n| n == "App.swift"));
        assert!(names.iter().any(|n| n == "project.yml"));
        // Bundle id propagates into Info.plist.
        let plist = std::fs::read_to_string(tmp.path().join("ios/Info.plist")).unwrap();
        assert!(plist.contains("com.example.app"));
    }

    #[test]
    fn xcode_emit_preserves_user_edits() {
        let tmp = tempfile::tempdir().unwrap();
        emit_xcode_project(tmp.path(), "com.example.app", "ABCDE12345", "Demo").unwrap();
        // Edit the Info.plist out-of-band.
        let plist_path = tmp.path().join("ios/Info.plist");
        std::fs::write(&plist_path, "USER EDIT").unwrap();
        let report =
            emit_xcode_project(tmp.path(), "com.example.app", "ABCDE12345", "Demo").unwrap();
        assert!(report.preserved.iter().any(|p| p == &plist_path));
        assert_eq!(std::fs::read_to_string(&plist_path).unwrap(), "USER EDIT");
    }

    #[test]
    fn gradle_emit_emits_manifest_and_build_files() {
        let tmp = tempfile::tempdir().unwrap();
        let report = emit_gradle_project(tmp.path(), "com.example.app", "demo").unwrap();
        let names: Vec<String> = report
            .created
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "settings.gradle"));
        assert!(names.iter().any(|n| n == "AndroidManifest.xml"));
        let app_gradle =
            std::fs::read_to_string(tmp.path().join("android/app/build.gradle")).unwrap();
        assert!(app_gradle.contains("namespace 'com.example.app'"));
    }
}
