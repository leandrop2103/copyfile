//! Transfer statistics, performance metrics, and benchmark view.

use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_SUCCESS, COLOR_WARNING};
use crate::gui::views::header::{format_bytes, format_duration};
use eframe::egui::{Color32, Grid, RichText, Ui};
use std::time::Duration;

/// Renders the real-time statistics and summary panel.
#[allow(clippy::too_many_arguments)]
pub fn render_stats(
    ui: &mut Ui,
    total_files: usize,
    copied_files: usize,
    skipped_files: usize,
    error_files: usize,
    total_bytes: u64,
    copied_bytes: u64,
    current_speed_bps: f64,
    average_speed_bps: f64,
    elapsed: Duration,
    eta: Option<Duration>,
) {
    ui.add_space(8.0);

    // --- Cards Grid ---
    Grid::new("stats_grid")
        .num_columns(2)
        .spacing([40.0, 16.0])
        .show(ui, |ui| {
            // Column 1: Performance Metrics
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.label(RichText::new("⚡ Desempenho e Velocidade").strong());
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Velocidade Atual:");
                        ui.label(
                            RichText::new(format!("{}/s", format_bytes(current_speed_bps as u64)))
                                .strong()
                                .color(COLOR_WARNING),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Velocidade Média:");
                        ui.label(
                            RichText::new(format!("{}/s", format_bytes(average_speed_bps as u64)))
                                .strong()
                                .color(COLOR_ACCENT),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tempo Decorrido:");
                        ui.label(RichText::new(format_duration(elapsed)).strong());
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tempo Restante (ETA):");
                        let eta_str = eta.map_or_else(|| "-".to_string(), format_duration);
                        ui.label(RichText::new(eta_str).strong().color(COLOR_WARNING));
                    });
                });
            });

            // Column 2: Volume & File Counts
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.label(RichText::new("📊 Contagem e Volume de Arquivos").strong());
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Total de Itens:");
                        ui.label(RichText::new(total_files.to_string()).strong());
                    });

                    ui.horizontal(|ui| {
                        ui.label("✔ Copiados com Sucesso:");
                        ui.label(
                            RichText::new(copied_files.to_string())
                                .strong()
                                .color(COLOR_SUCCESS),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("⚠ Ignorados / Pulados:");
                        ui.label(
                            RichText::new(skipped_files.to_string())
                                .strong()
                                .color(COLOR_WARNING),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("✖ Arquivos com Erro:");
                        ui.label(
                            RichText::new(error_files.to_string())
                                .strong()
                                .color(COLOR_ERROR),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Volume Transferido:");
                        let remaining_bytes = total_bytes.saturating_sub(copied_bytes);
                        ui.label(
                            RichText::new(format!(
                                "{} / {} (Restam {})",
                                format_bytes(copied_bytes),
                                format_bytes(total_bytes),
                                format_bytes(remaining_bytes)
                            ))
                            .strong()
                            .color(Color32::from_rgb(226, 232, 240)),
                        );
                    });
                });
            });
            ui.end_row();
        });
}
