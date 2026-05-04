//! `.tscn` scene loading + tree reconstruction.
//!
//! Godot stores nodes flat in the `.tscn` file with a `parent="A/B"`
//! attribute pointing at the parent's NodePath. We rebuild that into
//! an explicit tree (`GodotScene::children`) so the UI can render it
//! without re-walking the flat list every frame.
//!
//! The tricky bit: `godot-properties-parser` only gives us each
//! section's `header_type` and the property body — it discards the
//! header attributes inside the brackets (`name=...`, `type=...`,
//! `parent=...`). We pre-parse the source file ourselves to recover
//! those attributes and zip them up with the parser's section index.

use std::fs;
use std::path::{Path, PathBuf};

use godot_properties_parser::{parse_scene_file, parsers::parser_scene_file::SceneFile};

#[derive(Debug)]
pub enum GodotSceneError {
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for GodotSceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error reading .tscn: {e}"),
            Self::Parse(msg) => write!(f, "failed to parse .tscn: {msg}"),
        }
    }
}

impl std::error::Error for GodotSceneError {}

impl From<std::io::Error> for GodotSceneError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Index into [`GodotScene::nodes`].
pub type NodeId = usize;

#[derive(Debug, Clone)]
pub struct GodotNode {
    pub name: String,
    /// e.g. `Node2D`, `Sprite2D`, `CharacterBody2D`. None for
    /// instanced scenes (`instance=ExtResource("...")`) where Godot
    /// itself omits the explicit type.
    pub type_name: Option<String>,
    /// Parent `NodePath`-style string verbatim from the file:
    /// - `None` for the root node
    /// - `Some(".")` for direct children of the root
    /// - `Some("Player/Sprite")` for nested children
    pub parent_path: Option<String>,
    /// Indices into [`GodotScene::nodes`].
    pub children: Vec<NodeId>,
    /// Properties from the body (`position = Vector2(...)`, etc).
    /// Stored verbatim — interpretation lives in the inspector.
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct GodotScene {
    pub path: PathBuf,
    pub nodes: Vec<GodotNode>,
    pub root: Option<NodeId>,
    /// `gd_scene format=N` from the header. We surface it so callers
    /// can warn on Godot 3 (format=2) projects, where the format diverges
    /// enough that we don't claim full fidelity.
    pub format_version: Option<u32>,
    /// External resources referenced by `ExtResource("id")` in node
    /// bodies. Useful for the inspector to resolve script / texture
    /// paths to absolute filesystem paths.
    pub ext_resources: Vec<ExtResource>,
    /// The full `godot-properties-parser` view, kept for callers that
    /// need anything we didn't lift into typed fields above.
    pub raw: SceneFile,
}

#[derive(Debug, Clone)]
pub struct ExtResource {
    pub id: String,
    pub type_name: Option<String>,
    /// `res://...` path verbatim.
    pub path: Option<String>,
}

impl GodotScene {
    /// Load a `.tscn` from disk.
    pub fn load(path: &Path) -> Result<Self, GodotSceneError> {
        let content = fs::read_to_string(path)?;
        Self::from_str(&content, path.to_path_buf())
    }

    /// Parse a scene from raw text. `path` is stored on the result for
    /// later reference and doesn't have to exist.
    pub fn from_str(content: &str, path: PathBuf) -> Result<Self, GodotSceneError> {
        let (_, parsed) =
            parse_scene_file(content).map_err(|e| GodotSceneError::Parse(format!("{e:?}")))?;

        // Re-scan the source for `[...]` lines so we can recover the
        // header attributes the property parser drops on the floor.
        let headers = scan_section_headers(content);

        // The parser emits sections in document order via `all_sections`,
        // so we can zip them positionally with `headers`.
        let format_version = parsed.header.as_ref().and_then(|_| {
            headers.iter().find(|h| h.kind == "gd_scene").and_then(|h| {
                h.attrs
                    .iter()
                    .find(|(k, _)| k == "format")
                    .and_then(|(_, v)| v.parse().ok())
            })
        });

        let ext_resources: Vec<ExtResource> = headers
            .iter()
            .filter(|h| h.kind == "ext_resource")
            .map(|h| ExtResource {
                id: h.attr("id").unwrap_or_default().to_string(),
                type_name: h.attr("type").map(str::to_string),
                path: h.attr("path").map(str::to_string),
            })
            .collect();

        // Build the node list in document order, then reconnect parents
        // by NodePath string. The first `[node]` is the root by Godot
        // convention (it has no `parent=` attribute).
        let node_headers: Vec<&SectionHeader> =
            headers.iter().filter(|h| h.kind == "node").collect();

        // Match each node header to its parsed Section by position so
        // we can pull the property body from the parser's output.
        let mut nodes: Vec<GodotNode> = Vec::with_capacity(node_headers.len());
        for (idx, header) in node_headers.iter().enumerate() {
            let properties = parsed
                .nodes
                .get(idx)
                .map(|s| {
                    s.properties
                        .iter()
                        .map(|p| (p.key.clone(), p.value.clone()))
                        .collect()
                })
                .unwrap_or_default();
            nodes.push(GodotNode {
                name: header.attr("name").unwrap_or("(unnamed)").to_string(),
                type_name: header.attr("type").map(str::to_string),
                parent_path: header.attr("parent").map(str::to_string),
                children: Vec::new(),
                properties,
            });
        }

        // Resolve children. The root is whichever node has no `parent=`.
        // Subsequent nodes' `parent="."` means root; `parent="A"` means
        // child of the node named A; `parent="A/B"` means grandchild.
        let root = nodes.iter().position(|n| n.parent_path.is_none());
        if let Some(root_id) = root {
            // Map (path-from-root → node id) so we can look up parents
            // by their NodePath string. Empty string = root itself.
            let mut path_to_id: std::collections::HashMap<String, NodeId> =
                std::collections::HashMap::new();
            path_to_id.insert(String::new(), root_id);

            for id in 0..nodes.len() {
                if Some(id) == root {
                    continue;
                }
                let parent_path = nodes[id].parent_path.clone().unwrap_or_default();
                let normalised = if parent_path == "." {
                    String::new()
                } else {
                    parent_path
                };
                if let Some(&parent_id) = path_to_id.get(&normalised) {
                    nodes[parent_id].children.push(id);
                    let own_path = if normalised.is_empty() {
                        nodes[id].name.clone()
                    } else {
                        format!("{normalised}/{}", nodes[id].name)
                    };
                    path_to_id.insert(own_path, id);
                }
                // If the parent path doesn't resolve we silently drop
                // the child from the tree. That's a malformed scene
                // file; surfacing it as a warning is a future polish
                // pass — for now the user still sees the file open.
            }
        }

        Ok(Self {
            path,
            nodes,
            root,
            format_version,
            ext_resources,
            raw: parsed,
        })
    }

    /// Pretty-print the tree as indented lines. Mostly useful for
    /// debugging and tests; the UI walks `children` directly.
    pub fn debug_tree(&self) -> String {
        let mut out = String::new();
        if let Some(root) = self.root {
            self.write_tree(root, 0, &mut out);
        }
        out
    }

    fn write_tree(&self, id: NodeId, depth: usize, out: &mut String) {
        let node = &self.nodes[id];
        for _ in 0..depth {
            out.push_str("  ");
        }
        match &node.type_name {
            Some(t) => out.push_str(&format!("{} ({})\n", node.name, t)),
            None => out.push_str(&format!("{}\n", node.name)),
        }
        for &child in &node.children {
            self.write_tree(child, depth + 1, out);
        }
    }
}

/// One `[...]` line in the source file. We keep the raw header type
/// (first token, e.g. `node`) and the attribute key/value pairs that
/// `godot-properties-parser` discards.
struct SectionHeader {
    kind: String,
    attrs: Vec<(String, String)>,
}

impl SectionHeader {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Scan a `.tscn` source for `[...]` lines and pull out each section's
/// header type + attribute key/value pairs. We respect quoted strings
/// so a `parent="."` attribute doesn't get tripped up by the dot, and
/// we balance brackets so `something=[1, 2]` doesn't end the header
/// early.
fn scan_section_headers(source: &str) -> Vec<SectionHeader> {
    let mut headers = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('[') {
            continue;
        }
        // Find the matching closing bracket. We can't just use
        // `rfind(']')` because attribute values like `[1, 2, 3]` are
        // legal, so we need to track depth and quoting.
        let bytes = trimmed.as_bytes();
        let mut depth: i32 = 0;
        let mut in_string = false;
        let mut escape = false;
        let mut close: Option<usize> = None;
        for (i, &b) in bytes.iter().enumerate() {
            let c = b as char;
            if escape {
                escape = false;
                continue;
            }
            match c {
                '\\' if in_string => escape = true,
                '"' => in_string = !in_string,
                '[' if !in_string => depth += 1,
                ']' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = close else { continue };
        // `trimmed` looks like `[node name="Main" type="Node2D"]`.
        // We want the slice between the outer brackets.
        let inner = &trimmed[1..end];
        let mut tokens = inner.splitn(2, char::is_whitespace);
        let kind = tokens.next().unwrap_or("").to_string();
        let attr_blob = tokens.next().unwrap_or("");
        let attrs = split_attrs(attr_blob);
        headers.push(SectionHeader { kind, attrs });
    }
    headers
}

/// `name="Main" type="Node2D" parent="."` → vec of (key, value).
/// Values are returned without surrounding quotes for ergonomics.
fn split_attrs(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut chars = s.char_indices().peekable();
    while let Some(&(_, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        // Read key up to '='.
        let key_start = chars.peek().map(|(i, _)| *i).unwrap_or(s.len());
        let mut key_end = key_start;
        while let Some(&(i, ch)) = chars.peek() {
            if ch == '=' {
                key_end = i;
                chars.next();
                break;
            }
            key_end = i + ch.len_utf8();
            chars.next();
        }
        let key = s[key_start..key_end].trim().to_string();
        if key.is_empty() {
            break;
        }
        // Read value: either a quoted string (handle escapes) or
        // a bracket-balanced bareword up to the next whitespace.
        let value = read_attr_value(&mut chars, s);
        out.push((key, value));
    }
    out
}

fn read_attr_value(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>, s: &str) -> String {
    let Some(&(start, first)) = chars.peek() else {
        return String::new();
    };
    if first == '"' {
        chars.next();
        let val_start = chars.peek().map(|(i, _)| *i).unwrap_or(s.len());
        let mut val_end = val_start;
        let mut escape = false;
        while let Some(&(i, ch)) = chars.peek() {
            if escape {
                escape = false;
                val_end = i + ch.len_utf8();
                chars.next();
                continue;
            }
            match ch {
                '\\' => {
                    escape = true;
                    val_end = i + ch.len_utf8();
                    chars.next();
                }
                '"' => {
                    let result = s[val_start..i].to_string();
                    chars.next();
                    return result;
                }
                _ => {
                    val_end = i + ch.len_utf8();
                    chars.next();
                }
            }
        }
        return s[val_start..val_end].to_string();
    }
    // Bareword (e.g. `format=3`). Allow nested `[...]` and `(...)`
    // because Godot writes things like `something=PackedStringArray("a")`.
    let mut depth_b: i32 = 0;
    let mut depth_p: i32 = 0;
    let mut in_string = false;
    let mut end = start;
    while let Some(&(i, ch)) = chars.peek() {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            end = i + ch.len_utf8();
            chars.next();
            continue;
        }
        if ch.is_whitespace() && depth_b == 0 && depth_p == 0 {
            break;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth_b += 1,
            ']' => depth_b -= 1,
            '(' => depth_p += 1,
            ')' => depth_p -= 1,
            _ => {}
        }
        end = i + ch.len_utf8();
        chars.next();
    }
    s[start..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_scene() -> Option<PathBuf> {
        let p = PathBuf::from("/tmp/godot_sample/scenes/main.tscn");
        p.is_file().then_some(p)
    }

    #[test]
    fn parses_sample_scene_into_tree() {
        let Some(path) = sample_scene() else {
            return;
        };
        let scene = GodotScene::load(&path).expect("loads sample scene");
        assert_eq!(scene.format_version, Some(3));
        assert_eq!(scene.ext_resources.len(), 1);
        assert_eq!(scene.ext_resources[0].id, "1_player");
        let root = scene.root.expect("scene has a root");
        let root_node = &scene.nodes[root];
        assert_eq!(root_node.name, "Main");
        assert_eq!(root_node.type_name.as_deref(), Some("Node2D"));
        // Background, Player are direct children of root.
        assert_eq!(root_node.children.len(), 2);
        // Player has PlayerSprite + Camera as children.
        let player_idx = scene.nodes.iter().position(|n| n.name == "Player").unwrap();
        assert_eq!(scene.nodes[player_idx].children.len(), 2);
    }

    #[test]
    fn debug_tree_renders_indentation() {
        let Some(path) = sample_scene() else {
            return;
        };
        let scene = GodotScene::load(&path).unwrap();
        let tree = scene.debug_tree();
        assert!(tree.contains("Main (Node2D)"));
        assert!(tree.contains("  Player (CharacterBody2D)"));
        assert!(tree.contains("    PlayerSprite (Sprite2D)"));
    }

    #[test]
    fn split_attrs_handles_quoted_values() {
        let pairs = split_attrs(r#"name="Main" type="Node2D" parent="." "#);
        assert_eq!(
            pairs,
            vec![
                ("name".to_string(), "Main".to_string()),
                ("type".to_string(), "Node2D".to_string()),
                ("parent".to_string(), ".".to_string()),
            ]
        );
    }

    #[test]
    fn split_attrs_handles_bareword_with_parens() {
        let pairs = split_attrs(r#"format=3 uid="uid://abc""#);
        assert_eq!(
            pairs,
            vec![
                ("format".to_string(), "3".to_string()),
                ("uid".to_string(), "uid://abc".to_string()),
            ]
        );
    }

    #[test]
    fn split_attrs_keeps_packed_array_intact_as_value() {
        // `something=PackedStringArray("a", "b")` — the value contains
        // both parens and quotes. Our value reader has to keep the
        // commas + quotes inside the parens together rather than
        // splitting at the comma.
        let pairs = split_attrs(r#"features=PackedStringArray("4.4", "GL Compatibility")"#);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "features");
        assert!(pairs[0].1.starts_with("PackedStringArray("));
        assert!(pairs[0].1.ends_with(')'));
    }

    #[test]
    fn split_attrs_handles_escaped_quotes_in_string() {
        let pairs = split_attrs(r#"name="Has \"quotes\" inside""#);
        assert_eq!(pairs.len(), 1);
        // The reader strips the outer quotes and keeps the escapes
        // verbatim — interpretation is the caller's job.
        assert_eq!(pairs[0].0, "name");
        assert!(pairs[0].1.contains("quotes"));
    }

    /// A self-contained two-level scene useful for exercising tree
    /// reconstruction without touching the filesystem.
    fn two_level_scene_source() -> &'static str {
        r#"[gd_scene load_steps=1 format=3]

[node name="Root" type="Node2D"]

[node name="A" type="Node2D" parent="."]

[node name="B" type="Node2D" parent="."]

[node name="A1" type="Sprite2D" parent="A"]

[node name="A2" type="Sprite2D" parent="A"]
"#
    }

    #[test]
    fn from_str_assembles_tree_without_disk_access() {
        let scene = GodotScene::from_str(two_level_scene_source(), PathBuf::from("/dummy.tscn"))
            .expect("inline parse");
        let root = scene.root.expect("has root");
        assert_eq!(scene.nodes[root].name, "Root");
        assert_eq!(scene.nodes[root].children.len(), 2);

        // Find node A and confirm it has 2 children (A1, A2).
        let a_id = scene
            .nodes
            .iter()
            .position(|n| n.name == "A")
            .expect("A exists");
        assert_eq!(scene.nodes[a_id].children.len(), 2);
        let names: Vec<&str> = scene.nodes[a_id]
            .children
            .iter()
            .map(|&id| scene.nodes[id].name.as_str())
            .collect();
        assert!(names.contains(&"A1"));
        assert!(names.contains(&"A2"));
    }

    #[test]
    fn from_str_resolves_three_level_nesting() {
        let source = r#"[gd_scene load_steps=1 format=3]

[node name="Root" type="Node2D"]

[node name="Mid" type="Node2D" parent="."]

[node name="Leaf" type="Sprite2D" parent="Mid"]

[node name="Pixel" type="Sprite2D" parent="Mid/Leaf"]
"#;
        let scene = GodotScene::from_str(source, PathBuf::from("/x.tscn")).unwrap();
        let pixel = scene
            .nodes
            .iter()
            .find(|n| n.name == "Pixel")
            .expect("deepest node exists");
        assert_eq!(pixel.parent_path.as_deref(), Some("Mid/Leaf"));

        // The grandparent ("Mid") should have exactly one child ("Leaf"),
        // and "Leaf" should have exactly one child ("Pixel").
        let leaf_id = scene.nodes.iter().position(|n| n.name == "Leaf").unwrap();
        assert_eq!(scene.nodes[leaf_id].children.len(), 1);
        assert_eq!(scene.nodes[scene.nodes[leaf_id].children[0]].name, "Pixel");
    }

    #[test]
    fn from_str_drops_orphaned_nodes_silently() {
        // `[node parent="Ghost"]` references a parent that doesn't
        // exist. We don't crash — we just leave the orphan out of the
        // tree (the file is malformed and the reader's job is best-
        // effort viewing, not validation).
        let source = r#"[gd_scene load_steps=1 format=3]

[node name="Root" type="Node2D"]

[node name="Orphan" type="Sprite2D" parent="Ghost"]
"#;
        let scene = GodotScene::from_str(source, PathBuf::from("/x.tscn")).unwrap();
        let root = scene.root.unwrap();
        assert_eq!(
            scene.nodes[root].children.len(),
            0,
            "orphan should not be attached"
        );
        // The orphan is still in `nodes` (so future migration tooling
        // can surface it as a warning), just not reachable from root.
        assert!(scene.nodes.iter().any(|n| n.name == "Orphan"));
    }

    #[test]
    fn from_str_handles_no_root() {
        // A `.tscn` with zero `[node]` sections (just resources). The
        // result has no root and an empty node list — the caller should
        // render this as "(empty scene)".
        let source = r#"[gd_resource type="Resource" format=3]

[ext_resource type="Texture2D" path="res://icon.png" id="1"]
"#;
        let scene = GodotScene::from_str(source, PathBuf::from("/x.tres")).unwrap();
        assert!(scene.root.is_none());
        assert!(scene.nodes.is_empty());
        assert_eq!(scene.ext_resources.len(), 1);
        assert_eq!(
            scene.ext_resources[0].path.as_deref(),
            Some("res://icon.png")
        );
    }

    #[test]
    fn from_str_preserves_node_properties() {
        let source = r#"[gd_scene load_steps=1 format=3]

[node name="Root" type="Sprite2D"]
position = Vector2(100, 200)
modulate = Color(1, 0.5, 0.5, 1)
"#;
        let scene = GodotScene::from_str(source, PathBuf::from("/x.tscn")).unwrap();
        let root = &scene.nodes[scene.root.unwrap()];
        let pos = root
            .properties
            .iter()
            .find(|(k, _)| k == "position")
            .expect("position property survives");
        assert!(pos.1.starts_with("Vector2"));
        assert!(root.properties.iter().any(|(k, _)| k == "modulate"));
    }

    #[test]
    fn from_str_extracts_multiple_ext_resources() {
        let source = r#"[gd_scene load_steps=3 format=3]

[ext_resource type="Script" path="res://a.gd" id="1_a"]
[ext_resource type="Texture2D" path="res://b.png" id="2_b"]
[ext_resource type="PackedScene" path="res://c.tscn" id="3_c"]

[node name="Root" type="Node2D"]
"#;
        let scene = GodotScene::from_str(source, PathBuf::from("/x.tscn")).unwrap();
        assert_eq!(scene.ext_resources.len(), 3);
        let ids: Vec<&str> = scene.ext_resources.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["1_a", "2_b", "3_c"]);
        assert_eq!(
            scene.ext_resources[1].type_name.as_deref(),
            Some("Texture2D")
        );
    }

    #[test]
    fn from_str_recovers_format_version() {
        // We pull `format=N` out of the `[gd_scene format=N]` header
        // attributes. Make sure the value is parsed as a number, not
        // captured as a string.
        let v3 = "[gd_scene load_steps=1 format=3]\n";
        let scene = GodotScene::from_str(v3, PathBuf::from("/")).unwrap();
        assert_eq!(scene.format_version, Some(3));

        let v2 = "[gd_scene load_steps=1 format=2]\n";
        let scene = GodotScene::from_str(v2, PathBuf::from("/")).unwrap();
        assert_eq!(scene.format_version, Some(2));
    }

    #[test]
    fn from_str_format_version_missing_when_no_header() {
        // No `[gd_scene]` line at all → format_version stays None
        // rather than defaulting to a misleading value.
        let no_header = "[node name=\"X\" type=\"Node\"]\n";
        let scene = GodotScene::from_str(no_header, PathBuf::from("/")).unwrap();
        assert!(scene.format_version.is_none());
    }

    #[test]
    fn debug_tree_handles_empty_scene() {
        let scene = GodotScene::from_str("", PathBuf::from("/")).unwrap();
        // No root → empty string, not a panic.
        assert_eq!(scene.debug_tree(), "");
    }

    #[test]
    fn scan_section_headers_skips_property_lines() {
        // Lines like `position = Vector2(...)` shouldn't be mistaken
        // for section headers just because they contain `(...)`.
        let source = r#"[node name="X" type="Node2D"]
position = Vector2(0, 0)
modulate = Color(1, 1, 1, 1)
"#;
        let headers = scan_section_headers(source);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].kind, "node");
    }

    #[test]
    fn scan_section_headers_balances_brackets_in_values() {
        // `something=[1, 2, 3]` is legal inside a header; the closing
        // bracket of that array shouldn't end the header early.
        let source = r#"[node name="X" type="Node" something=[1, 2, 3]]"#;
        let headers = scan_section_headers(source);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].attr("name"), Some("X"));
        assert_eq!(headers[0].attr("type"), Some("Node"));
        assert_eq!(headers[0].attr("something"), Some("[1, 2, 3]"));
    }
}
