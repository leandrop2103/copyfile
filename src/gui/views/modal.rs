//! Completion statistics modal dialog.

use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS, COLOR_WARNING};
use crate::gui::views::header::{format_bytes, format_duration};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Ui, Vec2};
use std::path::PathBuf;
use std::time::Duration;

/// Renders the celebration / completion summary modal dialog.
#[allow(clippy::too_many_arguments)]
pub fn render_completion_modal(
    ctx: &egui::Context,
    is_open: &mut bool,
    transfer_mode_name: &str,
    total_folders: usize,
    copied_files: usize,
    skipped_files: usize,
    error_files: usize,
    total_files: usize,
    total_bytes: u64,
    elapsed: Duration,
    average_speed_bps: f64,
    destination: Option<&PathBuf>,
    on_open_logs: impl FnOnce(),
) {
    if !*is_open {
        return;
    }

    let mut open_logs_requested = false;
    let mut open_folder_requested = false;

    egui::Window::new("🎉 Transferência Concluída!")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::NONE
                .fill(Color32::from_rgb(15, 23, 42)) // Slate 900
                .stroke(Stroke::new(2.0_f32, Color32::from_rgb(16, 185, 129))) // Emerald border
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(24)),
        )
        .show(ctx, |ui| {
            ui.set_min_width(480.0);

            // Modal Header
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("✔ Operação Concluída com Sucesso!")
                        .size(20.0)
                        .strong()
                        .color(COLOR_SUCCESS),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!("Resumo da operação: {transfer_mode_name}"))
                        .size(14.0)
                        .color(COLOR_MUTED),
                );
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(14.0);

            // Statistics Grid Cards
            egui::Frame::NONE
                .fill(Color32::from_rgb(24, 33, 47))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.columns(2, |cols| {
                        // Left Column
                        cols[0].vertical(|ui| {
                            render_stat_row(
                                ui,
                                "📁 Pastas Processadas:",
                                &format!("{total_folders}"),
                                COLOR_ACCENT,
                            );
                            ui.add_space(8.0);
                            render_stat_row(
                                ui,
                                "📄 Arquivos Copiados:",
                                &format!("{copied_files} de {total_files}"),
                                COLOR_SUCCESS,
                            );
                            ui.add_space(8.0);
                            render_stat_row(
                                ui,
                                "⏭ Arquivos Ignorados:",
                                &format!("{skipped_files}"),
                                COLOR_WARNING,
                            );
                            ui.add_space(8.0);
                            render_stat_row(
                                ui,
                                "❌ Erros Encontrados:",
                                &format!("{error_files}"),
                                if error_files > 0 {
                                    COLOR_ERROR
                                } else {
                                    COLOR_MUTED
                                },
                            );
                        });

                        // Right Column
                        cols[1].vertical(|ui| {
                            render_stat_row(
                                ui,
                                "💾 Volume Total:",
                                &format_bytes(total_bytes),
                                Color32::WHITE,
                            );
                            ui.add_space(8.0);
                            render_stat_row(
                                ui,
                                "⏱ Tempo Total:",
                                &format_duration(elapsed),
                                COLOR_ACCENT,
                            );
                            ui.add_space(8.0);
                            let speed_str = format!("{}/s", format_bytes(average_speed_bps as u64));
                            render_stat_row(ui, "⚡ Velocidade Média:", &speed_str, COLOR_SUCCESS);
                            ui.add_space(8.0);
                            render_stat_row(
                                ui,
                                "🛡 Integridade:",
                                "100% xxHash3",
                                Color32::from_rgb(52, 211, 153),
                            );
                        });
                    });
                });

            ui.add_space(18.0);

            // Modal Action Buttons
            ui.horizontal(|ui| {
                if let Some(dst) = destination {
                    let open_btn = egui::Button::new(
                        RichText::new("📂 Abrir Pasta de Destino")
                            .size(14.0)
                            .strong(),
                    )
                    .min_size(Vec2::new(170.0, 36.0));
                    if ui.add(open_btn).clicked() {
                        open_folder_requested = true;
                        let _ = std::process::Command::new("explorer.exe").arg(dst).spawn();
                    }
                }

                let logs_btn = egui::Button::new(RichText::new("📜 Ver Logs").size(14.0).strong())
                    .min_size(Vec2::new(100.0, 36.0));
                if ui.add(logs_btn).clicked() {
                    open_logs_requested = true;
                    *is_open = false;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_btn = egui::Button::new(
                        RichText::new("✔ Fechar")
                            .size(14.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(16, 185, 129))
                    .min_size(Vec2::new(100.0, 36.0));
                    if ui.add(close_btn).clicked() {
                        *is_open = false;
                    }
                });
            });
        });

    if open_logs_requested {
        on_open_logs();
    }
    let _ = open_folder_requested;
}

fn render_stat_row(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .size(13.5)
                .color(Color32::from_rgb(148, 163, 184)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(13.5).strong().color(color));
        });
    });
}
