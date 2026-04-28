//! Asset import settings stored as .meta sidecar files.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetImportSettings {
    Texture {
        #[serde(default = "default_max_size")]
        max_size: u32,
        #[serde(default)]
        generate_mipmaps: bool,
        #[serde(default = "default_filter")]
        filter_mode: String,
    },
    Model {
        #[serde(default = "default_scale_factor")]
        scale_factor: f32,
        #[serde(default)]
        flip_uvs: bool,
        #[serde(default = "default_true_import")]
        import_materials: bool,
    },
    Audio {
        #[serde(default = "default_sample_rate")]
        sample_rate: u32,
        #[serde(default)]
        force_mono: bool,
    },
    Unknown,
}

fn default_max_size() -> u32 {
    2048
}
fn default_filter() -> String {
    "Linear".to_string()
}
fn default_scale_factor() -> f32 {
    1.0
}
fn default_true_import() -> bool {
    true
}
fn default_sample_rate() -> u32 {
    44100
}

impl AssetImportSettings {
    /// Detect the appropriate settings type from a file extension.
    pub fn for_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "hdr" | "exr" => {
                AssetImportSettings::Texture {
                    max_size: default_max_size(),
                    generate_mipmaps: false,
                    filter_mode: default_filter(),
                }
            }
            "glb" | "gltf" | "obj" | "stl" | "ply" => AssetImportSettings::Model {
                scale_factor: default_scale_factor(),
                flip_uvs: false,
                import_materials: true,
            },
            "wav" | "ogg" | "mp3" | "flac" => AssetImportSettings::Audio {
                sample_rate: default_sample_rate(),
                force_mono: false,
            },
            _ => AssetImportSettings::Unknown,
        }
    }

    /// Load .meta file for an asset. Returns default settings if no meta exists.
    pub fn load(asset_path: &str) -> Self {
        let meta_path = format!("{}.meta", asset_path);
        if let Ok(content) = std::fs::read_to_string(&meta_path) {
            if let Ok(settings) = ron::from_str::<AssetImportSettings>(&content) {
                return settings;
            }
        }
        let ext = asset_path.rsplit('.').next().unwrap_or("");
        Self::for_extension(ext)
    }

    /// Save .meta file for an asset.
    pub fn save(&self, asset_path: &str) -> Result<(), String> {
        let meta_path = format!("{}.meta", asset_path);
        let s = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| e.to_string())?;
        std::fs::write(&meta_path, s).map_err(|e| e.to_string())
    }

    /// Process the asset according to the current import settings.
    ///
    /// For textures this resizes to `max_size` (if the image exceeds it) and
    /// optionally generates mipmaps.  Model / Audio processing is deferred.
    ///
    /// `dead_code` is allowed because the only current callers are unit
    /// tests in this module — the asset browser UI that will invoke this
    /// in production is still scaffolding (see `app::asset_browser`).
    #[allow(dead_code)]
    pub fn process(&self, asset_path: &str) -> Result<String, String> {
        match self {
            AssetImportSettings::Texture {
                max_size,
                generate_mipmaps,
                ..
            } => {
                let img =
                    image::open(asset_path).map_err(|e| format!("Failed to open image: {e}"))?;

                let (w, h) = (img.width(), img.height());
                let limit = *max_size;

                let img = if w > limit || h > limit {
                    let ratio = (limit as f64 / w as f64).min(limit as f64 / h as f64);
                    let new_w = (w as f64 * ratio).round() as u32;
                    let new_h = (h as f64 * ratio).round() as u32;
                    image::DynamicImage::ImageRgba8(image::imageops::resize(
                        &img,
                        new_w,
                        new_h,
                        image::imageops::FilterType::Lanczos3,
                    ))
                } else {
                    img
                };

                let (final_w, final_h) = (img.width(), img.height());

                // Generate simple mipmap chain (half-size levels saved alongside)
                if *generate_mipmaps {
                    let stem = asset_path
                        .rsplit_once('.')
                        .map(|(s, _)| s)
                        .unwrap_or(asset_path);
                    let ext = asset_path.rsplit('.').next().unwrap_or("png");
                    let mut mip = img.clone();
                    let mut level = 1u32;
                    loop {
                        let mw = (mip.width() / 2).max(1);
                        let mh = (mip.height() / 2).max(1);
                        if mw == 0 || mh == 0 {
                            break;
                        }
                        mip = image::DynamicImage::ImageRgba8(image::imageops::resize(
                            &mip,
                            mw,
                            mh,
                            image::imageops::FilterType::Lanczos3,
                        ));
                        let mip_path = format!("{}_mip{}.{}", stem, level, ext);
                        mip.save(&mip_path)
                            .map_err(|e| format!("Failed to save mip level {level}: {e}"))?;
                        level += 1;
                        if mw <= 1 && mh <= 1 {
                            break;
                        }
                    }
                }

                // Save (overwrite) the main image
                img.save(asset_path)
                    .map_err(|e| format!("Failed to save processed image: {e}"))?;

                if w > limit || h > limit {
                    Ok(format!("Resized to {}x{}", final_w, final_h))
                } else {
                    Ok(format!(
                        "Image already within limits ({}x{})",
                        final_w, final_h
                    ))
                }
            }
            AssetImportSettings::Model {
                scale_factor,
                flip_uvs: _,
                import_materials: _,
            } => process_model(asset_path, *scale_factor),
            AssetImportSettings::Audio { .. } => {
                Ok("Audio import settings noted (processing deferred)".to_string())
            }
            AssetImportSettings::Unknown => Ok("No processing needed".to_string()),
        }
    }
}

/// Validate-and-stat for OBJ / STL / PLY / glTF / glb files. Returns a
/// human-readable summary (vertex / triangle / submesh counts, bounds
/// scaled by the import setting's `scale_factor`) so the caller can
/// surface it in a status bar or asset inspector.
///
/// Format coverage:
/// - **OBJ** uses the project's existing `tobj` dependency.
/// - **STL / PLY / glTF / glb** delegate to the loader the model
///   preview already uses, keeping format support in one place.
///
/// Conversion (e.g. OBJ → glb) is a future expansion point — the
/// pipeline trait below is the seam to bolt that onto without
/// touching this function.
fn process_model(asset_path: &str, scale_factor: f32) -> Result<String, String> {
    let ext = asset_path.rsplit('.').next().unwrap_or("").to_lowercase();

    match ext.as_str() {
        "obj" => {
            let load_options = tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ignore_lines: true,
                ignore_points: true,
            };
            let (models, _materials) = tobj::load_obj(asset_path, &load_options)
                .map_err(|e| format!("Failed to load OBJ: {e}"))?;

            let mut total_vertices = 0u32;
            let mut total_triangles = 0u32;
            let (mut min_xyz, mut max_xyz) = ([f32::MAX; 3], [f32::MIN; 3]);

            for m in &models {
                total_vertices += (m.mesh.positions.len() / 3) as u32;
                total_triangles += (m.mesh.indices.len() / 3) as u32;
                for chunk in m.mesh.positions.chunks_exact(3) {
                    for i in 0..3 {
                        if chunk[i] < min_xyz[i] {
                            min_xyz[i] = chunk[i];
                        }
                        if chunk[i] > max_xyz[i] {
                            max_xyz[i] = chunk[i];
                        }
                    }
                }
            }

            // Scaled bounds match what Bevy will see after the import
            // applies `scale_factor` — useful for spotting models that
            // are 100× too big without opening the scene editor.
            let bounds = if total_vertices > 0 {
                let dx = (max_xyz[0] - min_xyz[0]) * scale_factor;
                let dy = (max_xyz[1] - min_xyz[1]) * scale_factor;
                let dz = (max_xyz[2] - min_xyz[2]) * scale_factor;
                format!("{:.2} × {:.2} × {:.2}", dx, dy, dz)
            } else {
                "(empty mesh)".to_string()
            };

            Ok(format!(
                "OBJ: {} sub-mesh(es), {} verts, {} tris, bounds {}",
                models.len(),
                total_vertices,
                total_triangles,
                bounds
            ))
        }
        "stl" | "ply" | "glb" | "gltf" => {
            // Stat using the same loader the wireframe preview uses
            // so we always agree on the numbers between import-stat
            // and model-preview rendering.
            let data = crate::app::BerryCodeApp::load_model_data(asset_path)
                .ok_or_else(|| format!("Failed to load {ext} model"))?;
            let dx = (data.bounds_max[0] - data.bounds_min[0]) * scale_factor;
            let dy = (data.bounds_max[1] - data.bounds_min[1]) * scale_factor;
            let dz = (data.bounds_max[2] - data.bounds_min[2]) * scale_factor;
            Ok(format!(
                "{}: {} mesh(es), {} verts, {} tris, bounds {:.2} × {:.2} × {:.2}",
                ext.to_uppercase(),
                data.meshes.len(),
                data.vertices.len(),
                data.triangles.len(),
                dx,
                dy,
                dz
            ))
        }
        other => Err(format!(
            "Unsupported model format: .{other} (supported: obj, stl, ply, gltf, glb)"
        )),
    }
}

/// Trait for plugin-style asset converters. Crates that want to ship
/// custom format support (FBX, USD, custom voxel, …) implement this
/// and register their impl with [`AssetImportRegistry`]. Kept minimal
/// for now — the pipeline only knows about validation and conversion;
/// re-encoding (e.g. OBJ → glb) is a follow-up.
///
/// `dead_code` allowed because v0.5 ships the trait + scan/process
/// path; the actual third-party registrations land alongside the
/// asset-import-pipeline polish in v0.6.
#[allow(dead_code)]
pub trait AssetConverter: Send + Sync {
    /// Lower-case extensions this converter recognises.
    fn extensions(&self) -> &[&str];
    /// Quick validate / stat pass — analogous to [`process_model`]
    /// but for arbitrary formats. Returns a human-readable summary.
    fn validate(&self, asset_path: &str) -> Result<String, String>;
    /// Convert the asset to a Bevy-friendly format (e.g. glb).
    /// `output_path` is where the converted file should be written.
    /// Default impl returns an error so converters opt in to the
    /// conversion side without being forced to implement it.
    fn convert(&self, _asset_path: &str, _output_path: &str) -> Result<(), String> {
        Err("conversion not implemented for this converter".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_texture() {
        assert!(matches!(
            AssetImportSettings::for_extension("png"),
            AssetImportSettings::Texture { .. }
        ));
    }

    #[test]
    fn detect_model() {
        assert!(matches!(
            AssetImportSettings::for_extension("glb"),
            AssetImportSettings::Model { .. }
        ));
    }

    #[test]
    fn detect_audio() {
        assert!(matches!(
            AssetImportSettings::for_extension("wav"),
            AssetImportSettings::Audio { .. }
        ));
    }

    #[test]
    fn detect_unknown() {
        assert!(matches!(
            AssetImportSettings::for_extension("xyz"),
            AssetImportSettings::Unknown
        ));
    }

    #[test]
    fn process_texture_resize() {
        // Create a temporary 256x256 red PNG image
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("test.png");
        let img = image::RgbaImage::from_pixel(256, 256, image::Rgba([255, 0, 0, 255]));
        img.save(&img_path).unwrap();

        let settings = AssetImportSettings::Texture {
            max_size: 128,
            generate_mipmaps: false,
            filter_mode: "Linear".to_string(),
        };

        let result = settings.process(img_path.to_str().unwrap());
        assert!(result.is_ok(), "process should succeed: {:?}", result);
        let msg = result.unwrap();
        assert!(
            msg.contains("Resized to"),
            "expected resize message, got: {}",
            msg
        );

        // Verify the saved image dimensions
        let processed = image::open(&img_path).unwrap();
        assert!(
            processed.width() <= 128,
            "width should be <= 128, got {}",
            processed.width()
        );
        assert!(
            processed.height() <= 128,
            "height should be <= 128, got {}",
            processed.height()
        );
    }

    #[test]
    fn process_unknown_returns_ok() {
        let settings = AssetImportSettings::Unknown;
        let result = settings.process("/nonexistent/file.xyz");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "No processing needed");
    }
}
