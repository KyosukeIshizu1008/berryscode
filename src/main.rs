//! BerryCode - 100% Rust Native Desktop Code Editor
//! Built with egui + eframe (Pure Native, No WebView)

use berry_editor::egui_app::BerryCodeApp;

fn main() -> eframe::Result<()> {
    // Initialize tracing for debugging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("🚀 Starting BerryCode Native Desktop Editor (egui)");

    // Configure fonts (Codicon for icons)
    let mut fonts = egui::FontDefinitions::default();

    // Load Codicon font for icons
    const CODICON_FONT_BYTES: &[u8] = include_bytes!("../assets/codicon.ttf");
    fonts.font_data.insert(
        "codicon".to_owned(),
        egui::FontData::from_static(CODICON_FONT_BYTES),
    );

    // Insert Codicon font at the beginning of the Proportional family
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "codicon".to_owned());

    tracing::info!("📦 Codicon font embedded: {} bytes", CODICON_FONT_BYTES.len());

    // Configure eframe window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("BerryCode - Rust Native Editor")
            .with_resizable(true),
        renderer: eframe::Renderer::Wgpu, // Use WGPU backend for performance
        ..Default::default()
    };

    // Launch egui app
    eframe::run_native(
        "BerryCode",
        options,
        Box::new(move |cc| {
            // Apply custom fonts
            cc.egui_ctx.set_fonts(fonts);

            // Apply dark theme (IntelliJ Darcula-inspired)
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            tracing::info!("✅ egui context initialized");

            Ok(Box::new(BerryCodeApp::new(cc)))
        }),
    )
}
