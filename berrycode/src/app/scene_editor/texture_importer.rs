//! Texture atlas packing and compression settings.

use serde::{Deserialize, Serialize};

use crate::app::BerryCodeApp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureAtlas {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub entries: Vec<AtlasEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasEntry {
    pub name: String,
    pub source_path: String,
    pub rect: [u32; 4], // x, y, w, h
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureCompressionSettings {
    pub format: CompressionFormat,
    pub quality: u8, // 0-100
    pub generate_mipmaps: bool,
    pub max_size: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CompressionFormat {
    None,
    Bc1,
    Bc3,
    Bc7,
    Astc4x4,
    Astc6x6,
}

pub struct TextureImporterState {
    pub open: bool,
    pub atlas: TextureAtlas,
    pub compression: TextureCompressionSettings,
    pub new_entry_name: String,
    pub new_entry_path: String,
}

impl Default for TextureImporterState {
    fn default() -> Self {
        Self {
            open: false,
            atlas: TextureAtlas {
                name: "main_atlas".into(),
                width: 2048,
                height: 2048,
                entries: Vec::new(),
            },
            compression: TextureCompressionSettings {
                format: CompressionFormat::Bc7,
                quality: 80,
                generate_mipmaps: true,
                max_size: 4096,
            },
            new_entry_name: String::new(),
            new_entry_path: String::new(),
        }
    }
}

impl TextureAtlas {
    /// Simple shelf-based rect packing. Returns the assigned rect or None if full.
    pub fn pack_entry(
        &mut self,
        name: String,
        source_path: String,
        w: u32,
        h: u32,
    ) -> Option<[u32; 4]> {
        let rect = self.find_free_rect(w, h)?;
        self.entries.push(AtlasEntry {
            name,
            source_path,
            rect,
        });
        Some(rect)
    }

    fn find_free_rect(&self, w: u32, h: u32) -> Option<[u32; 4]> {
        if w > self.width || h > self.height {
            return None;
        }
        // Simple row-based scan
        let mut y = 0u32;
        while y + h <= self.height {
            let mut x = 0u32;
            let row_height = h;
            'try_x: while x + w <= self.width {
                let candidate = [x, y, w, h];
                for e in &self.entries {
                    if rects_overlap(candidate, e.rect) {
                        x = e.rect[0] + e.rect[2];
                        continue 'try_x;
                    }
                }
                return Some(candidate);
            }
            // Advance y past tallest entry in this row
            let mut max_h = row_height;
            for e in &self.entries {
                if e.rect[1] >= y && e.rect[1] < y + row_height {
                    max_h = max_h.max(e.rect[3]);
                }
            }
            y += max_h;
        }
        None
    }
}

fn rects_overlap(a: [u32; 4], b: [u32; 4]) -> bool {
    a[0] < b[0] + b[2] && a[0] + a[2] > b[0] && a[1] < b[1] + b[3] && a[1] + a[3] > b[1]
}

impl BerryCodeApp {
    pub(crate) fn render_texture_importer(&mut self, ctx: &egui::Context) {
        if !self.texture_importer.open {
            return;
        }
        let mut open = self.texture_importer.open;
        egui::Window::new("Texture Importer")
            .open(&mut open)
            .default_size([420.0, 360.0])
            .show(ctx, |ui| {
                ui.heading("Atlas");
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.texture_importer.atlas.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.add(
                        egui::DragValue::new(&mut self.texture_importer.atlas.width)
                            .range(64..=8192),
                    );
                    ui.label("x");
                    ui.add(
                        egui::DragValue::new(&mut self.texture_importer.atlas.height)
                            .range(64..=8192),
                    );
                });
                ui.separator();
                ui.label(format!(
                    "Entries: {}",
                    self.texture_importer.atlas.entries.len()
                ));
                for e in &self.texture_importer.atlas.entries {
                    ui.label(format!(
                        "  {} @ [{},{} {}x{}]",
                        e.name, e.rect[0], e.rect[1], e.rect[2], e.rect[3]
                    ));
                }
                ui.separator();
                ui.heading("Compression");
                egui::ComboBox::from_label("Format")
                    .selected_text(format!("{:?}", self.texture_importer.compression.format))
                    .show_ui(ui, |ui| {
                        for fmt in [
                            CompressionFormat::None,
                            CompressionFormat::Bc1,
                            CompressionFormat::Bc3,
                            CompressionFormat::Bc7,
                            CompressionFormat::Astc4x4,
                            CompressionFormat::Astc6x6,
                        ] {
                            ui.selectable_value(
                                &mut self.texture_importer.compression.format,
                                fmt,
                                format!("{:?}", fmt),
                            );
                        }
                    });
                ui.add(
                    egui::Slider::new(&mut self.texture_importer.compression.quality, 0..=100)
                        .text("Quality"),
                );
                ui.checkbox(
                    &mut self.texture_importer.compression.generate_mipmaps,
                    "Generate Mipmaps",
                );
                ui.add(
                    egui::DragValue::new(&mut self.texture_importer.compression.max_size)
                        .range(64..=8192)
                        .prefix("Max: "),
                );
            });
        self.texture_importer.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_single_entry() {
        let mut atlas = TextureAtlas {
            name: "test".into(),
            width: 256,
            height: 256,
            entries: vec![],
        };
        let rect = atlas.pack_entry("a".into(), "a.png".into(), 64, 64);
        assert_eq!(rect, Some([0, 0, 64, 64]));
        assert_eq!(atlas.entries.len(), 1);
    }

    #[test]
    fn pack_multiple_no_overlap() {
        let mut atlas = TextureAtlas {
            name: "test".into(),
            width: 256,
            height: 256,
            entries: vec![],
        };
        let r1 = atlas
            .pack_entry("a".into(), "a.png".into(), 128, 128)
            .unwrap();
        let r2 = atlas
            .pack_entry("b".into(), "b.png".into(), 128, 128)
            .unwrap();
        assert!(!rects_overlap(r1, r2));
    }

    #[test]
    fn pack_entry_too_large() {
        let mut atlas = TextureAtlas {
            name: "test".into(),
            width: 64,
            height: 64,
            entries: vec![],
        };
        assert!(atlas
            .pack_entry("big".into(), "big.png".into(), 128, 128)
            .is_none());
    }

    #[test]
    fn rects_overlap_cases() {
        assert!(rects_overlap([0, 0, 10, 10], [5, 5, 10, 10]));
        assert!(!rects_overlap([0, 0, 10, 10], [10, 0, 10, 10]));
        assert!(!rects_overlap([0, 0, 10, 10], [0, 10, 10, 10]));
    }

    #[test]
    fn compression_defaults() {
        let s = TextureCompressionSettings {
            format: CompressionFormat::Bc7,
            quality: 80,
            generate_mipmaps: true,
            max_size: 4096,
        };
        assert_eq!(s.format, CompressionFormat::Bc7);
        assert!(s.generate_mipmaps);
    }
}
