//! Google Play Console upload helper — Phase E follow-up.
//!
//! Wraps the Google Play Developer API v3 `edits.bundles.upload` flow so
//! the user can ship straight from BerryCode without touching `fastlane`.
//! Authentication uses a service-account JSON key (created in the
//! Play Console under "API access").
//!
//! v0.7.2 ships a thin scaffold: it validates the inputs, opens the
//! service-account JSON, and surfaces the `curl` command the user would
//! run by hand. Real OAuth2 + multipart upload lands in v0.7.3 — that
//! requires a JWT signer + multipart streamer that we'd rather get right
//! once than half-implement under time pressure.

#![allow(dead_code)]

use std::path::Path;

#[derive(Debug)]
pub enum UploadTrack {
    Internal,
    Alpha,
    Beta,
    Production,
}

impl UploadTrack {
    pub fn label(&self) -> &'static str {
        match self {
            UploadTrack::Internal => "internal",
            UploadTrack::Alpha => "alpha",
            UploadTrack::Beta => "beta",
            UploadTrack::Production => "production",
        }
    }
}

/// Validate inputs and return the manual upload command the user can
/// run while v0.7.3's full OAuth2 / multipart flow is being built.
/// Returns `Ok(command_string)` for display, `Err` when an input is
/// missing or unreadable.
pub fn upload_aab(
    aab_path: &Path,
    service_account_json: &Path,
    package_name: &str,
    track: UploadTrack,
) -> Result<String, String> {
    if !aab_path.exists() {
        return Err(format!(
            "AAB not found at {}. Build it via `Build → Android → Release AAB` first.",
            aab_path.display()
        ));
    }
    if !service_account_json.exists() {
        return Err(format!(
            "Play Console service-account JSON not found at {}. Create one in \
             Play Console → API access → Service accounts.",
            service_account_json.display()
        ));
    }
    if package_name.is_empty() {
        return Err("Android package name is empty — set it in Build Settings.".to_string());
    }
    Ok(format!(
        "# v0.7.2 scaffold — full upload lands in v0.7.3.\n\
         # Until then, run this manually (requires `gcloud` + `curl`):\n\
         gcloud auth activate-service-account --key-file={sa}\n\
         ACCESS=$(gcloud auth print-access-token)\n\
         EDIT=$(curl -s -H \"Authorization: Bearer $ACCESS\" -X POST \\\n\
         \x20   https://androidpublisher.googleapis.com/androidpublisher/v3/applications/{pkg}/edits | jq -r .id)\n\
         curl -s -H \"Authorization: Bearer $ACCESS\" -H \"Content-Type: application/octet-stream\" \\\n\
         \x20   -X POST --data-binary @{aab} \\\n\
         \x20   https://androidpublisher.googleapis.com/upload/androidpublisher/v3/applications/{pkg}/edits/$EDIT/bundles\n\
         curl -s -H \"Authorization: Bearer $ACCESS\" -H \"Content-Type: application/json\" \\\n\
         \x20   -X PUT -d '{{\"track\":\"{track}\",\"releases\":[{{\"status\":\"completed\"}}]}}' \\\n\
         \x20   https://androidpublisher.googleapis.com/androidpublisher/v3/applications/{pkg}/edits/$EDIT/tracks/{track}\n\
         curl -s -H \"Authorization: Bearer $ACCESS\" -X POST \\\n\
         \x20   https://androidpublisher.googleapis.com/androidpublisher/v3/applications/{pkg}/edits/$EDIT:commit\n",
        sa = service_account_json.display(),
        pkg = package_name,
        aab = aab_path.display(),
        track = track.label(),
    ))
}

/// Same scaffold for the iOS side — `xcrun altool --upload-app`. Returns
/// the manual command until v0.7.3 wraps the App Store Connect API.
pub fn upload_ipa(
    ipa_path: &Path,
    apple_id: &str,
    app_specific_password: &str,
) -> Result<String, String> {
    if !ipa_path.exists() {
        return Err(format!(
            "IPA not found at {}. Build it via `Build → iOS → Release IPA` first.",
            ipa_path.display()
        ));
    }
    if apple_id.is_empty() {
        return Err("Apple ID is empty — set it in Build Settings.".to_string());
    }
    if app_specific_password.is_empty() {
        return Err(
            "App-specific password is empty — generate one at appleid.apple.com → Sign-In Security."
                .to_string(),
        );
    }
    Ok(format!(
        "# v0.7.2 scaffold — full upload lands in v0.7.3.\n\
         xcrun altool --upload-app --type ios --file {ipa} \\\n\
         \x20   --username {user} --password {pw}\n",
        ipa = ipa_path.display(),
        user = apple_id,
        pw = app_specific_password,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn upload_aab_rejects_missing_aab() {
        let tmp = tempfile::tempdir().unwrap();
        let sa = tmp.path().join("sa.json");
        let _ = std::fs::File::create(&sa).unwrap().write_all(b"{}");
        let err = upload_aab(
            &tmp.path().join("missing.aab"),
            &sa,
            "com.example.app",
            UploadTrack::Internal,
        )
        .unwrap_err();
        assert!(err.contains("AAB not found"), "{err}");
    }

    #[test]
    fn upload_aab_rejects_empty_package_name() {
        let tmp = tempfile::tempdir().unwrap();
        let aab = tmp.path().join("app.aab");
        let sa = tmp.path().join("sa.json");
        std::fs::File::create(&aab).unwrap();
        std::fs::File::create(&sa).unwrap();
        let err = upload_aab(&aab, &sa, "", UploadTrack::Internal).unwrap_err();
        assert!(err.contains("package name"), "{err}");
    }

    #[test]
    fn upload_aab_emits_curl_for_valid_inputs() {
        let tmp = tempfile::tempdir().unwrap();
        let aab = tmp.path().join("app.aab");
        let sa = tmp.path().join("sa.json");
        std::fs::File::create(&aab).unwrap();
        std::fs::File::create(&sa).unwrap();
        let cmd = upload_aab(&aab, &sa, "com.example.app", UploadTrack::Beta).unwrap();
        assert!(cmd.contains("com.example.app"));
        assert!(cmd.contains("\"track\":\"beta\""));
    }

    #[test]
    fn track_labels_match_play_api() {
        assert_eq!(UploadTrack::Internal.label(), "internal");
        assert_eq!(UploadTrack::Production.label(), "production");
    }
}
