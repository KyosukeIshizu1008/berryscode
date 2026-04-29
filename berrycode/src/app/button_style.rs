//! Reusable button helpers (IntelliJ New UI inspired).
//!
//! The global `Visuals` configured in `setup_egui_style` already gives
//! every plain `ui.button(...)` the right rest / hover / active look,
//! so most call sites need no migration. The helpers here cover the
//! cases that need a different *role* than the default:
//!
//! - [`primary_button`]: accent-blue affirmative action ("Run", "Save").
//! - [`danger_button`]: red destructive action ("Delete", "Force kill").
//! - [`icon_button`]: square button with a codicon glyph — replaces the
//!   inconsistent emoji-glyph buttons (▶ ⏹ ⟲ 🗑) with font-rendered
//!   icons that match the activity-bar style.

/// VS Code Dark+ primary button blue.
const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(14, 99, 156); // #0E639C
const ACCENT_BLUE_HOVER: egui::Color32 = egui::Color32::from_rgb(17, 119, 187); // #1177BB
const ACCENT_BLUE_ACTIVE: egui::Color32 = egui::Color32::from_rgb(13, 85, 137); // #0D5589

/// Destructive red used for "Remove", "Delete", "Force quit".
const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(196, 64, 64); // #C44040
const DANGER_RED_HOVER: egui::Color32 = egui::Color32::from_rgb(220, 76, 76);
const DANGER_RED_ACTIVE: egui::Color32 = egui::Color32::from_rgb(168, 50, 50);

/// Affirmative action button with accent blue background.
pub fn primary_button(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>) -> egui::Response {
    let visuals = ui.visuals_mut();
    let saved = (
        visuals.widgets.inactive.bg_fill,
        visuals.widgets.inactive.weak_bg_fill,
        visuals.widgets.inactive.fg_stroke,
        visuals.widgets.inactive.bg_stroke,
        visuals.widgets.hovered.bg_fill,
        visuals.widgets.hovered.weak_bg_fill,
        visuals.widgets.active.bg_fill,
        visuals.widgets.active.weak_bg_fill,
    );
    visuals.widgets.inactive.bg_fill = ACCENT_BLUE;
    visuals.widgets.inactive.weak_bg_fill = ACCENT_BLUE;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.hovered.bg_fill = ACCENT_BLUE_HOVER;
    visuals.widgets.hovered.weak_bg_fill = ACCENT_BLUE_HOVER;
    visuals.widgets.active.bg_fill = ACCENT_BLUE_ACTIVE;
    visuals.widgets.active.weak_bg_fill = ACCENT_BLUE_ACTIVE;

    let response = ui.button(label);

    let visuals = ui.visuals_mut();
    visuals.widgets.inactive.bg_fill = saved.0;
    visuals.widgets.inactive.weak_bg_fill = saved.1;
    visuals.widgets.inactive.fg_stroke = saved.2;
    visuals.widgets.inactive.bg_stroke = saved.3;
    visuals.widgets.hovered.bg_fill = saved.4;
    visuals.widgets.hovered.weak_bg_fill = saved.5;
    visuals.widgets.active.bg_fill = saved.6;
    visuals.widgets.active.weak_bg_fill = saved.7;
    response
}

/// Destructive action button with red background.
pub fn danger_button(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>) -> egui::Response {
    let visuals = ui.visuals_mut();
    let saved = (
        visuals.widgets.inactive.bg_fill,
        visuals.widgets.inactive.weak_bg_fill,
        visuals.widgets.inactive.fg_stroke,
        visuals.widgets.inactive.bg_stroke,
        visuals.widgets.hovered.bg_fill,
        visuals.widgets.hovered.weak_bg_fill,
        visuals.widgets.active.bg_fill,
        visuals.widgets.active.weak_bg_fill,
    );
    visuals.widgets.inactive.bg_fill = DANGER_RED;
    visuals.widgets.inactive.weak_bg_fill = DANGER_RED;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.hovered.bg_fill = DANGER_RED_HOVER;
    visuals.widgets.hovered.weak_bg_fill = DANGER_RED_HOVER;
    visuals.widgets.active.bg_fill = DANGER_RED_ACTIVE;
    visuals.widgets.active.weak_bg_fill = DANGER_RED_ACTIVE;

    let response = ui.button(label);

    let visuals = ui.visuals_mut();
    visuals.widgets.inactive.bg_fill = saved.0;
    visuals.widgets.inactive.weak_bg_fill = saved.1;
    visuals.widgets.inactive.fg_stroke = saved.2;
    visuals.widgets.inactive.bg_stroke = saved.3;
    visuals.widgets.hovered.bg_fill = saved.4;
    visuals.widgets.hovered.weak_bg_fill = saved.5;
    visuals.widgets.active.bg_fill = saved.6;
    visuals.widgets.active.weak_bg_fill = saved.7;
    response
}

/// Codicon glyph constants reused across panels.
pub mod glyph {
    pub const REFRESH: &str = "\u{eb37}";
    pub const ADD: &str = "\u{ea60}";
    pub const SAVE: &str = "\u{eb4b}";
    pub const TRASH: &str = "\u{ea81}";
    pub const PLAY: &str = "\u{eb2c}";
    pub const STOP: &str = "\u{ead7}";
    pub const RESTART: &str = "\u{ead2}";
    pub const TAG: &str = "\u{ea66}";
    pub const REMOTE: &str = "\u{eb3a}";
    pub const GIT_COMMIT: &str = "\u{eafc}";
    pub const CLOSE: &str = "\u{ea76}";
    pub const FOLDER_OPENED: &str = "\u{eaf7}";
    pub const CLEAR_ALL: &str = "\u{eabf}";
}

/// Default-style button with a codicon glyph prefix. Mixes the
/// codicon font (for the glyph) and the proportional UI font (for
/// the label) inside a single button via `LayoutJob`.
pub fn button_with_icon(ui: &mut egui::Ui, glyph: &str, label: &str) -> egui::Response {
    ui.button(icon_label_job(ui, glyph, label, false))
}

/// Primary (accent-blue) variant of [`button_with_icon`].
pub fn primary_button_with_icon(ui: &mut egui::Ui, glyph: &str, label: &str) -> egui::Response {
    let job = icon_label_job(ui, glyph, label, true);
    primary_button(ui, job)
}

fn icon_label_job(
    ui: &egui::Ui,
    glyph: &str,
    label: &str,
    on_accent: bool,
) -> egui::text::LayoutJob {
    let text_color = if on_accent {
        egui::Color32::WHITE
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    let mut job = egui::text::LayoutJob::default();
    job.append(
        glyph,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(12.0, egui::FontFamily::Name("codicon".into())),
            color: text_color,
            ..Default::default()
        },
    );
    job.append(
        &format!("  {label}"),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(13.0),
            color: text_color,
            ..Default::default()
        },
    );
    job
}

/// Compact square button rendering a single codicon glyph. Use this
/// for inline action toolbars (start / stop / restart / delete). The
/// glyph is rendered through the codicon font so it matches the
/// activity-bar look exactly.
pub fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> egui::Response {
    let text = egui::RichText::new(glyph)
        .family(egui::FontFamily::Name("codicon".into()))
        .size(13.0);
    let response = ui.add_sized(
        egui::vec2(22.0, 22.0),
        egui::Button::new(text).min_size(egui::vec2(22.0, 22.0)),
    );
    if !tooltip.is_empty() {
        response.on_hover_text(tooltip)
    } else {
        response
    }
}
