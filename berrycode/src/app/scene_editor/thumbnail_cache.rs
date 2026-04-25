#![allow(dead_code)]
//! Lazy thumbnail cache for asset previews.

use std::collections::HashMap;

pub struct ThumbnailCache {
    cache: HashMap<String, Option<egui::TextureHandle>>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Get or generate a thumbnail for the given file path.
    /// Returns None if the file is not a supported image type or failed to load.
    pub fn get_or_create(
        &mut self,
        ctx: &egui::Context,
        path: &str,
    ) -> Option<&egui::TextureHandle> {
        if !self.cache.contains_key(path) {
            let thumb = Self::generate_thumbnail(ctx, path);
            self.cache.insert(path.to_string(), thumb);
        }
        self.cache.get(path).and_then(|opt| opt.as_ref())
    }

    fn generate_thumbnail(ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
        let ext = path.rsplit('.').next()?.to_lowercase();

        // 3D model formats: generate colored placeholder icons
        if matches!(ext.as_str(), "glb" | "gltf" | "obj" | "stl" | "ply") {
            let color_image = Self::generate_model_placeholder(&ext);
            return Some(ctx.load_texture(
                format!("thumb_{}", path),
                color_image,
                egui::TextureOptions::LINEAR,
            ));
        }

        if !matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp"
        ) {
            return None;
        }

        let img = image::open(path).ok()?;
        let thumb = img.thumbnail(48, 48);
        let rgba = thumb.to_rgba8();
        let (w, h) = (rgba.width() as usize, rgba.height() as usize);
        let pixels = rgba.into_raw();

        let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
        Some(ctx.load_texture(
            format!("thumb_{}", path),
            color_image,
            egui::TextureOptions::LINEAR,
        ))
    }

    /// Generate a colored placeholder icon for 3D model files.
    /// Each format gets a distinctive color with a simple 3D box shape.
    fn generate_model_placeholder(ext: &str) -> egui::ColorImage {
        let (r, g, b) = match ext {
            "glb" | "gltf" => (220, 140, 40),
            "obj" => (60, 120, 220),
            "stl" => (60, 180, 80),
            "ply" => (160, 80, 200),
            _ => (120, 120, 120),
        };
        let size = 48;
        let bg = egui::Color32::from_rgb(30, 32, 36);
        let mut pixels = vec![bg; size * size];

        // Draw a simple 3D box icon: lighter top face, darker side face
        for y in 8..40 {
            for x in 8..40 {
                if y < 24 && x >= 12 && x < 36 {
                    // Top face: brighter
                    pixels[y * size + x] = egui::Color32::from_rgb(
                        (r as u16 * 120 / 100).min(255) as u8,
                        (g as u16 * 120 / 100).min(255) as u8,
                        (b as u16 * 120 / 100).min(255) as u8,
                    );
                } else if y >= 24 {
                    // Side face: darker
                    pixels[y * size + x] = egui::Color32::from_rgb(
                        (r as u16 * 80 / 100) as u8,
                        (g as u16 * 80 / 100) as u8,
                        (b as u16 * 80 / 100) as u8,
                    );
                }
            }
        }

        egui::ColorImage {
            size: [size, size],
            pixels,
            source_size: egui::Vec2::new(size as f32, size as f32),
        }
    }

    /// Clear the cache (e.g., when files change).
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Returns true if the given file extension is a supported 3D model format.
    pub fn is_model_extension(ext: &str) -> bool {
        matches!(ext, "glb" | "gltf" | "obj" | "stl" | "ply")
    }

    /// Returns true if the given file extension is a supported image format.
    pub fn is_image_extension(ext: &str) -> bool {
        matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    }

    /// Returns true if the given file extension is any supported thumbnail format
    /// (image or 3D model).
    pub fn is_supported_extension(ext: &str) -> bool {
        Self::is_model_extension(ext) || Self::is_image_extension(ext)
    }

    /// Get the color associated with a model format for placeholder rendering.
    pub fn model_format_color(ext: &str) -> (u8, u8, u8) {
        match ext {
            "glb" | "gltf" => (220, 140, 40),
            "obj" => (60, 120, 220),
            "stl" => (60, 180, 80),
            "ply" => (160, 80, 200),
            _ => (120, 120, 120),
        }
    }

    /// Returns the number of cached entries.
    pub fn cached_count(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_placeholder_glb() {
        let img = ThumbnailCache::generate_model_placeholder("glb");
        assert_eq!(img.size, [48, 48]);
        assert_eq!(img.pixels.len(), 48 * 48);
        // Background pixel at (0,0)
        assert_eq!(img.pixels[0], egui::Color32::from_rgb(30, 32, 36));
        // Top face pixel at (14, 16) should be bright orange
        let top = img.pixels[16 * 48 + 14];
        assert_ne!(top, egui::Color32::from_rgb(30, 32, 36));
        // Side face pixel at (14, 30) should be darker orange
        let side = img.pixels[30 * 48 + 14];
        assert_ne!(side, egui::Color32::from_rgb(30, 32, 36));
    }

    #[test]
    fn test_model_placeholder_gltf() {
        let img = ThumbnailCache::generate_model_placeholder("gltf");
        assert_eq!(img.size, [48, 48]);
        // gltf should share the same color as glb
        let glb_img = ThumbnailCache::generate_model_placeholder("glb");
        assert_eq!(img.pixels[16 * 48 + 14], glb_img.pixels[16 * 48 + 14]);
    }

    #[test]
    fn test_model_placeholder_obj() {
        let img = ThumbnailCache::generate_model_placeholder("obj");
        assert_eq!(img.size, [48, 48]);
        let top = img.pixels[16 * 48 + 14];
        // obj is blue-ish: g > r
        assert!(top.g() > top.r());
    }

    #[test]
    fn test_model_placeholder_stl() {
        let img = ThumbnailCache::generate_model_placeholder("stl");
        assert_eq!(img.size, [48, 48]);
        let top = img.pixels[16 * 48 + 14];
        // stl is green-ish: g > r and g > b
        assert!(top.g() > top.r());
        assert!(top.g() > top.b());
    }

    #[test]
    fn test_model_placeholder_ply() {
        let img = ThumbnailCache::generate_model_placeholder("ply");
        assert_eq!(img.size, [48, 48]);
        let top = img.pixels[16 * 48 + 14];
        // ply is purple-ish: r > g and b > g
        assert!(top.r() > top.g());
        assert!(top.b() > top.g());
    }

    #[test]
    fn test_model_placeholder_unknown_ext() {
        let img = ThumbnailCache::generate_model_placeholder("xyz");
        assert_eq!(img.size, [48, 48]);
        // Falls back to gray
        let top = img.pixels[16 * 48 + 14];
        assert_eq!(top.r(), top.g());
    }

    #[test]
    fn test_is_model_extension() {
        assert!(ThumbnailCache::is_model_extension("glb"));
        assert!(ThumbnailCache::is_model_extension("gltf"));
        assert!(ThumbnailCache::is_model_extension("obj"));
        assert!(ThumbnailCache::is_model_extension("stl"));
        assert!(ThumbnailCache::is_model_extension("ply"));
        assert!(!ThumbnailCache::is_model_extension("png"));
        assert!(!ThumbnailCache::is_model_extension("rs"));
        assert!(!ThumbnailCache::is_model_extension(""));
    }

    #[test]
    fn test_is_image_extension() {
        assert!(ThumbnailCache::is_image_extension("png"));
        assert!(ThumbnailCache::is_image_extension("jpg"));
        assert!(ThumbnailCache::is_image_extension("jpeg"));
        assert!(ThumbnailCache::is_image_extension("gif"));
        assert!(ThumbnailCache::is_image_extension("webp"));
        assert!(ThumbnailCache::is_image_extension("bmp"));
        assert!(!ThumbnailCache::is_image_extension("glb"));
        assert!(!ThumbnailCache::is_image_extension("rs"));
        assert!(!ThumbnailCache::is_image_extension(""));
    }

    #[test]
    fn test_is_supported_extension() {
        // Models
        assert!(ThumbnailCache::is_supported_extension("glb"));
        assert!(ThumbnailCache::is_supported_extension("obj"));
        // Images
        assert!(ThumbnailCache::is_supported_extension("png"));
        assert!(ThumbnailCache::is_supported_extension("jpg"));
        // Unsupported
        assert!(!ThumbnailCache::is_supported_extension("rs"));
        assert!(!ThumbnailCache::is_supported_extension("toml"));
    }

    #[test]
    fn test_model_format_color() {
        assert_eq!(ThumbnailCache::model_format_color("glb"), (220, 140, 40));
        assert_eq!(ThumbnailCache::model_format_color("gltf"), (220, 140, 40));
        assert_eq!(ThumbnailCache::model_format_color("obj"), (60, 120, 220));
        assert_eq!(ThumbnailCache::model_format_color("stl"), (60, 180, 80));
        assert_eq!(ThumbnailCache::model_format_color("ply"), (160, 80, 200));
        assert_eq!(ThumbnailCache::model_format_color("xyz"), (120, 120, 120));
    }

    #[test]
    fn test_cached_count() {
        let cache = ThumbnailCache::new();
        assert_eq!(cache.cached_count(), 0);
    }

    #[test]
    fn test_clear() {
        let mut cache = ThumbnailCache::new();
        cache.clear();
        assert_eq!(cache.cached_count(), 0);
    }
}
