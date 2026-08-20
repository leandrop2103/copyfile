//! Modern visual theme, status colors, and UI styling for CopyFile.

use eframe::egui::{Color32, Stroke, Visuals};

pub const COLOR_SUCCESS: Color32 = Color32::from_rgb(74, 222, 128); // Emerald Green
pub const COLOR_WARNING: Color32 = Color32::from_rgb(251, 191, 36); // Amber / Orange
pub const COLOR_ERROR: Color32 = Color32::from_rgb(248, 113, 113); // Rose Red
pub const COLOR_ACCENT: Color32 = Color32::from_rgb(59, 130, 246); // Blue Accent
pub const COLOR_MUTED: Color32 = Color32::from_rgb(148, 163, 184); // Slate Gray

/// Configures the egui visuals with a modern dark theme inspired by TeraCopy.
pub fn configure_theme(ctx: &eframe::egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(241, 245, 249));
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(15, 23, 42); // Deep navy/slate
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 41, 59);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(51, 65, 85);
    visuals.widgets.active.bg_fill = Color32::from_rgb(71, 85, 105);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85));
    visuals.window_fill = Color32::from_rgb(15, 23, 42);
    visuals.panel_fill = Color32::from_rgb(15, 23, 42);
    ctx.set_visuals(visuals);
}
