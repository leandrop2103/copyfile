//! Modern visual theme, status colors, typography and UI styling for CopyFile.

use eframe::egui::{Color32, FontFamily, FontId, Stroke, TextStyle, Vec2, Visuals};

pub const COLOR_SUCCESS: Color32 = Color32::from_rgb(74, 222, 128); // Emerald Green
pub const COLOR_WARNING: Color32 = Color32::from_rgb(251, 191, 36); // Amber / Orange
pub const COLOR_ERROR: Color32 = Color32::from_rgb(248, 113, 113); // Rose Red
pub const COLOR_ACCENT: Color32 = Color32::from_rgb(59, 130, 246); // Blue Accent
pub const COLOR_MUTED: Color32 = Color32::from_rgb(148, 163, 184); // Slate Gray

/// Configures the egui visuals with a modern dark theme and larger, highly legible typography.
pub fn configure_theme(ctx: &eframe::egui::Context) {
    let mut fonts = eframe::egui::FontDefinitions::default();

    // 1. Incorpora a fonte Caskaydia Cove Nerd Font estaticamente no executável
    const NERD_FONT_DATA: &[u8] = include_bytes!("../../assets/CaskaydiaCoveNerdFont-Regular.ttf");
    fonts.font_data.insert(
        "caskaydia_nerd".to_owned(),
        std::sync::Arc::new(eframe::egui::FontData::from_static(NERD_FONT_DATA)),
    );

    // 2. Tenta carregar Segoe UI do Windows para texto nativo, mantendo Caskaydia como fallback para todos os glifos
    #[cfg(windows)]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let font_path = format!("{}\\Fonts\\segoeui.ttf", windir);
        if let Ok(font_data) = std::fs::read(&font_path) {
            fonts.font_data.insert(
                "segoeui".to_owned(),
                std::sync::Arc::new(eframe::egui::FontData::from_owned(font_data)),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "segoeui".to_owned());
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .push("caskaydia_nerd".to_owned());
        } else {
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "caskaydia_nerd".to_owned());
        }
    }

    #[cfg(not(windows))]
    {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "caskaydia_nerd".to_owned());
    }

    // 3. Para Monospace (hashes, caminhos, logs), Caskaydia Cove Nerd Font é a primária absoluta
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "caskaydia_nerd".to_owned());

    ctx.set_fonts(fonts);

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

    // Configure larger fonts for superior readability
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(15.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        ),
        (
            TextStyle::Small,
            FontId::new(13.0, FontFamily::Proportional),
        ),
    ]
    .into();

    // Spacing and button padding
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.scroll = eframe::egui::style::ScrollStyle::solid();

    ctx.set_style(style);
}
