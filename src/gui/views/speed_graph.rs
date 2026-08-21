//! Real-time interactive Speed Graph view with smooth gradient fill and dynamic metrics.

use crate::gui::theme::{COLOR_ACCENT, COLOR_MUTED, COLOR_SUCCESS};
use crate::gui::views::header::format_bytes;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Shape, Stroke, Ui, Vec2};
use std::collections::VecDeque;

/// Historical speed sample history tracker.
#[derive(Debug, Clone)]
pub struct SpeedHistory {
    samples: VecDeque<f64>,
    max_samples: usize,
    peak_speed: f64,
}

impl Default for SpeedHistory {
    fn default() -> Self {
        Self::new(160) // 160 samples @ 250ms = 40 seconds of history
    }
}

impl SpeedHistory {
    #[must_use]
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
            peak_speed: 0.0,
        }
    }

    /// Adds a new speed sample (bytes per second).
    pub fn push(&mut self, speed_bps: f64) {
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(speed_bps);
        if speed_bps > self.peak_speed {
            self.peak_speed = speed_bps;
        }
    }

    /// Clears history.
    pub fn clear(&mut self) {
        self.samples.clear();
        self.peak_speed = 0.0;
    }

    #[must_use]
    pub fn peak(&self) -> f64 {
        self.peak_speed
    }
}

/// Renders the real-time speed graph canvas into the UI.
pub fn render_speed_graph(
    ui: &mut Ui,
    history: &SpeedHistory,
    current_speed: f64,
    avg_speed: f64,
    height: f32,
) {
    let available_width = ui.available_width().max(200.0);
    let desired_size = Vec2::new(available_width, height);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);

    // 1. Draw Background Frame
    painter.rect_filled(
        rect,
        CornerRadius::same(8),
        Color32::from_rgb(15, 23, 42), // Slate 900
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)),
        egui::StrokeKind::Inside,
    );

    let samples = &history.samples;
    if samples.len() < 2 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Aguardando transferência para traçar o gráfico de taxa...",
            egui::FontId::proportional(13.5),
            COLOR_MUTED,
        );
        return;
    }

    // Determine scale ceiling
    let max_val = samples
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(avg_speed)
        .max(1024.0 * 1024.0); // At least 1 MB/s scale ceiling

    let ceiling = max_val * 1.15; // 15% headroom

    let graph_margin_left = 65.0;
    let graph_margin_right = 16.0;
    let graph_margin_top = 24.0;
    let graph_margin_bottom = 24.0;

    let plot_rect = Rect::from_min_max(
        Pos2::new(
            rect.min.x + graph_margin_left,
            rect.min.y + graph_margin_top,
        ),
        Pos2::new(
            rect.max.x - graph_margin_right,
            rect.max.y - graph_margin_bottom,
        ),
    );

    // 2. Draw Horizontal Grid Lines & Scale Labels
    let grid_steps = 4;
    for i in 0..=grid_steps {
        let frac = i as f32 / grid_steps as f32;
        let y = plot_rect.max.y - frac * plot_rect.height();
        let speed_at_step = (frac as f64) * ceiling;

        // Grid line
        painter.line_segment(
            [Pos2::new(plot_rect.min.x, y), Pos2::new(plot_rect.max.x, y)],
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(51, 65, 85, 90)),
        );

        // Y-axis Label
        let label = format!("{}/s", format_bytes(speed_at_step as u64));
        painter.text(
            Pos2::new(plot_rect.min.x - 8.0, y),
            egui::Align2::RIGHT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(148, 163, 184),
        );
    }

    // 3. Map samples to Points
    let num_samples = samples.len();
    let dx = plot_rect.width() / ((num_samples - 1) as f32).max(1.0);

    let mut points: Vec<Pos2> = Vec::with_capacity(num_samples);
    for (i, &s) in samples.iter().enumerate() {
        let x = plot_rect.min.x + (i as f32) * dx;
        let frac = (s / ceiling).clamp(0.0, 1.0) as f32;
        let y = plot_rect.max.y - frac * plot_rect.height();
        points.push(Pos2::new(x, y));
    }

    // 4. Draw Gradient / Filled Area under curve
    if points.len() >= 2 {
        let mut fill_mesh = egui::Mesh::default();
        let fill_color_top = Color32::from_rgba_unmultiplied(16, 185, 129, 60); // Emerald translucent
        let fill_color_bottom = Color32::from_rgba_unmultiplied(16, 185, 129, 5);

        for i in 0..(points.len() - 1) {
            let p1 = points[i];
            let p2 = points[i + 1];
            let b1 = Pos2::new(p1.x, plot_rect.max.y);
            let b2 = Pos2::new(p2.x, plot_rect.max.y);

            let idx = fill_mesh.vertices.len() as u32;
            fill_mesh.vertices.push(egui::epaint::Vertex {
                pos: p1,
                uv: egui::epaint::WHITE_UV,
                color: fill_color_top,
            });
            fill_mesh.vertices.push(egui::epaint::Vertex {
                pos: p2,
                uv: egui::epaint::WHITE_UV,
                color: fill_color_top,
            });
            fill_mesh.vertices.push(egui::epaint::Vertex {
                pos: b2,
                uv: egui::epaint::WHITE_UV,
                color: fill_color_bottom,
            });
            fill_mesh.vertices.push(egui::epaint::Vertex {
                pos: b1,
                uv: egui::epaint::WHITE_UV,
                color: fill_color_bottom,
            });

            fill_mesh
                .indices
                .extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
        }
        painter.add(Shape::mesh(fill_mesh));

        // 5. Draw Smooth Stroke Curve
        for i in 0..(points.len() - 1) {
            painter.line_segment(
                [points[i], points[i + 1]],
                Stroke::new(2.2_f32, Color32::from_rgb(52, 211, 153)), // Bright Emerald
            );
        }
    }

    // 6. Draw Average Speed Indicator Line
    if avg_speed > 0.0 {
        let avg_y =
            plot_rect.max.y - ((avg_speed / ceiling).clamp(0.0, 1.0) as f32) * plot_rect.height();
        painter.line_segment(
            [
                Pos2::new(plot_rect.min.x, avg_y),
                Pos2::new(plot_rect.max.x, avg_y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(56, 189, 248, 160)), // Blue accent line
        );
        painter.text(
            Pos2::new(plot_rect.max.x - 4.0, avg_y - 8.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("Média: {}/s", format_bytes(avg_speed as u64)),
            egui::FontId::proportional(11.0),
            COLOR_ACCENT,
        );
    }

    // 7. Graph Header Overlay Badges
    let cur_str = format!("{}/s", format_bytes(current_speed as u64));
    let peak_str = format!("{}/s", format_bytes(history.peak() as u64));

    painter.text(
        Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("Taxa Atual: {cur_str}"),
        egui::FontId::proportional(13.5),
        COLOR_SUCCESS,
    );

    painter.text(
        Pos2::new(rect.max.x - 14.0, rect.min.y + 12.0),
        egui::Align2::RIGHT_TOP,
        format!("Pico Máximo: {peak_str}"),
        egui::FontId::proportional(12.5),
        Color32::from_rgb(148, 163, 184),
    );
}
