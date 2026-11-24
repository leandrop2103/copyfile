use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_SUCCESS, COLOR_WARNING};
use crate::gui::views::header::{format_bytes, format_duration};
use crate::gui::views::speed_graph::{render_speed_graph, SpeedHistory};
use eframe::egui::{self, Color32, CornerRadius, RichText, ScrollArea, Stroke, Ui};
use std::time::Duration;

/// Renders the real-time statistics and summary panel with interactive speed graph.
#[allow(clippy::too_many_arguments)]
pub fn render_stats(
    ui: &mut Ui,
    total_files: usize,
    copied_files: usize,
    skipped_files: usize,
    error_files: usize,
    total_bytes: u64,
    copied_bytes: u64,
    processed_bytes: u64,
    current_speed_bps: f64,
    average_speed_bps: f64,
    elapsed: Duration,
    eta: Option<Duration>,
    speed_history: &SpeedHistory,
) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(6.0);

            // --- Real-Time Speed Graph ---
            ui.label(
                RichText::new("📈 Histórico de Velocidade em Tempo Real (MB/s)")
                    .strong()
                    .size(15.0),
            );
            ui.add_space(4.0);
            render_speed_graph(
                ui,
                speed_history,
                current_speed_bps,
                average_speed_bps,
                165.0,
            );
            ui.add_space(12.0);

            // --- Responsive 2-Column Metric Cards ---
            ui.columns(2, |columns| {
                // Column 1: Performance Metrics
                columns[0].style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(24, 33, 47))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(14))
                    .show(&mut columns[0], |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new("⚡ Desempenho e Velocidade")
                                .strong()
                                .size(15.0),
                        );
                        ui.separator();
                        ui.add_space(4.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Velocidade Atual:");
                            ui.label(
                                RichText::new(format!(
                                    "{}/s",
                                    format_bytes(current_speed_bps as u64)
                                ))
                                .strong()
                                .color(COLOR_WARNING),
                            );
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Velocidade Média:");
                            ui.label(
                                RichText::new(format!(
                                    "{}/s",
                                    format_bytes(average_speed_bps as u64)
                                ))
                                .strong()
                                .color(COLOR_ACCENT),
                            );
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Pico Máximo:");
                            ui.label(
                                RichText::new(format!(
                                    "{}/s",
                                    format_bytes(speed_history.peak() as u64)
                                ))
                                .strong()
                                .color(COLOR_SUCCESS),
                            );
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Tempo Decorrido:");
                            ui.label(RichText::new(format_duration(elapsed)).strong());
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Tempo Restante (ETA):");
                            let eta_str = eta.map_or_else(|| "-".to_string(), format_duration);
                            ui.label(RichText::new(eta_str).strong().color(COLOR_WARNING));
                        });
                    });

                // Column 2: Volume & File Counts
                columns[1].style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(24, 33, 47))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(14))
                    .show(&mut columns[1], |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new("📊 Contagem e Volume de Arquivos")
                                .strong()
                                .size(15.0),
                        );
                        ui.separator();
                        ui.add_space(4.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Total de Itens:");
                            ui.label(RichText::new(total_files.to_string()).strong());
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("✔ Copiados com Sucesso:");
                            ui.label(
                                RichText::new(copied_files.to_string())
                                    .strong()
                                    .color(COLOR_SUCCESS),
                            );
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("⚠ Ignorados / Pulados:");
                            ui.label(
                                RichText::new(skipped_files.to_string())
                                    .strong()
                                    .color(COLOR_WARNING),
                            );
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("✖ Arquivos com Erro:");
                            ui.label(
                                RichText::new(error_files.to_string())
                                    .strong()
                                    .color(COLOR_ERROR),
                            );
                        });

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Volume Transferido:");
                            let remaining_bytes = total_bytes.saturating_sub(processed_bytes);
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

            ui.add_space(12.0);
        });
}
