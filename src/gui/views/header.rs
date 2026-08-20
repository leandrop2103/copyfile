//! Header view displaying path selectors, action buttons, and dual progress bars.

use crate::core::types::CopyConfig;
use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_SUCCESS, COLOR_WARNING};
use eframe::egui::{self, Color32, ProgressBar, RichText, Ui};
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
        .inner_margin(12.0)
        .show(ui, |ui| {
            // --- Row 1: Source & Destination Paths Selection ---
            ui.horizontal(|ui| {
                ui.label(RichText::new("📁 Origem:").strong());
                if sources.is_empty() {
                    ui.label(
                        RichText::new("Nenhum arquivo/pasta selecionado")
                            .italics()
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                } else if sources.len() == 1 {
                    ui.label(RichText::new(sources[0].to_string_lossy()).strong());
                } else {
                    ui.label(
                        RichText::new(format!("{} itens selecionados", sources.len()))
                            .strong()
                            .color(COLOR_ACCENT),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !is_running {
                        if ui.button("🗑 Limpar").clicked() {
                            on_clear();
                        }
                        if ui.button("+ Adicionar Arquivos").clicked() {
                            if let Some(files) = rfd::FileDialog::new().pick_files() {
                                for f in files {
                                    if !sources.contains(&f) {
                                        sources.push(f);
                                    }
                                }
                            }
                        }
                        if ui.button("+ Adicionar Pastas").clicked() {
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

            ui.add_space(4.0);

            // --- Row 2: Destination Selection ---
            ui.horizontal(|ui| {
                ui.label(RichText::new("🎯 Destino:").strong());
                if let Some(dst) = destination {
                    ui.label(RichText::new(dst.to_string_lossy()).strong());
                } else {
                    ui.label(
                        RichText::new("Selecione a pasta de destino...")
                            .italics()
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !is_running && ui.button("📂 Escolher Destino").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            *destination = Some(folder);
                        }
                    }
                });
            });

            ui.separator();

            // --- Row 3: Action Controls & Checkbox Options ---
            ui.horizontal(|ui| {
                if !is_running {
                    let can_start = !sources.is_empty() && destination.is_some();
                    let start_btn = egui::Button::new(
                        RichText::new("▶ Iniciar Cópia")
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(if can_start {
                        COLOR_SUCCESS
                    } else {
                        Color32::from_rgb(51, 65, 85)
                    });
                    if ui.add_enabled(can_start, start_btn).clicked() {
                        on_start();
                    }
                } else {
                    let pause_label = if is_paused {
                        "▶ Continuar"
                    } else {
                        "⏸ Pausar"
                    };
                    if ui.button(RichText::new(pause_label).strong()).clicked() {
                        on_toggle_pause();
                    }

                    if ui
                        .button(RichText::new("⏹ Cancelar").color(COLOR_ERROR))
                        .clicked()
                    {
                        on_cancel();
                    }
                }

                ui.add_space(16.0);

                // Quick options
                ui.checkbox(&mut config.verify_checksum, "Verificar xxHash");
                ui.checkbox(&mut config.ignore_errors, "Ignorar erros");
                ui.checkbox(&mut config.preserve_metadata, "Preservar metadados");
            });

            ui.add_space(8.0);

            // --- Row 4: Dual Progress Bars (Current File & Total) ---
            // Progress Bar 1: Current File
            let file_progress = if current_file_total > 0 {
                (current_file_copied as f32 / current_file_total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let current_file_str = current_file_name
                .as_deref()
                .unwrap_or("Aguardando início...");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Arquivo:").small());
                ui.label(RichText::new(current_file_str).small().strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} / {} ({:.1}%)",
                            format_bytes(current_file_copied),
                            format_bytes(current_file_total),
                            file_progress * 100.0
                        ))
                        .small(),
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
                ui.label(RichText::new("Total:").small());
                ui.label(
                    RichText::new(format!(
                        "{} de {} ({:.1}%)",
                        format_bytes(total_copied),
                        format_bytes(total_bytes),
                        total_progress * 100.0
                    ))
                    .small()
                    .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_running {
                        ui.label(
                            RichText::new(format!("⏳ Restante: {eta_str} | ⚡ {speed_str}"))
                                .small()
                                .color(COLOR_WARNING),
                        );
                    } else if total_copied > 0 && total_copied == total_bytes {
                        ui.label(
                            RichText::new(format!("✔ Concluído em {}", format_duration(elapsed)))
                                .small()
                                .color(COLOR_SUCCESS),
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
