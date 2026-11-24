//! Transfer Queue management view with job reordering, start-queue action, and zero-overflow cards.

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
    on_start_queue: impl FnOnce(),
) {
    ui.add_space(6.0);

    let waiting = queue.waiting_count();
    let can_enqueue = !current_sources.is_empty() && current_destination.is_some();

    // Queue Header & Actions
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("📋 Fila ({waiting} aguardando)"))
                .strong()
                .size(15.5),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let clear_btn = egui::Button::new(RichText::new("🗑 Limpar Concluídos").size(13.0));
            if ui.add(clear_btn).clicked() {
                queue.clear_finished();
            }

            if !is_running && waiting > 0 {
                let start_q_btn = egui::Button::new(
                    RichText::new("▶ Iniciar Fila")
                        .strong()
                        .size(13.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(16, 185, 129))
                .min_size(Vec2::new(115.0, 28.0));

                if ui.add(start_q_btn).clicked() {
                    on_start_queue();
                }
            }

            let add_btn = egui::Button::new(
                RichText::new("➕ Enfileirar Seleção Atual")
                    .strong()
                    .size(13.0)
                    .color(if can_enqueue {
                        Color32::WHITE
                    } else {
                        COLOR_MUTED
                    }),
            )
            .fill(if can_enqueue {
                Color32::from_rgb(37, 99, 235)
            } else {
                Color32::from_rgb(45, 55, 72)
            })
            .min_size(Vec2::new(165.0, 28.0));

            if ui
                .add_enabled(can_enqueue, add_btn)
                .on_disabled_hover_text("Selecione ao menos uma origem e um destino acima")
                .clicked()
            {
                on_enqueue_current();
            }
        });
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);

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
                RichText::new(
                    "Selecione pastas na barra superior e clique em '➕ Enfileirar Seleção Atual'.",
                )
                .color(COLOR_MUTED)
                .size(13.5),
            );
        });
        return;
    }

    let mut remove_id = None;
    let mut move_up_id = None;
    let mut move_down_id = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for job in queue.jobs() {
                let status_color = match job.status {
                    QueueStatus::Waiting => COLOR_WARNING,
                    QueueStatus::Running => COLOR_ACCENT,
                    QueueStatus::Completed => COLOR_SUCCESS,
                    QueueStatus::Cancelled => COLOR_ERROR,
                };

                egui::Frame::NONE
                    .fill(Color32::from_rgb(24, 33, 47))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("#{} {}", job.id, job.status.label()))
                                    .strong()
                                    .color(status_color)
                                    .size(13.5),
                            );

                            ui.separator();

                            let mode_name = job.config.transfer_mode.label();
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!(
                                        "{mode_name}  •  {} origem(ns)",
                                        job.sources.len()
                                    ))
                                    .size(13.0)
                                    .color(Color32::from_rgb(203, 213, 225)),
                                )
                                .truncate(),
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if job.status == QueueStatus::Waiting {
                                        let rem_btn = egui::Button::new(
                                            RichText::new("✖ Remover")
                                                .color(COLOR_ERROR)
                                                .size(12.0),
                                        );
                                        if ui.add(rem_btn).clicked() {
                                            remove_id = Some(job.id);
                                        }

                                        if ui
                                            .button(RichText::new("⬇").size(12.0))
                                            .on_hover_text("Mover para baixo na fila")
                                            .clicked()
                                        {
                                            move_down_id = Some(job.id);
                                        }
                                        if ui
                                            .button(RichText::new("⬆").size(12.0))
                                            .on_hover_text("Mover para cima na fila")
                                            .clicked()
                                        {
                                            move_up_id = Some(job.id);
                                        }
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);

                        // Destination path (truncated cleanly)
                        let dst_str = job.destination.display().to_string();
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("🎯 Destino: {dst_str}"))
                                    .size(13.0)
                                    .color(Color32::from_rgb(148, 163, 184)),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&dst_str);

                        // Sources preview (truncated cleanly)
                        let sources_str = job
                            .sources
                            .iter()
                            .map(|s| s.file_name().and_then(|n| n.to_str()).unwrap_or("?"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("📁 Origens: {sources_str}"))
                                    .size(12.5)
                                    .color(Color32::from_rgb(100, 116, 139)),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&sources_str);
                    });

                ui.add_space(6.0);
            }
        });

    if let Some(id) = move_up_id {
        queue.move_up(id);
    }
    if let Some(id) = move_down_id {
        queue.move_down(id);
    }
    if let Some(id) = remove_id {
        queue.remove_job(id);
    }
}
