//! Modern visual theme (Dark, Light, and Automatic System follow), semantic palette, and typography for CopyFile.

use eframe::egui::{
    self, Color32, FontFamily, FontId, Stroke, TextStyle, Theme, ThemePreference, Vec2, Visuals,
};
use std::path::PathBuf;

// Legacy dark-mode constants retained for any external callers
pub const COLOR_SUCCESS: Color32 = Color32::from_rgb(74, 222, 128);
pub const COLOR_WARNING: Color32 = Color32::from_rgb(251, 191, 36);
pub const COLOR_ERROR: Color32 = Color32::from_rgb(248, 113, 113);
pub const COLOR_ACCENT: Color32 = Color32::from_rgb(59, 130, 246);
pub const COLOR_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

/// Selectable UI theme mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Follows Windows Light/Dark theme automatically (default).
    #[default]
    System,
    /// Forces Light Mode regardless of OS setting.
    Light,
    /// Forces Dark Mode regardless of OS setting.
    Dark,
}

impl ThemeMode {
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::System, Self::Light, Self::Dark]
    }

    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::System => "🌗 Automático (Sistema)",
            Self::Light => "☀ Modo Claro",
            Self::Dark => "🌙 Modo Escuro",
        }
    }

    #[must_use]
    pub const fn short_label(&self) -> &'static str {
        match self {
            Self::System => "🌗 Auto",
            Self::Light => "☀ Claro",
            Self::Dark => "🌙 Escuro",
        }
    }

    /// Cycles cleanly: System -> Light -> Dark -> System (or flips directly away from current effective mode).
    #[must_use]
    pub const fn next_cycle(self, currently_dark: bool) -> Self {
        match self {
            Self::System => {
                if currently_dark {
                    Self::Light
                } else {
                    Self::Dark
                }
            }
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }

    #[must_use]
    pub const fn to_preference(self) -> ThemePreference {
        match self {
            Self::System => ThemePreference::System,
            Self::Light => ThemePreference::Light,
            Self::Dark => ThemePreference::Dark,
        }
    }

    fn config_path() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("CopyFile").join("theme.txt"))
        }
        #[cfg(not(windows))]
        {
            std::env::var_os("HOME")
                .map(|p| PathBuf::from(p).join(".config").join("copyfile_theme.txt"))
        }
    }

    /// Loads saved theme preference from disk, defaulting to `ThemeMode::System`.
    #[must_use]
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::System;
        };
        match std::fs::read_to_string(path).ok().as_deref().map(str::trim) {
            Some("light") => Self::Light,
            Some("dark") => Self::Dark,
            _ => Self::System,
        }
    }

    /// Persists theme preference to disk silently.
    pub fn save(self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let val = match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        };
        let _ = std::fs::write(path, val);
    }
}

/// Complete theme-aware color palette with high contrast for both Dark and Light modes.
#[derive(Debug, Clone, Copy)]
pub struct AppPalette {
    pub is_dark: bool,
    pub bg_window: Color32,
    pub bg_header: Color32,
    pub bg_card: Color32,
    pub bg_card_inner: Color32,
    pub bg_badge: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub accent: Color32,
    pub dest_bg: Color32,
    pub dest_border: Color32,
    pub dest_text: Color32,
    pub dest_subtext: Color32,
    pub alert_error_bg: Color32,
    pub btn_disabled_bg: Color32,
    pub btn_secondary_bg: Color32,
    pub btn_secondary_text: Color32,
    pub graph_bg: Color32,
    pub graph_grid: Color32,
    pub graph_line: Color32,
    pub graph_fill_top: Color32,
    pub graph_fill_bottom: Color32,
    pub graph_avg_line: Color32,
}

impl AppPalette {
    /// Returns the semantic color palette matching the active egui theme (`Dark` or `Light`).
    #[must_use]
    pub fn from_dark_mode(is_dark: bool) -> Self {
        if is_dark {
            Self {
                is_dark: true,
                bg_window: Color32::from_rgb(15, 23, 42), // Slate 900
                bg_header: Color32::from_rgb(24, 33, 47), // Slate 850
                bg_card: Color32::from_rgb(24, 33, 47),   // Slate 850
                bg_card_inner: Color32::from_rgb(30, 41, 59), // Slate 800
                bg_badge: Color32::from_rgb(30, 41, 59),  // Slate 800
                border: Color32::from_rgb(51, 65, 85),    // Slate 700
                border_strong: Color32::from_rgb(71, 85, 105), // Slate 600
                text_primary: Color32::from_rgb(241, 245, 249), // Slate 100
                text_secondary: Color32::from_rgb(203, 213, 225), // Slate 300
                text_muted: Color32::from_rgb(148, 163, 184), // Slate 400
                success: Color32::from_rgb(74, 222, 128), // Green 400
                warning: Color32::from_rgb(251, 191, 36), // Amber 400
                error: Color32::from_rgb(248, 113, 113),  // Red 400
                accent: Color32::from_rgb(59, 130, 246),  // Blue 500
                dest_bg: Color32::from_rgb(20, 50, 40),
                dest_border: Color32::from_rgb(16, 185, 129),
                dest_text: Color32::from_rgb(52, 211, 153),
                dest_subtext: Color32::from_rgb(167, 243, 208),
                alert_error_bg: Color32::from_rgb(60, 20, 20),
                btn_disabled_bg: Color32::from_rgb(45, 55, 72),
                btn_secondary_bg: Color32::from_rgb(51, 65, 85),
                btn_secondary_text: Color32::from_rgb(241, 245, 249),
                graph_bg: Color32::from_rgb(15, 23, 42),
                graph_grid: Color32::from_rgba_unmultiplied(51, 65, 85, 90),
                graph_line: Color32::from_rgb(52, 211, 153),
                graph_fill_top: Color32::from_rgba_unmultiplied(16, 185, 129, 60),
                graph_fill_bottom: Color32::from_rgba_unmultiplied(16, 185, 129, 5),
                graph_avg_line: Color32::from_rgba_unmultiplied(56, 189, 248, 160),
            }
        } else {
            Self {
                is_dark: false,
                bg_window: Color32::from_rgb(248, 250, 252), // Slate 50
                bg_header: Color32::from_rgb(241, 245, 249), // Slate 100
                bg_card: Color32::from_rgb(255, 255, 255),   // Pure White
                bg_card_inner: Color32::from_rgb(241, 245, 249), // Slate 100
                bg_badge: Color32::from_rgb(226, 232, 240),  // Slate 200
                border: Color32::from_rgb(203, 213, 225),    // Slate 300
                border_strong: Color32::from_rgb(148, 163, 184), // Slate 400
                text_primary: Color32::from_rgb(15, 23, 42), // Slate 900
                text_secondary: Color32::from_rgb(51, 65, 85), // Slate 700
                text_muted: Color32::from_rgb(100, 116, 139), // Slate 500
                success: Color32::from_rgb(21, 128, 61),     // Green 700
                warning: Color32::from_rgb(180, 83, 9),      // Amber 700
                error: Color32::from_rgb(220, 38, 38),       // Red 600
                accent: Color32::from_rgb(29, 78, 216),      // Blue 700
                dest_bg: Color32::from_rgb(236, 253, 245),   // Emerald 50
                dest_border: Color32::from_rgb(16, 185, 129), // Emerald 500
                dest_text: Color32::from_rgb(4, 120, 87),    // Emerald 700
                dest_subtext: Color32::from_rgb(6, 95, 70),  // Emerald 800
                alert_error_bg: Color32::from_rgb(254, 242, 242), // Red 50
                btn_disabled_bg: Color32::from_rgb(226, 232, 240),
                btn_secondary_bg: Color32::from_rgb(226, 232, 240),
                btn_secondary_text: Color32::from_rgb(15, 23, 42),
                graph_bg: Color32::from_rgb(255, 255, 255),
                graph_grid: Color32::from_rgba_unmultiplied(203, 213, 225, 160),
                graph_line: Color32::from_rgb(5, 150, 105),
                graph_fill_top: Color32::from_rgba_unmultiplied(16, 185, 129, 45),
                graph_fill_bottom: Color32::from_rgba_unmultiplied(16, 185, 129, 5),
                graph_avg_line: Color32::from_rgba_unmultiplied(29, 78, 216, 150),
            }
        }
    }

    #[must_use]
    pub fn from_ui(ui: &egui::Ui) -> Self {
        Self::from_dark_mode(ui.visuals().dark_mode)
    }

    #[must_use]
    pub fn from_ctx(ctx: &egui::Context) -> Self {
        Self::from_dark_mode(ctx.style().visuals.dark_mode)
    }
}

fn build_dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(241, 245, 249));
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(15, 23, 42);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 41, 59);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(51, 65, 85);
    visuals.widgets.active.bg_fill = Color32::from_rgb(71, 85, 105);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85));
    visuals.window_fill = Color32::from_rgb(15, 23, 42);
    visuals.panel_fill = Color32::from_rgb(15, 23, 42);
    visuals.faint_bg_color = Color32::from_rgb(20, 29, 46);
    visuals.extreme_bg_color = Color32::from_rgb(10, 16, 30);
    visuals
}

fn build_light_visuals() -> Visuals {
    let mut visuals = Visuals::light();
    visuals.override_text_color = Some(Color32::from_rgb(15, 23, 42));
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(248, 250, 252);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(226, 232, 240);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(203, 213, 225);
    visuals.widgets.active.bg_fill = Color32::from_rgb(148, 163, 184);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(203, 213, 225));
    visuals.window_fill = Color32::from_rgb(248, 250, 252);
    visuals.panel_fill = Color32::from_rgb(248, 250, 252);
    visuals.faint_bg_color = Color32::from_rgb(241, 245, 249);
    visuals.extreme_bg_color = Color32::from_rgb(255, 255, 255);
    visuals
}

/// Applies the requested `ThemeMode` (System, Light, or Dark) to the egui context.
pub fn apply_theme_mode(ctx: &egui::Context, mode: ThemeMode) {
    ctx.set_theme(mode.to_preference());
}

/// Configures embedded fonts, typography, and BOTH Dark & Light visuals so System theme switching works seamlessly.
pub fn configure_theme(ctx: &egui::Context, initial_mode: ThemeMode) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Incorpora a fonte Caskaydia Cove Nerd Font estaticamente no executável
    const NERD_FONT_DATA: &[u8] = include_bytes!("../../assets/CaskaydiaCoveNerdFont-Regular.ttf");
    fonts.font_data.insert(
        "caskaydia_nerd".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(NERD_FONT_DATA)),
    );

    // 2. Tenta carregar Segoe UI do Windows para texto nativo, mantendo Caskaydia como fallback para todos os glifos
    #[cfg(windows)]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let font_path = format!("{windir}\\Fonts\\segoeui.ttf");
        if let Ok(font_data) = std::fs::read(&font_path) {
            fonts.font_data.insert(
                "segoeui".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(font_data)),
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

    // Configure BOTH Dark and Light styles so switching between System/Light/Dark is 100% consistent!
    let configure_style = |visuals: Visuals| {
        let mut style = egui::Style {
            visuals,
            ..Default::default()
        };
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
        style.spacing.button_padding = Vec2::new(14.0, 8.0);
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.scroll = egui::style::ScrollStyle::solid();
        style
    };

    ctx.set_style_of(Theme::Dark, configure_style(build_dark_visuals()));
    ctx.set_style_of(Theme::Light, configure_style(build_light_visuals()));

    // Apply saved or System theme preference
    apply_theme_mode(ctx, initial_mode);
}
