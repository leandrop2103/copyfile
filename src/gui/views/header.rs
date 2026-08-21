//! Header view displaying path selectors, user presets, disk space, action buttons, and dual progress bars.

use crate::core::disk::get_disk_space;
use crate::core::profiles::{
    get_current_user_presets, get_presets_for_user_path, list_all_windows_users,
};
use crate::core::types::{CopyConfig, OverwritePolicy};
use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS, COLOR_WARNING};
use eframe::egui::{self, Color32, CornerRadius, ProgressBar, RichText, Stroke, Ui, Vec2};
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
    processed_bytes: u64,
    total_bytes: u64,
    copied_files: usize,
    skipped_files: usize,
    error_files: usize,
    total_files: usize,
    current_speed_bps: f64,
    elapsed: Duration,
    eta: Option<Duration>,
    on_start: impl FnOnce(),
    on_toggle_pause: impl FnOnce(),
    on_cancel: impl FnOnce(),
    on_clear: impl FnOnce(),
    on_open_sources_manager: impl FnOnce(),
) {
    let mut open_sources_manager_requested = false;

    egui::Frame::NONE
        .fill(Color32::from_rgb(24, 33, 47))
        .inner_margin(14.0)
        .show(ui, |ui| {
            // --- Row 1: Source Header & Action Buttons ---
            ui.horizontal(|ui| {
                ui.label(RichText::new("📁 Origens Selecionadas:").strong().size(16.0));
                if !sources.is_empty() {
                    let manage_btn = ui.add(
                        egui::Button::new(
                            RichText::new(format!("📋 Gerenciar ({}) ▾", sources.len()))
                                .size(13.5)
                                .color(COLOR_ACCENT)
                                .strong(),
                        )
                        .frame(false),
                    );
                    if manage_btn.on_hover_text("Clique para abrir o Gerenciador de Pastas e Arquivos de Origem").clicked() {
                        open_sources_manager_requested = true;
                    }
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
                            RichText::new("📄 + Arquivos")
                                .strong()
                                .size(14.0),
                        )
                        .min_size(Vec2::new(120.0, 32.0));
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
                            RichText::new("📁 + Pastas")
                                .strong()
                                .size(14.0),
                        )
                        .fill(Color32::from_rgb(37, 99, 235))
                        .min_size(Vec2::new(110.0, 32.0));
                        if ui.add(add_folders_btn).clicked() {
                            if let Some(folders) = rfd::FileDialog::new().pick_folders() {
                                for folder in folders {
                                    if !sources.contains(&folder) {
                                        sources.push(folder);
                                    }
                                }
                            }
                        }

                        // One-Click User Profile Quick Preset Button
                        let quick_btn = egui::Button::new(
                            RichText::new("⚡ Backup do Usuário")
                                .strong()
                                .size(14.0)
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(124, 58, 237)) // Purple preset
                        .min_size(Vec2::new(170.0, 32.0));
                        if ui.add(quick_btn).on_hover_text("Adiciona automaticamente Área de Trabalho, Documentos, Downloads, Imagens, Vídeos e Chrome do usuário atual").clicked() {
                            for preset in get_current_user_presets() {
                                if preset.exists && !sources.contains(&preset.path) {
                                    sources.push(preset.path);
                                }
                            }
                        }

                        // Check if other Windows users exist on disk
                        let other_users = list_all_windows_users();
                        if other_users.len() > 1 {
                            ui.menu_button(RichText::new("👤 Outro Usuário ▼").size(14.0), |ui| {
                                for (name, user_path) in &other_users {
                                    if ui.button(format!("👤 Carregar {}", name)).clicked() {
                                        for preset in get_presets_for_user_path(user_path) {
                                            if !sources.contains(&preset.path) {
                                                sources.push(preset.path);
                                            }
                                        }
                                        ui.close_menu();
                                    }
                                }
                            });
                        }
                    }
                });
            });

            // --- Row 1.5: List of Selected Source Badges ---
            if sources.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Nenhuma pasta adicionada. Clique em '⚡ Backup do Usuário', '+ Pastas' ou arraste itens para cá.")
                            .italics()
                            .size(13.5)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                });
            } else {
                ui.add_space(2.0);
                let mut remove_idx = None;

                ui.horizontal(|ui| {
                    // Show at most the first 3 items as badges
                    for (idx, src) in sources.iter().enumerate().take(3) {
                        let is_dir = src.is_dir();
                        let icon = if is_dir { "📁" } else { "📄" };
                        let full_path = src.to_string_lossy().to_string();
                        let short_name = src
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&full_path);

                        egui::Frame::NONE
                            .fill(Color32::from_rgb(30, 41, 59))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(71, 85, 105)))
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(egui::Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{icon} {short_name}"))
                                            .size(13.0)
                                            .strong()
                                            .color(Color32::from_rgb(226, 232, 240)),
                                    )
                                    .on_hover_text(&full_path);

                                    if !is_running {
                                        let remove_btn = ui.add(
                                            egui::Button::new(
                                                RichText::new("✖")
                                                    .color(COLOR_ERROR)
                                                    .size(11.0)
                                                    .strong(),
                                            )
                                            .frame(false),
                                        );
                                        if remove_btn.clicked() {
                                            remove_idx = Some(idx);
                                        }
                                    }
                                });
                            });
                    }

                    // If more than 3 items, show Dropdown Menu with the rest
                    if sources.len() > 3 {
                        let remaining_count = sources.len() - 3;
                        let menu_label = format!("📁 +{remaining_count} restantes ▼");

                        ui.menu_button(
                            RichText::new(menu_label)
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                            |ui| {
                                ui.set_min_width(320.0);
                                ui.label(
                                    RichText::new(format!("Itens adicionais ({remaining_count}):"))
                                        .size(12.5)
                                        .color(COLOR_MUTED),
                                );
                                ui.separator();

                                for (idx, src) in sources.iter().enumerate().skip(3) {
                                    let is_dir = src.is_dir();
                                    let icon = if is_dir { "📁" } else { "📄" };
                                    let path_str = src.to_string_lossy().to_string();

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("{icon} {path_str}"))
                                                .size(13.0)
                                                .color(Color32::from_rgb(226, 232, 240)),
                                        );

                                        if !is_running {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .button(
                                                            RichText::new("✖")
                                                                .color(COLOR_ERROR)
                                                                .size(11.0),
                                                        )
                                                        .clicked()
                                                    {
                                                        remove_idx = Some(idx);
                                                        ui.close_menu();
                                                    }
                                                },
                                            );
                                        }
                                    });
                                }

                                ui.add_space(6.0);
                                ui.separator();
                                if ui
                                    .button(
                                        RichText::new("📋 Abrir Gerenciador Completo...")
                                            .color(COLOR_ACCENT)
                                            .strong()
                                            .size(13.0),
                                    )
                                    .clicked()
                                {
                                    open_sources_manager_requested = true;
                                    ui.close_menu();
                                }
                            },
                        );
                    }
                });

                if let Some(idx) = remove_idx {
                    sources.remove(idx);
                }
            }

            ui.add_space(6.0);

            // --- Row 2: Destination Header, Space Checking & Picker ---
            let disk_space_info = destination.as_ref().and_then(|d| get_disk_space(d));

            ui.horizontal(|ui| {
                ui.label(RichText::new("🎯 Destino:").strong().size(16.0));

                if !is_running {
                    let pick_dst_btn = egui::Button::new(
                        RichText::new("📂 Escolher Destino")
                            .strong()
                            .size(14.0)
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(16, 185, 129))
                    .min_size(Vec2::new(150.0, 32.0));
                    if ui.add(pick_dst_btn).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            *destination = Some(folder);
                        }
                    }
                }

                if let Some(dst) = destination.as_ref() {
                    let display_dst = dst.to_string_lossy().to_string();
                    let space_str = disk_space_info.map_or_else(
                        String::new,
                        |info| {
                            format!(
                                "  (Livre: {} de {})",
                                format_bytes(info.available_bytes),
                                format_bytes(info.total_bytes)
                            )
                        },
                    );

                    egui::Frame::NONE
                        .fill(Color32::from_rgb(20, 50, 40))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(16, 185, 129)))
                        .corner_radius(CornerRadius::same(5))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("📂 {display_dst}{space_str}"))
                                        .strong()
                                        .size(13.5)
                                        .color(Color32::from_rgb(52, 211, 153)),
                                );
                                if !is_running
                                    && ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("✖")
                                                    .color(COLOR_ERROR)
                                                    .size(11.0)
                                                    .strong(),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                {
                                    *destination = None;
                                }
                            });
                        });
                } else {
                    ui.label(
                        RichText::new("Selecione a pasta de destino para o backup...")
                            .italics()
                            .size(14.0)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );
                }
            });

            // Pre-Flight Disk Space Warning Alert
            if let Some(info) = disk_space_info {
                if total_bytes > 0 && total_bytes > info.available_bytes {
                    ui.add_space(4.0);
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(60, 20, 20))
                        .stroke(Stroke::new(1.0_f32, COLOR_ERROR))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "⚠ ATENÇÃO: Espaço insuficiente na unidade de destino! Necessário: {} | Disponível: {}",
                                        format_bytes(total_bytes),
                                        format_bytes(info.available_bytes)
                                    ))
                                    .color(COLOR_ERROR)
                                    .strong()
                                    .size(13.5),
                                );
                            });
                        });
                }
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // --- Row 3: Action Controls & Rules ---
            ui.horizontal(|ui| {
                if !is_running {
                    let can_start = !sources.is_empty() && destination.is_some();
                    let start_label = if config.transfer_mode == crate::core::types::TransferMode::Move {
                        "▶ MOVER (RECORTAR)"
                    } else {
                        "▶ INICIAR CÓPIA"
                    };
                    let start_color = if config.transfer_mode == crate::core::types::TransferMode::Move {
                        Color32::from_rgb(217, 119, 6) // Amber/Orange for move
                    } else {
                        COLOR_SUCCESS
                    };
                    let start_btn = egui::Button::new(
                        RichText::new(start_label)
                            .strong()
                            .size(16.0)
                            .color(Color32::WHITE),
                    )
                    .fill(if can_start {
                        start_color
                    } else {
                        Color32::from_rgb(51, 65, 85)
                    })
                    .min_size(Vec2::new(180.0, 36.0));

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

                ui.add_space(12.0);

                // Transfer Mode selector
                if !is_running {
                    egui::ComboBox::from_id_salt("header_mode_combo")
                        .selected_text(RichText::new(config.transfer_mode.label()).size(13.5).strong())
                        .show_ui(ui, |ui| {
                            for mode in crate::core::types::TransferMode::all() {
                                ui.selectable_value(
                                    &mut config.transfer_mode,
                                    mode,
                                    RichText::new(mode.label()).size(13.5),
                                );
                            }
                        });
                }

                ui.add_space(8.0);

                // Quick options
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
            let (file_progress, file_label_str, file_pct_str) = if is_running {
                let p = if current_file_total > 0 {
                    (current_file_copied as f32 / current_file_total as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let name = current_file_name.as_deref().unwrap_or("Transferindo...");
                let pct = format!(
                    "{} / {} ({:.1}%)",
                    format_bytes(current_file_copied),
                    format_bytes(current_file_total),
                    p * 100.0
                );
                (p, name, pct)
            } else if total_files > 0 && (copied_files + skipped_files + error_files == total_files) {
                (1.0, "Transferência Concluída!", "100.0%".to_string())
            } else {
                (0.0, "Aguardando início...", "-".to_string())
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new("Arquivo:").size(14.0));
                ui.label(
                    RichText::new(file_label_str)
                        .size(14.0)
                        .strong()
                        .color(Color32::from_rgb(226, 232, 240)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(file_pct_str)
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
            let processed_files_count = copied_files + skipped_files + error_files;
            let is_finished = total_files > 0 && processed_files_count == total_files && !is_running;

            let total_progress = if is_finished {
                1.0
            } else if total_bytes > 0 {
                (processed_bytes as f32 / total_bytes as f32).clamp(0.0, 1.0)
            } else if total_files > 0 {
                (processed_files_count as f32 / total_files as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let speed_str = format!("{}/s", format_bytes(current_speed_bps as u64));
            let eta_str = eta.map_or_else(|| "calculando...".to_string(), format_duration);
            let total_display_bytes = if is_finished { total_bytes } else { processed_bytes };

            ui.horizontal(|ui| {
                ui.label(RichText::new("Total:").size(14.0).strong());
                ui.label(
                    RichText::new(format!(
                        "{} de {} ({:.1}%)  •  {}/{} itens",
                        format_bytes(total_display_bytes),
                        format_bytes(total_bytes),
                        total_progress * 100.0,
                        processed_files_count,
                        total_files,
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
                    } else if is_finished {
                        ui.label(
                            RichText::new(format!(
                                "✔ Concluído em {} ({} copiados, {} ignorados, {} erros)",
                                format_duration(elapsed),
                                copied_files,
                                skipped_files,
                                error_files
                            ))
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

    if open_sources_manager_requested {
        on_open_sources_manager();
    }
}
