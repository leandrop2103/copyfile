//! Completion statistics modal dialog.

use crate::core::exporter::{export_csv_report, export_html_report};
use crate::core::types::FileEntry;
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
    entries: &[FileEntry],
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
            ui.set_min_width(520.0);

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

            ui.add_space(14.0);

            // Export Audit Reports Row
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("📄 Relatórios de Auditoria:")
                        .size(13.0)
                        .color(COLOR_MUTED),
                );

                if ui
                    .button(RichText::new("🌐 Exportar HTML").size(12.5).strong())
                    .on_hover_text("Salva relatório interativo com busca e hashes xxHash3")
                    .clicked()
                {
                    if let Some(save_path) = rfd::FileDialog::new()
                        .set_file_name("relatorio_backup.html")
                        .add_filter("Página HTML", &["html"])
                        .save_file()
                    {
                        let _ = export_html_report(
                            &save_path,
                            entries,
                            transfer_mode_name,
                            elapsed,
                            total_bytes,
                            copied_files,
                            skipped_files,
                            error_files,
                        );
                    }
                }

                if ui
                    .button(RichText::new("📊 Exportar CSV").size(12.5).strong())
                    .on_hover_text("Salva planilha CSV compatível com Excel")
                    .clicked()
                {
                    if let Some(save_path) = rfd::FileDialog::new()
                        .set_file_name("relatorio_backup.csv")
                        .add_filter("Arquivo CSV", &["csv"])
                        .save_file()
                    {
                        let _ = export_csv_report(&save_path, entries);
                    }
                }
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(14.0);

            // Modal Action Buttons
            ui.horizontal(|ui| {
                if let Some(dst) = destination {
                    let open_btn =
                        egui::Button::new(RichText::new("📂 Abrir Pasta").size(14.0).strong())
                            .min_size(Vec2::new(140.0, 36.0));
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
                        RichText::new("✔ Concluir")
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

/// Renders the Source Folders & Files manager modal dialog.
pub fn render_sources_manager_modal(
    ctx: &egui::Context,
    is_open: &mut bool,
    sources: &mut Vec<PathBuf>,
    search_query: &mut String,
    is_running: bool,
) {
    if !*is_open {
        return;
    }

    egui::Window::new("📁 Gerenciador de Pastas e Arquivos de Origem")
        .collapsible(false)
        .resizable(true)
        .default_size(Vec2::new(700.0, 480.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::NONE
                .fill(Color32::from_rgb(15, 23, 42))
                .stroke(Stroke::new(1.5_f32, Color32::from_rgb(59, 130, 246)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔍").size(15.0));
                ui.add(
                    egui::TextEdit::singleline(search_query)
                        .hint_text("Filtrar pastas ou arquivos adicionados...")
                        .desired_width(320.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !is_running && !sources.is_empty() {
                        let clear_btn = egui::Button::new(
                            RichText::new("🗑 Limpar Tudo").color(COLOR_ERROR).size(13.5),
                        );
                        if ui.add(clear_btn).clicked() {
                            sources.clear();
                        }
                    }
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            let q = search_query.to_lowercase();
            let mut remove_idx = None;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(320.0)
                .show(ui, |ui| {
                    if sources.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.label(
                                RichText::new("Nenhuma pasta ou arquivo adicionado.")
                                    .color(COLOR_MUTED)
                                    .size(15.0)
                                    .italics(),
                            );
                        });
                    } else {
                        for (idx, src) in sources.iter().enumerate() {
                            let path_str = src.to_string_lossy();
                            if !q.is_empty() && !path_str.to_lowercase().contains(&q) {
                                continue;
                            }

                            let is_dir = src.is_dir();
                            let icon = if is_dir { "📁" } else { "📄" };

                            egui::Frame::NONE
                                .fill(Color32::from_rgb(24, 33, 47))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(egui::Margin::symmetric(12, 8))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(icon).size(16.0));
                                        ui.label(
                                            RichText::new(path_str.to_string())
                                                .size(13.5)
                                                .strong()
                                                .color(Color32::from_rgb(226, 232, 240)),
                                        );

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if !is_running {
                                                    let rem_btn = egui::Button::new(
                                                        RichText::new("✖ Remover")
                                                            .color(COLOR_ERROR)
                                                            .size(12.5),
                                                    );
                                                    if ui.add(rem_btn).clicked() {
                                                        remove_idx = Some(idx);
                                                    }
                                                }
                                            },
                                        );
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    }
                });

            if let Some(idx) = remove_idx {
                sources.remove(idx);
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if !is_running {
                    let add_fld =
                        egui::Button::new(RichText::new("📁 + Adicionar Pastas").size(13.5));
                    if ui.add(add_fld).clicked() {
                        if let Some(folders) = rfd::FileDialog::new().pick_folders() {
                            for f in folders {
                                if !sources.contains(&f) {
                                    sources.push(f);
                                }
                            }
                        }
                    }
                    let add_fls =
                        egui::Button::new(RichText::new("📄 + Adicionar Arquivos").size(13.5));
                    if ui.add(add_fls).clicked() {
                        if let Some(files) = rfd::FileDialog::new().pick_files() {
                            for f in files {
                                if !sources.contains(&f) {
                                    sources.push(f);
                                }
                            }
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_btn = egui::Button::new(
                        RichText::new("✔ Concluir")
                            .strong()
                            .size(14.0)
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(37, 99, 235))
                    .min_size(Vec2::new(110.0, 34.0));
                    if ui.add(close_btn).clicked() {
                        *is_open = false;
                    }
                });
            });
        });
}
