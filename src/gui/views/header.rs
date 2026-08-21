//! Header view displaying path selectors, action buttons, and dual progress bars.

use crate::core::types::{CopyConfig, OverwritePolicy};
use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_SUCCESS, COLOR_WARNING};
use eframe::egui::{self, Color32, ProgressBar, RichText, Ui, Vec2};
use std::path::PathBuf;
use std::time::Duration;

/// Formats byte counts into human-readable strings (e.g. "1.45 GB", "350 KB").
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Formats duration into a concise time string (e.g. "2m 15s", "1h 05m").
#[must_use]
pub fn format_duration(dur: Duration) -> String {
    let total_secs = dur.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{hours}h {minutes:02}m {seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

/// Renders the top header section of CopyFile.
#[allow(clippy::too_many_arguments)]
pub fn render_header(
    ui: &mut Ui,
    sources: &mut Vec<PathBuf>,
    destination: &mut Option<PathBuf>,
    config: &mut CopyConfig,
    is_running: bool,
    is_paused: bool,
    current_file_name: &Option<String>,
    current_file_copied: u64,
    current_file_total: u64,
    total_copied: u64,
    total_bytes: u64,
    current_speed_bps: f64,
    elapsed: Duration,
    eta: Option<Duration>,
    on_start: impl FnOnce(),
    on_toggle_pause: impl FnOnce(),
    on_cancel: impl FnOnce(),
    on_clear: impl FnOnce(),
) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(24, 33, 47))
        .inner_margin(14.0)
        .show(ui, |ui| {
            // --- Row 1: Source Selection Box ---
            ui.horizontal(|ui| {
                ui.label(RichText::new("📁 Origem:").strong().size(16.0));
                if sources.is_empty() {
                    ui.label(
                        RichText::new("Nenhum arquivo ou pasta selecionado (Arraste aqui ou use os botões ao lado)")
                            .italics()
                            .size(14.0)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                } else if sources.len() == 1 {
                    ui.label(
                        RichText::new(sources[0].to_string_lossy())
                            .strong()
                            .size(15.0)
                            .color(Color32::WHITE),
                    );
                } else {
                    ui.label(
                        RichText::new(format!("{} itens selecionados para cópia", sources.len()))
                            .strong()
                            .size(15.0)
                            .color(COLOR_ACCENT),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !is_running {
                        if !sources.is_empty() {
                            let clear_btn = egui::Button::new(RichText::new("🗑 Limpar").size(14.0))
                                .min_size(Vec2::new(80.0, 32.0));
                            if ui.add(clear_btn).clicked() {
                                on_clear();
                            }
                        }

                        let add_files_btn = egui::Button::new(
                            RichText::new("📄 + Adicionar Arquivos")
                                .strong()
                                .size(14.0),
                        )
                        .min_size(Vec2::new(170.0, 32.0));
                        if ui.add(add_files_btn).clicked() {
                            if let Some(files) = rfd::FileDialog::new().pick_files() {
                                for f in files {
                                    if !sources.contains(&f) {
                                        sources.push(f);
                                    }
                                }
                            }
                        }

                        let add_folders_btn = egui::Button::new(
                            RichText::new("📁 + Adicionar Pastas")
                                .strong()
                                .size(14.0),
                        )
                        .fill(Color32::from_rgb(37, 99, 235))
                        .min_size(Vec2::new(160.0, 32.0));
                        if ui.add(add_folders_btn).clicked() {
                            if let Some(folders) = rfd::FileDialog::new().pick_folders() {
                                for folder in folders {
                                    if !sources.contains(&folder) {
                                        sources.push(folder);
                                    }
                                }
                            }
                        }
                    }
                });
            });

            ui.add_space(6.0);

            // --- Row 2: Destination Selection Box ---
            ui.horizontal(|ui| {
                ui.label(RichText::new("🎯 Destino:").strong().size(16.0));
                if let Some(dst) = destination {
                    ui.label(
                        RichText::new(dst.to_string_lossy())
                            .strong()
                            .size(15.0)
                            .color(Color32::WHITE),
                    );
                } else {
                    ui.label(
                        RichText::new("Selecione a pasta de destino para salvar o backup...")
                            .italics()
                            .size(14.0)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !is_running {
                        let pick_dst_btn = egui::Button::new(
                            RichText::new("📂 Escolher Destino")
                                .strong()
                                .size(14.0)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(16, 185, 129))
                        .min_size(Vec2::new(160.0, 32.0));
                        if ui.add(pick_dst_btn).clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                *destination = Some(folder);
                            }
                        }
                    }
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // --- Row 3: Action Controls & Rules ---
            ui.horizontal(|ui| {
                if !is_running {
                    let can_start = !sources.is_empty() && destination.is_some();
                    let start_btn = egui::Button::new(
                        RichText::new("▶ INICIAR CÓPIA")
                            .strong()
                            .size(16.0)
                            .color(Color32::WHITE),
                    )
                    .fill(if can_start {
                        COLOR_SUCCESS
                    } else {
                        Color32::from_rgb(51, 65, 85)
                    })
                    .min_size(Vec2::new(170.0, 36.0));

                    if ui.add_enabled(can_start, start_btn).clicked() {
                        on_start();
                    }
                } else {
                    let pause_label = if is_paused {
                        "▶ CONTINUAR"
                    } else {
                        "⏸ PAUSAR"
                    };
                    let pause_btn = egui::Button::new(
                        RichText::new(pause_label).strong().size(15.0),
                    )
                    .min_size(Vec2::new(130.0, 36.0));
                    if ui.add(pause_btn).clicked() {
                        on_toggle_pause();
                    }

                    let cancel_btn = egui::Button::new(
                        RichText::new("⏹ CANCELAR")
                            .strong()
                            .size(15.0)
                            .color(COLOR_ERROR),
                    )
                    .min_size(Vec2::new(130.0, 36.0));
                    if ui.add(cancel_btn).clicked() {
                        on_cancel();
                    }
                }

                ui.add_space(16.0);

                // Quick options with clear checkboxes
                ui.checkbox(&mut config.verify_checksum, RichText::new("Verificar xxHash3").size(14.0));
                ui.checkbox(&mut config.ignore_errors, RichText::new("Ignorar erros").size(14.0));
                ui.checkbox(&mut config.preserve_metadata, RichText::new("Preservar metadados").size(14.0));

                // Quick Overwrite Policy dropdown
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("header_overwrite_combo")
                        .selected_text(RichText::new(config.overwrite_policy.label()).size(13.0))
                        .show_ui(ui, |ui| {
                            for policy in OverwritePolicy::all() {
                                ui.selectable_value(
                                    &mut config.overwrite_policy,
                                    policy,
                                    RichText::new(policy.label()).size(13.0),
                                );
                            }
                        });
                    ui.label(RichText::new("Regra:").size(14.0).strong());
                });
            });

            ui.add_space(8.0);

            // --- Row 4: Dual Progress Bars ---
            // Progress Bar 1: Current Active File
            let file_progress = if current_file_total > 0 {
                (current_file_copied as f32 / current_file_total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let current_file_str = current_file_name
                .as_deref()
                .unwrap_or("Aguardando início...");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Arquivo:").size(14.0));
                ui.label(
                    RichText::new(current_file_str)
                        .size(14.0)
                        .strong()
                        .color(Color32::from_rgb(226, 232, 240)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} / {} ({:.1}%)",
                            format_bytes(current_file_copied),
                            format_bytes(current_file_total),
                            file_progress * 100.0
                        ))
                        .size(14.0)
                        .monospace(),
                    );
                });
            });
            ui.add(
                ProgressBar::new(file_progress)
                    .animate(is_running && !is_paused)
                    .fill(COLOR_ACCENT),
            );

            ui.add_space(4.0);

            // Progress Bar 2: Total Transfer
            let total_progress = if total_bytes > 0 {
                (total_copied as f32 / total_bytes as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let speed_str = format!("{}/s", format_bytes(current_speed_bps as u64));
            let eta_str = eta.map_or_else(|| "calculando...".to_string(), format_duration);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Total:").size(14.0).strong());
                ui.label(
                    RichText::new(format!(
                        "{} de {} ({:.1}%)",
                        format_bytes(total_copied),
                        format_bytes(total_bytes),
                        total_progress * 100.0
                    ))
                    .size(14.0)
                    .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_running {
                        ui.label(
                            RichText::new(format!("⏳ Restante: {eta_str}  |  ⚡ {speed_str}"))
                                .size(14.0)
                                .color(COLOR_WARNING)
                                .strong(),
                        );
                    } else if total_copied > 0 && total_copied == total_bytes {
                        ui.label(
                            RichText::new(format!("✔ Concluído em {}", format_duration(elapsed)))
                                .size(14.0)
                                .color(COLOR_SUCCESS)
                                .strong(),
                        );
                    }
                });
            });
            ui.add(
                ProgressBar::new(total_progress)
                    .animate(is_running && !is_paused)
                    .fill(COLOR_SUCCESS),
            );
        });
}
