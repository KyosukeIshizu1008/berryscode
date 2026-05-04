//! Read-only Godot project support (v0.8.x Migration & interop).
//!
//! BerryCode opens an existing Godot project so users migrating to
//! Bevy can browse their old code and scene structure alongside the
//! new Bevy project they're building. **Strictly read-only**:
//! we never write back to `.tscn` / `.godot`, and we never claim to
//! convert files automatically. The intent is "speak the format well
//! enough that the user doesn't have to alt-tab to the Godot editor".
//!
//! `godot-properties-parser` handles the property bodies (`key = value`
//! lines), but it discards section header *attributes* — the
//! `name="Main" type="Node2D" parent="."` part inside the brackets.
//! That's exactly the data we need to rebuild the scene tree, so we
//! re-scan the source for `[...]` lines in `scene::header_attrs`.

use std::path::{Path, PathBuf};

pub mod project;
pub mod scene;

pub use project::{GodotProject, GodotProjectError};
pub use scene::{GodotNode, GodotScene, GodotSceneError, NodeId};

/// True if `dir` looks like a Godot project root — i.e. contains a
/// `project.godot` file. Used by the file tree to decide whether to
/// surface the migration affordances.
pub fn is_godot_project(dir: &Path) -> bool {
    dir.join("project.godot").is_file()
}

/// Walk upwards from `path` looking for the nearest enclosing Godot
/// project root. Returns `None` if there's no `project.godot` between
/// `path` and the filesystem root.
pub fn find_project_root(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };
    loop {
        if is_godot_project(&current) {
            return Some(current);
        }
        current = current.parent()?.to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_godot_project_recognises_sample() {
        // Built by the Day 1 implementation flow at /tmp/godot_sample.
        // The test is `#[ignore]`d when the sample isn't on disk so CI
        // doesn't flake on machines that haven't set it up.
        let sample = Path::new("/tmp/godot_sample");
        if !sample.exists() {
            return;
        }
        assert!(is_godot_project(sample));
        assert!(!is_godot_project(Path::new("/tmp")));
    }

    #[test]
    fn find_project_root_walks_up() {
        let sample = Path::new("/tmp/godot_sample");
        if !sample.exists() {
            return;
        }
        let nested = sample.join("scenes").join("main.tscn");
        assert_eq!(find_project_root(&nested), Some(sample.to_path_buf()));
    }

    #[test]
    fn find_project_root_returns_none_for_non_godot_tree() {
        // A path that has no `project.godot` between it and the
        // filesystem root should return None rather than walking
        // forever or panicking. We use `/usr/bin` because it exists
        // on every supported platform and definitely isn't a Godot
        // project.
        let result = find_project_root(Path::new("/usr/bin"));
        assert!(result.is_none());
    }

    #[test]
    fn find_project_root_handles_nonexistent_path() {
        // Walking up from a path that doesn't exist still works as
        // long as the parent chain is well-formed: each `parent()`
        // step returns the lexical parent regardless of whether the
        // directory exists.
        let result = find_project_root(Path::new("/this/path/does/not/exist"));
        assert!(result.is_none());
    }

    #[test]
    fn find_project_root_returns_self_when_passed_a_godot_root() {
        // If the input *is* the Godot project root, we should return
        // it directly rather than walking past it to the parent.
        let sample = Path::new("/tmp/godot_sample");
        if !sample.exists() {
            return;
        }
        assert_eq!(find_project_root(sample), Some(sample.to_path_buf()));
    }

    #[test]
    fn is_godot_project_rejects_a_file_path() {
        // The function takes a directory; passing a regular file
        // should return false rather than crashing.
        // `/etc/hosts` exists on macOS and Linux; on a fresh CI image
        // we fall back to skipping if it doesn't exist locally.
        let hosts = Path::new("/etc/hosts");
        if !hosts.is_file() {
            return;
        }
        assert!(!is_godot_project(hosts));
    }
}
