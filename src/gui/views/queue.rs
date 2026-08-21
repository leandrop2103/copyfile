//! Transfer Queue management view.

use crate::core::queue::{QueueStatus, TransferQueue};
use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS, COLOR_WARNING};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Ui, Vec2};
use std::path::PathBuf;

/// Renders the Transfer Queue management tab.
pub fn render_queue_view(
    ui: &mut Ui,
    queue: &mut TransferQueue,
    is_running: bool,
    current_sources: &[PathBuf],
    current_destination: &Option<PathBuf>,
    on_enqueue_current: impl FnOnce(),
) {
    ui.add_space(8.0);

    // Queue Header & Actions
    ui.horizontal(|ui| {
        let count = queue.waiting_count();
        ui.label(
            RichText::new(format!("🗂 Fila de Transferências ({count} aguardando)"))
                .strong()
                .size(16.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let clear_btn = egui::Button::new(RichText::new("🗑 Limpar Concluídos").size(13.0));
            if ui.add(clear_btn).clicked() {
                queue.clear_finished();
            }

            if !is_running || (!current_sources.is_empty() && current_destination.is_some()) {
                let add_btn = egui::Button::new(
                    RichText::new("➕ Enfileirar Seleção Atual")
                        .strong()
                        .size(13.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(37, 99, 235))
                .min_size(Vec2::new(170.0, 30.0));

                if ui.add(add_btn).clicked() {
                    on_enqueue_current();
                }
            }
        });
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    if queue.jobs().is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(
                RichText::new("A fila de transferências está vazia.")
                    .color(COLOR_MUTED)
                    .size(16.0)
                    .italics(),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("Você pode selecionar pastas na barra superior e clicar em '➕ Enfileirar Seleção Atual'.")
                    .color(COLOR_MUTED)
                    .size(13.5),
            );
        });
        return;
    }

    let mut remove_id = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for job in queue.jobs() {
                let (status_color, status_icon) = match job.status {
                    QueueStatus::Waiting => (COLOR_WARNING, "⏳"),
                    QueueStatus::Running => (COLOR_ACCENT, "⚡"),
                    QueueStatus::Completed => (COLOR_SUCCESS, "✔"),
                    QueueStatus::Cancelled => (COLOR_ERROR, "✖"),
                };

                egui::Frame::NONE
                    .fill(Color32::from_rgb(24, 33, 47))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Status Badge
                            ui.label(
                                RichText::new(format!("{status_icon} {}", job.status.label()))
                                    .strong()
                                    .color(status_color)
                                    .size(13.5),
                            );

                            ui.separator();

                            // Mode & Items Info
                            let mode_name = job.config.transfer_mode.label();
                            ui.label(
                                RichText::new(format!(
                                    "Modo: {mode_name} | {} origens",
                                    job.sources.len()
                                ))
                                .size(13.0)
                                .color(Color32::from_rgb(203, 213, 225)),
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if job.status == QueueStatus::Waiting {
                                        let rem_btn = egui::Button::new(
                                            RichText::new("✖ Remover da Fila")
                                                .color(COLOR_ERROR)
                                                .size(12.0),
                                        );
                                        if ui.add(rem_btn).clicked() {
                                            remove_id = Some(job.id);
                                        }
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);

                        // Destination path
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("🎯 Destino: {}", job.destination.display()))
                                    .size(13.0)
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );
                        });

                        // Sources preview
                        ui.horizontal(|ui| {
                            let sources_str = job
                                .sources
                                .iter()
                                .map(|s| s.file_name().and_then(|n| n.to_str()).unwrap_or("?"))
                                .collect::<Vec<_>>()
                                .join(", ");
                            ui.label(
                                RichText::new(format!("📁 Pastas: {sources_str}"))
                                    .size(12.5)
                                    .color(Color32::from_rgb(100, 116, 139)),
                            );
                        });
                    });

                ui.add_space(6.0);
            }
        });

    if let Some(id) = remove_id {
        queue.remove_job(id);
    }
}
