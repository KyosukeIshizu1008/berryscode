//! `project.godot` loading. Thin wrapper over
//! `godot_properties_parser::parse_project_file` that pulls out the
//! handful of fields the file tree + status bar actually need today
//! (project name, main scene path, icon path). We deliberately keep
//! the rest of the parsed structure available via `raw` so future
//! features can dig deeper without re-parsing.

use std::fs;
use std::path::{Path, PathBuf};

use godot_properties_parser::{parse_project_file, parsers::parser_project_file::ProjectFile};

#[derive(Debug)]
pub enum GodotProjectError {
    NotFound(PathBuf),
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for GodotProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(p) => write!(f, "project.godot not found in {}", p.display()),
            Self::Io(e) => write!(f, "io error reading project.godot: {e}"),
            Self::Parse(msg) => write!(f, "failed to parse project.godot: {msg}"),
        }
    }
}

impl std::error::Error for GodotProjectError {}

impl From<std::io::Error> for GodotProjectError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[derive(Debug, Clone)]
pub struct GodotProject {
    pub root: PathBuf,
    pub name: Option<String>,
    /// `res://scenes/main.tscn` style path as written in `project.godot`.
    /// Resolve to an absolute path with [`GodotProject::resolve_res_path`].
    pub main_scene: Option<String>,
    pub icon: Option<String>,
    /// `config_version` from the file preamble. Useful to flag Godot 3
    /// projects (config_version=4 or below) where our parser is less
    /// confident.
    pub config_version: Option<u32>,
    /// Full parsed structure for callers that want to reach beyond the
    /// curated fields above.
    pub raw: ProjectFile,
}

impl GodotProject {
    /// Load the `project.godot` at `<root>/project.godot`.
    pub fn load(root: &Path) -> Result<Self, GodotProjectError> {
        let project_file_path = root.join("project.godot");
        if !project_file_path.is_file() {
            return Err(GodotProjectError::NotFound(root.to_path_buf()));
        }
        let content = fs::read_to_string(&project_file_path)?;
        Self::from_str(&content, root.to_path_buf())
    }

    /// Parse a `project.godot` from raw text. Useful for tests and for
    /// callers that already have the contents in memory (e.g. a future
    /// virtual filesystem or zip-archive backend).
    pub fn from_str(content: &str, root: PathBuf) -> Result<Self, GodotProjectError> {
        let (_, parsed) =
            parse_project_file(content).map_err(|e| GodotProjectError::Parse(format!("{e:?}")))?;

        // Pull a few well-known fields out of the parsed structure so
        // callers don't all need to spelunk the same Section paths.
        let config_version = parsed
            .preamble_properties
            .iter()
            .find(|p| p.key == "config_version")
            .and_then(|p| p.value.parse().ok());

        let app_prop = |key: &str| -> Option<String> {
            parsed
                .application
                .as_ref()?
                .properties
                .iter()
                .find(|p| p.key == key)
                .map(|p| strip_quotes(&p.value).to_string())
        };

        Ok(Self {
            root,
            name: app_prop("config/name"),
            main_scene: app_prop("run/main_scene"),
            icon: app_prop("config/icon"),
            config_version,
            raw: parsed,
        })
    }

    /// Resolve a `res://...` path against the project root.
    pub fn resolve_res_path(&self, res_path: &str) -> Option<PathBuf> {
        let stripped = res_path.strip_prefix("res://")?;
        Some(self.root.join(stripped))
    }
}

/// `"foo"` → `foo`. Property values in `.godot` are usually quoted
/// strings but the parser hands them to us with the quotes still on.
fn strip_quotes(s: &str) -> &str {
    let trimmed = s.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_root() -> Option<PathBuf> {
        let p = PathBuf::from("/tmp/godot_sample");
        p.exists().then_some(p)
    }

    #[test]
    fn loads_sample_project_metadata() {
        let Some(root) = sample_root() else {
            return;
        };
        let project = GodotProject::load(&root).expect("loads /tmp/godot_sample");
        assert_eq!(project.name.as_deref(), Some("GodotSample"));
        assert_eq!(
            project.main_scene.as_deref(),
            Some("res://scenes/main.tscn")
        );
        assert_eq!(project.config_version, Some(5));
    }

    #[test]
    fn resolve_res_path_against_root() {
        let Some(root) = sample_root() else {
            return;
        };
        let project = GodotProject::load(&root).unwrap();
        let resolved = project.resolve_res_path("res://scenes/main.tscn").unwrap();
        assert_eq!(resolved, root.join("scenes/main.tscn"));
    }

    #[test]
    fn missing_project_godot_is_an_error() {
        let err = GodotProject::load(Path::new("/tmp")).unwrap_err();
        assert!(matches!(err, GodotProjectError::NotFound(_)));
    }

    /// A minimal but syntactically real `project.godot`. Inline so the
    /// parsing tests don't depend on `/tmp/godot_sample` existing — the
    /// integration-style tests above cover the on-disk path.
    fn minimal_project_godot() -> &'static str {
        r#"; Engine config
config_version=5

[application]

config/name="Inline Sample"
run/main_scene="res://main.tscn"
config/icon="res://icon.svg"

[display]

window/size/viewport_width=1280
"#
    }

    #[test]
    fn from_str_parses_application_fields() {
        let project = GodotProject::from_str(minimal_project_godot(), PathBuf::from("/anywhere"))
            .expect("inline parse should succeed");
        assert_eq!(project.name.as_deref(), Some("Inline Sample"));
        assert_eq!(project.main_scene.as_deref(), Some("res://main.tscn"));
        assert_eq!(project.icon.as_deref(), Some("res://icon.svg"));
        assert_eq!(project.config_version, Some(5));
        assert_eq!(project.root, PathBuf::from("/anywhere"));
    }

    #[test]
    fn from_str_keeps_raw_for_unsurfaced_sections() {
        // The `[display]` section isn't lifted into a typed field on
        // `GodotProject`, but the full parsed structure should still be
        // reachable via `raw` so callers can dig in without re-parsing.
        let project = GodotProject::from_str(minimal_project_godot(), PathBuf::from("/")).unwrap();
        let display = project
            .raw
            .display
            .as_ref()
            .expect("[display] section round-trips through raw");
        assert!(display
            .properties
            .iter()
            .any(|p| p.key == "window/size/viewport_width"));
    }

    #[test]
    fn from_str_handles_no_application_section() {
        // `[application]` is optional in Godot — the editor will write
        // a project.godot with only `config_version=` and section-less
        // defaults if the user hasn't customised anything. Our wrapper
        // shouldn't blow up; it should just return None for everything
        // it would have pulled from `[application]`.
        let bare = "config_version=5\n";
        let project = GodotProject::from_str(bare, PathBuf::from("/")).unwrap();
        assert!(project.name.is_none());
        assert!(project.main_scene.is_none());
        assert!(project.icon.is_none());
        assert_eq!(project.config_version, Some(5));
    }

    #[test]
    fn resolve_res_path_rejects_non_res_prefix() {
        let project = GodotProject::from_str("config_version=5\n", PathBuf::from("/root")).unwrap();
        // Anything that isn't a `res://` URI is not the wrapper's
        // problem to resolve. Plain absolute paths, `user://` paths,
        // and bare filenames all return None.
        assert!(project.resolve_res_path("/absolute/path").is_none());
        assert!(project.resolve_res_path("user://savegame.dat").is_none());
        assert!(project.resolve_res_path("relative/path.tscn").is_none());
    }

    #[test]
    fn resolve_res_path_handles_root_resource() {
        let project = GodotProject::from_str("config_version=5\n", PathBuf::from("/root")).unwrap();
        // A bare `res://` (no path component) is technically valid and
        // points at the project root itself.
        assert_eq!(
            project.resolve_res_path("res://"),
            Some(PathBuf::from("/root"))
        );
    }

    #[test]
    fn config_version_is_optional() {
        let no_version = "[application]\nconfig/name=\"X\"\n";
        let project = GodotProject::from_str(no_version, PathBuf::from("/")).unwrap();
        assert!(project.config_version.is_none());
        assert_eq!(project.name.as_deref(), Some("X"));
    }

    #[test]
    fn strip_quotes_handles_unquoted_values() {
        // Property values aren't always quoted (`config_version=5` is bare).
        // The helper should pass those through unchanged rather than
        // accidentally trimming a leading/trailing char.
        assert_eq!(strip_quotes("5"), "5");
        assert_eq!(strip_quotes("\"5\""), "5");
        assert_eq!(strip_quotes("  \"hello\"  "), "hello");
        // Mismatched quotes (only on one side) shouldn't be stripped —
        // returning the malformed value is more honest than silently
        // half-fixing it.
        assert_eq!(strip_quotes("\"unclosed"), "\"unclosed");
    }
}
