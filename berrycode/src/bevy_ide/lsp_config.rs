//! Bevy-optimized rust-analyzer LSP configuration
//!
//! Generates initialization options that improve the IDE experience
//! for Bevy projects, including proc-macro support and diagnostics tuning.

use serde_json::{json, Value};

/// Generate rust-analyzer initialization options optimized for Bevy projects.
///
/// These settings enable proc-macro expansion (required for Bevy's derive macros
/// like `#[derive(Component)]`, `#[derive(Resource)]`, etc.), enable all cargo
/// features so that conditional compilation blocks are analyzed, and suppress
/// the common `unresolved-proc-macro` diagnostic that fires while rust-analyzer
/// is still loading.
pub fn bevy_rust_analyzer_options() -> Value {
    json!({
        "rust-analyzer.diagnostics.disabled": ["unresolved-proc-macro"],
        "rust-analyzer.cargo.features": "all",
        "rust-analyzer.procMacro.enable": true,
        "rust-analyzer.procMacro.attributes.enable": true,
        "rust-analyzer.check.command": "clippy"
    })
}

/// Merge Bevy-specific options into an existing rust-analyzer configuration.
///
/// Existing keys are preserved; Bevy-specific keys are added only if absent.
pub fn merge_with_existing(existing: &mut Value) {
    let bevy_opts = bevy_rust_analyzer_options();

    if let (Some(target), Some(source)) = (existing.as_object_mut(), bevy_opts.as_object()) {
        for (key, value) in source {
            target.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bevy_options_has_proc_macro_enabled() {
        let opts = bevy_rust_analyzer_options();
        assert_eq!(opts["rust-analyzer.procMacro.enable"], true);
    }

    #[test]
    fn test_bevy_options_has_clippy_check() {
        let opts = bevy_rust_analyzer_options();
        assert_eq!(opts["rust-analyzer.check.command"], "clippy");
    }

    #[test]
    fn test_merge_preserves_existing() {
        let mut existing = json!({
            "rust-analyzer.cargo.features": "default",
            "custom.key": "value"
        });
        merge_with_existing(&mut existing);

        // Existing key should NOT be overwritten
        assert_eq!(existing["rust-analyzer.cargo.features"], "default");
        // Custom key preserved
        assert_eq!(existing["custom.key"], "value");
        // New Bevy keys added
        assert_eq!(existing["rust-analyzer.procMacro.enable"], true);
    }
}
