use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Asset entry types
#[derive(Debug, Clone, PartialEq)]
pub enum AssetType {
    Image,   // .png, .jpg, .webp, .ktx2
    Model3D, // .gltf, .glb
    Audio,   // .ogg, .wav, .mp3, .flac
    Scene,   // .scn.ron, .ron
    Font,    // .ttf, .otf
    Shader,  // .wgsl, .frag, .vert
    Data,    // .json, .toml, .yaml
    Other,
}

/// Single asset entry
#[derive(Debug, Clone)]
pub struct AssetEntry {
    pub path: PathBuf,
    pub relative_path: String,
    pub file_name: String,
    pub asset_type: AssetType,
    pub size_bytes: u64,
    pub extension: String,
}

/// Asset browser state
pub struct AssetBrowserState {
    pub assets: Vec<AssetEntry>,
    pub asset_root: String,
    pub filter_query: String,
    pub filter_type: Option<AssetType>,
    pub selected_asset: Option<usize>,
    pub scan_pending: bool,
    pub view_mode: AssetViewMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssetViewMode {
    List,
    Grid,
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        Self {
            assets: Vec::new(),
            asset_root: "assets".to_string(),
            filter_query: String::new(),
            filter_type: None,
            selected_asset: None,
            scan_pending: true,
            view_mode: AssetViewMode::List,
        }
    }
}

impl AssetType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "ktx2" | "bmp" | "tga" | "dds" | "hdr" => {
                AssetType::Image
            }
            "gltf" | "glb" | "obj" | "fbx" => AssetType::Model3D,
            "ogg" | "wav" | "mp3" | "flac" | "aac" => AssetType::Audio,
            "ron" => AssetType::Scene,
            "ttf" | "otf" | "woff" | "woff2" => AssetType::Font,
            "wgsl" | "frag" | "vert" | "glsl" | "hlsl" => AssetType::Shader,
            "json" | "toml" | "yaml" | "yml" | "xml" => AssetType::Data,
            _ => AssetType::Other,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AssetType::Image => "\u{eb60}",   // codicon-file-media
            AssetType::Model3D => "\u{ea73}", // codicon-file-code
            AssetType::Audio => "\u{eb60}",   // codicon-file-media
            AssetType::Scene => "\u{ea7b}",   // codicon-file
            AssetType::Font => "\u{ea7b}",    // codicon-file
            AssetType::Shader => "\u{ea73}",  // codicon-file-code
            AssetType::Data => "\u{ea73}",    // codicon-file-code
            AssetType::Other => "\u{ea7b}",   // codicon-file
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AssetType::Image => "Image",
            AssetType::Model3D => "3D Model",
            AssetType::Audio => "Audio",
            AssetType::Scene => "Scene",
            AssetType::Font => "Font",
            AssetType::Shader => "Shader",
            AssetType::Data => "Data",
            AssetType::Other => "Other",
        }
    }
}

/// Scan a directory for assets
pub fn scan_assets(root_path: &str, asset_dir: &str) -> Vec<AssetEntry> {
    let full_path = Path::new(root_path).join(asset_dir);
    if !full_path.exists() {
        return Vec::new();
    }

    let mut assets = Vec::new();

    for entry in WalkDir::new(&full_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path().to_path_buf();
        let relative_path = path
            .strip_prefix(root_path)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let asset_type = AssetType::from_extension(&extension);

        assets.push(AssetEntry {
            path,
            relative_path,
            file_name,
            asset_type,
            size_bytes,
            extension,
        });
    }

    // Sort by type, then by name
    assets.sort_by(|a, b| {
        a.asset_type
            .label()
            .cmp(b.asset_type.label())
            .then(a.file_name.cmp(&b.file_name))
    });

    assets
}

/// Format byte size to human-readable string
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
