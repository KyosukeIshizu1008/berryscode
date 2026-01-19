//! BerryCode - egui Native Desktop Editor
//! Full implementation using egui_app module

use berry_editor::egui_app::BerryCodeApp;

fn main() -> eframe::Result<()> {
    // Initialize tracing with WGPU log filtering (FIX #2: WGPUログ削減)
    // デフォルトを warn にして、必要なものだけ info レベルに
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("warn")
                .add_directive("berry_editor=info".parse().unwrap())
                .add_directive("berrycode=info".parse().unwrap())
        });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    tracing::info!("🚀 Starting BerryCode egui Native Desktop Editor");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("BerryCode - Native Desktop Editor")
            .with_decorations(true) // Ensure window controls are visible
            .with_transparent(false),
        renderer: eframe::Renderer::Wgpu, // Force WGPU backend
        // FIX #3: Reactive Mode - request_repaint_after()で必要な時だけ再描画
        ..Default::default()
    };

    eframe::run_native(
        "BerryCode",
        options,
        Box::new(|cc| {
            // Setup fonts with Japanese support
            let mut fonts = egui::FontDefinitions::default();

            // Add Codicon font for icons
            if let Ok(font_data) = std::fs::read("assets/codicon.ttf") {
                tracing::info!("✅ Loaded Codicon font: {} bytes", font_data.len());
                fonts.font_data.insert(
                    "codicon".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                fonts.families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "codicon".to_owned());
                tracing::info!("✅ Codicon font added to Proportional family");
            } else {
                tracing::error!("❌ Failed to load Codicon font from assets/codicon.ttf");
            }

            // Add Japanese font (try monospace fonts first for better baseline alignment)
            let japanese_font_paths = vec![
                // Monospace fonts (better baseline alignment)
                "/System/Library/Fonts/Osaka.ttf",  // Osaka (monospace)
                // Fallback to proportional fonts with tweak
                "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
                "/System/Library/Fonts/Hiragino Sans GB.ttc",
                "/Library/Fonts/ヒラギノ角ゴ ProN W3.otf",
            ];

            for path in japanese_font_paths {
                if let Ok(font_data) = std::fs::read(path) {
                    // Use tweak to adjust vertical alignment
                    let mut font_data_with_tweak = egui::FontData::from_owned(font_data);

                    // Adjust y_offset to align Japanese characters with Latin baseline
                    // Positive values move the text down
                    font_data_with_tweak.tweak.y_offset_factor = 0.15;
                    font_data_with_tweak.tweak.y_offset = 2.0;

                    fonts.font_data.insert(
                        "japanese".to_owned(),
                        font_data_with_tweak,
                    );

                    // Add to both Proportional and Monospace families
                    fonts.families
                        .get_mut(&egui::FontFamily::Proportional)
                        .unwrap()
                        .push("japanese".to_owned());

                    fonts.families
                        .get_mut(&egui::FontFamily::Monospace)
                        .unwrap()
                        .push("japanese".to_owned());

                    tracing::info!("✅ Loaded Japanese font: {} (with baseline tweak)", path);
                    break;
                }
            }

            cc.egui_ctx.set_fonts(fonts);

            // Custom dark theme with #191a1c background and unified #D4D4D4 white
            let mut visuals = egui::Visuals::dark();
            visuals.override_text_color = Some(egui::Color32::from_rgb(212, 212, 212)); // #D4D4D4 (unified white)

            // Panel backgrounds - custom dark color #191a1c
            visuals.panel_fill = egui::Color32::from_rgb(25, 26, 28);          // #191A1C
            visuals.window_fill = egui::Color32::from_rgb(25, 26, 28);         // #191A1C
            visuals.extreme_bg_color = egui::Color32::from_rgb(25, 26, 28);    // #191A1C

            // Widget colors - adjusted for darker background
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(25, 26, 28); // #191A1C
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 26, 28);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 47, 50); // #2D2F32 (hover)
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 63, 65); // #3C3F41 (selected)

            // Selection color
            visuals.selection.bg_fill = egui::Color32::from_rgb(60, 63, 65); // #3C3F41
            visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(75, 78, 80));

            // Code editor background - same as panels
            visuals.code_bg_color = egui::Color32::from_rgb(25, 26, 28); // #191A1C

            cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(BerryCodeApp::new(cc)))
        }),
    )
}
