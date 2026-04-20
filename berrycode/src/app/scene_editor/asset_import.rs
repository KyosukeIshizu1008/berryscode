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
            AssetImportSettings::Model { .. } => {
                Ok("Model import settings noted (processing deferred)".to_string())
            }
            AssetImportSettings::Audio { .. } => {
                Ok("Audio import settings noted (processing deferred)".to_string())
            }
            AssetImportSettings::Unknown => Ok("No processing needed".to_string()),
        }
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
