//! Image preview for image files (png, jpg, gif, webp, bmp, ico)

use super::BerryCodeApp;

impl BerryCodeApp {
    /// Load an image file into an egui texture handle
    pub(crate) fn load_image_texture(
        ctx: &egui::Context,
        file_path: &str,
    ) -> Option<egui::TextureHandle> {
        let image_data = std::fs::read(file_path).ok()?;

        let ext = file_path.rsplit('.').next().unwrap_or("").to_lowercase();

        let (size, pixels) = if ext == "svg" {
            // SVG rendering using resvg
            let svg_str = String::from_utf8(image_data).ok()?;
            let opts = resvg::usvg::Options::default();
            let tree = resvg::usvg::Tree::from_str(&svg_str, &opts).ok()?;
            let tree_size = tree.size();
            let w = tree_size.width() as u32;
            let h = tree_size.height() as u32;
            // Render at 2x for retina
            let scale = 2.0_f32;
            let pw = (w as f32 * scale) as u32;
            let ph = (h as f32 * scale) as u32;
            let mut pixmap = resvg::tiny_skia::Pixmap::new(pw, ph)?;
            let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
            resvg::render(&tree, transform, &mut pixmap.as_mut());
            let rgba_data = pixmap.data().to_vec();
            ([pw as usize, ph as usize], rgba_data)
        } else {
            // Raster image decoding
            let img = image::load_from_memory(&image_data).ok()?;
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.into_raw();
            (size, pixels)
        };

        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
        let texture = ctx.load_texture(file_path, color_image, egui::TextureOptions::LINEAR);

        Some(texture)
    }

    /// Render image preview for the active tab.
    /// The caller must ensure `self.active_tab_idx` points to an image tab.
    pub(crate) fn render_image_preview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let tab = &mut self.editor_tabs[self.active_tab_idx];

        // Load texture on first render
        if tab.image_texture.is_none() {
            tab.image_texture = Self::load_image_texture(ctx, &tab.file_path);
        }

        match &tab.image_texture {
            Some(texture) => {
                let tex_size = texture.size_vec2();

                // Header with file info
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{}x{}",
                        tex_size.x as u32, tex_size.y as u32
                    ));
                    ui.separator();
                    let file_size = std::fs::metadata(&tab.file_path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    if file_size < 1024 {
                        ui.label(format!("{} B", file_size));
                    } else if file_size < 1024 * 1024 {
                        ui.label(format!("{:.1} KB", file_size as f64 / 1024.0));
                    } else {
                        ui.label(format!(
                            "{:.1} MB",
                            file_size as f64 / (1024.0 * 1024.0)
                        ));
                    }
                });
                ui.separator();

                // Centered image display with fit-to-view
                egui::ScrollArea::both().show(ui, |ui| {
                    let available = ui.available_size();
                    let scale = (available.x / tex_size.x)
                        .min(available.y / tex_size.y)
                        .min(1.0);
                    let display_size = egui::vec2(tex_size.x * scale, tex_size.y * scale);

                    // Center the image
                    let padding_x = (available.x - display_size.x).max(0.0) / 2.0;
                    let padding_y = (available.y - display_size.y).max(0.0) / 2.0;
                    ui.add_space(padding_y);
                    ui.horizontal(|ui| {
                        ui.add_space(padding_x);
                        ui.image(egui::load::SizedTexture::new(
                            texture.id(),
                            display_size,
                        ));
                    });
                });
            }
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("Cannot preview this image");
                    ui.label(&tab.file_path);

                    ui.label("Unsupported or corrupted image format");
                });
            }
        }
    }
}
