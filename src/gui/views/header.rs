//! Header view displaying path selectors, user presets, disk space, action buttons, and dual progress bars.

use crate::core::disk::get_disk_space;
pub use crate::core::format::{format_bytes, format_duration};
use crate::core::profiles::{
    get_current_user_presets, get_presets_for_user_path, list_all_windows_users,
};
use crate::core::types::{CopyConfig, OverwritePolicy};
use crate::gui::theme::AppPalette;
use eframe::egui::{self, Color32, CornerRadius, ProgressBar, RichText, Stroke, Ui, Vec2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type UserListCache = Option<(Vec<(String, PathBuf)>, Instant)>;

/// Truncates a string to at most `max_chars` characters, appending `…` if truncated.
fn truncate_label(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else if max_chars <= 1 {
        "…".to_string()
    } else {
        let prefix: String = s.chars().take(max_chars - 1).collect();
        format!("{prefix}…")
    }
}

/// Cached `is_dir` check so we never call `fs::metadata` 60 times/sec on network or slow drives.
fn cached_is_dir(path: &Path) -> bool {
    thread_local! {
        static DIR_CACHE: std::cell::RefCell<(HashMap<PathBuf, bool>, Instant)> =
            std::cell::RefCell::new((HashMap::new(), Instant::now()));
    }
    DIR_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        if cache.1.elapsed().as_secs() >= 15 || cache.0.len() > 512 {
            cache.0.clear();
            cache.1 = Instant::now();
        }
        *cache
            .0
            .entry(path.to_path_buf())
            .or_insert_with(|| path.is_dir())
    })
}

/// Cached list of Windows user profiles (refreshed every 30s instead of every frame!).
fn cached_windows_users() -> Vec<(String, PathBuf)> {
    thread_local! {
        static USERS_CACHE: std::cell::RefCell<UserListCache> =
            const { std::cell::RefCell::new(None) };
    }
    USERS_CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        let needs_refresh = match cache.as_ref() {
            None => true,
            Some((_, ts)) => ts.elapsed().as_secs() >= 30,
        };
        if needs_refresh {
            *cache = Some((list_all_windows_users(), Instant::now()));
        }
        cache.as_ref().map(|(v, _)| v.clone()).unwrap_or_default()
    })
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
    let pal = AppPalette::from_ui(ui);

    egui::Frame::NONE
        .fill(pal.bg_header)
        .inner_margin(12.0)
        .show(ui, |ui| {
            let avail_w = ui.available_width();
            let compact = avail_w < 920.0;

            // --- Row 1: Source Header & Action Buttons (Zero-collision responsive layout) ---
            let render_source_actions = |ui: &mut Ui,
                                         sources: &mut Vec<PathBuf>,
                                         on_clear: &mut Option<Box<dyn FnOnce()>>| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if !sources.is_empty() {
                    let clear_btn = egui::Button::new(RichText::new("🗑 Limpar").size(13.0))
                        .min_size(Vec2::new(72.0, 30.0));
                    if ui.add(clear_btn).clicked() {
                        if let Some(cb) = on_clear.take() {
                            cb();
                        }
                    }
                }

                let add_files_btn = egui::Button::new(
                    RichText::new("📄 + Arquivos").strong().size(13.0),
                )
                .min_size(Vec2::new(104.0, 30.0));
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
                        .size(13.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(37, 99, 235))
                .min_size(Vec2::new(96.0, 30.0));
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
                    RichText::new("⚡ Backup Usuário")
                        .strong()
                        .size(13.0)
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(124, 58, 237))
                .min_size(Vec2::new(138.0, 30.0));
                if ui
                    .add(quick_btn)
                    .on_hover_text(
                        "Adiciona automaticamente Área de Trabalho, Documentos, Downloads, Imagens, Vídeos e Chrome do usuário atual",
                    )
                    .clicked()
                {
                    for preset in get_current_user_presets() {
                        if preset.exists && !sources.contains(&preset.path) {
                            sources.push(preset.path);
                        }
                    }
                }

                // Cached list of Windows users
                let other_users = cached_windows_users();
                if other_users.len() > 1 {
                    ui.menu_button(RichText::new("👤 Usuários ▾").size(13.0), |ui| {
                        for (name, user_path) in &other_users {
                            if ui.button(format!("👤 Carregar {name}")).clicked() {
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
            };

            let mut clear_cb: Option<Box<dyn FnOnce()>> = Some(Box::new(on_clear));

            ui.horizontal(|ui| {
                ui.label(RichText::new("📁 Origens:").strong().size(15.5));
                if !sources.is_empty() {
                    let manage_btn = ui.add(
                        egui::Button::new(
                            RichText::new(format!("📋 Gerenciar ({})", sources.len()))
                                .size(13.0)
                                .color(pal.accent)
                                .strong(),
                        )
                        .frame(false),
                    );
                    if manage_btn
                        .on_hover_text(
                            "Clique para abrir o Gerenciador de Pastas e Arquivos de Origem",
                        )
                        .clicked()
                    {
                        open_sources_manager_requested = true;
                    }
                }

                if !is_running && !compact {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        render_source_actions(ui, sources, &mut clear_cb);
                    });
                }
            });

            // On narrow windows (< 920px), wrap action buttons cleanly onto a dedicated row so nothing overlaps!
            if !is_running && compact {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    render_source_actions(ui, sources, &mut clear_cb);
                });
            }

            // --- Row 1.5: List of Selected Source Badges (Truncated & Wrapped, Zero Overflow) ---
            if sources.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(
                            "Nenhuma pasta adicionada. Clique em '⚡ Backup Usuário', '+ Pastas' ou arraste itens para cá.",
                        )
                        .italics()
                        .size(13.0)
                        .color(pal.text_muted),
                    );
                });
            } else {
                ui.add_space(3.0);
                let mut remove_idx = None;

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);

                    // Show at most the first 3 items as compact truncated badges
                    for (idx, src) in sources.iter().enumerate().take(3) {
                        let is_dir = cached_is_dir(src);
                        let icon = if is_dir { "📁" } else { "📄" };
                        let full_path = src.to_string_lossy().to_string();
                        let raw_name = src
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&full_path);
                        let short_name = truncate_label(raw_name, 22);

                        egui::Frame::NONE
                            .fill(pal.bg_badge)
                            .stroke(Stroke::new(1.0_f32, pal.border_strong))
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(egui::Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.label(
                                        RichText::new(format!("{icon} {short_name}"))
                                            .size(12.5)
                                            .strong()
                                            .color(pal.text_primary),
                                    )
                                    .on_hover_text(&full_path);

                                    if !is_running {
                                        let remove_btn = ui.add(
                                            egui::Button::new(
                                                RichText::new("✖")
                                                    .color(pal.error)
                                                    .size(11.0)
                                                    .strong(),
                                            )
                                            .frame(false),
                                        );
                                        if remove_btn.on_hover_text("Remover origem").clicked() {
                                            remove_idx = Some(idx);
                                        }
                                    }
                                });
                            });
                    }

                    // If more than 3 items, show Dropdown Menu with the rest
                    if sources.len() > 3 {
                        let remaining_count = sources.len() - 3;
                        let menu_label = format!("📁 +{remaining_count} restantes ▾");

                        ui.menu_button(
                            RichText::new(menu_label)
                                .size(12.5)
                                .strong()
                                .color(pal.text_primary),
                            |ui| {
                                ui.set_min_width(340.0);
                                ui.set_max_width(480.0);
                                ui.label(
                                    RichText::new(format!("Itens adicionais ({remaining_count}):"))
                                        .size(12.5)
                                        .color(pal.text_muted),
                                );
                                ui.separator();

                                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                                    for (idx, src) in sources.iter().enumerate().skip(3) {
                                        let is_dir = cached_is_dir(src);
                                        let icon = if is_dir { "📁" } else { "📄" };
                                        let path_str = src.to_string_lossy().to_string();

                                        ui.horizontal(|ui| {
                                            if !is_running {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui
                                                            .button(
                                                                RichText::new("✖")
                                                                    .color(pal.error)
                                                                    .size(11.0),
                                                            )
                                                            .clicked()
                                                        {
                                                            remove_idx = Some(idx);
                                                            ui.close_menu();
                                                        }
                                                        ui.add(
                                                            egui::Label::new(
                                                                RichText::new(format!(
                                                                    "{icon} {path_str}"
                                                                ))
                                                                .size(12.5)
                                                                .color(pal.text_primary),
                                                            )
                                                            .truncate(),
                                                        )
                                                        .on_hover_text(&path_str);
                                                    },
                                                );
                                            } else {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(format!("{icon} {path_str}"))
                                                            .size(12.5)
                                                            .color(pal.text_primary),
                                                    )
                                                    .truncate(),
                                                )
                                                .on_hover_text(&path_str);
                                            }
                                        });
                                    }
                                });

                                ui.add_space(6.0);
                                ui.separator();
                                if ui
                                    .button(
                                        RichText::new("📋 Abrir Gerenciador Completo...")
                                            .color(pal.accent)
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

            // --- Row 2: Destination Header, Space Checking & Picker (Never overflows!) ---
            let mut disk_space_info = None;
            if let Some(d) = destination.as_ref() {
                type DiskCache = (Option<(u64, u64)>, Option<std::path::PathBuf>, Instant);
                thread_local! {
                    static DISK_CACHE: std::cell::RefCell<DiskCache> =
                        std::cell::RefCell::new((None, None, Instant::now()));
                }
                disk_space_info = DISK_CACHE.with(|c| {
                    let mut cache = c.borrow_mut();
                    if cache.1.as_ref() != Some(d) || cache.2.elapsed().as_secs() >= 2 {
                        let info = get_disk_space(d);
                        cache.0 = info.map(|i| (i.available_bytes, i.total_bytes));
                        cache.1 = Some(d.clone());
                        cache.2 = Instant::now();
                    }
                    cache.0
                });
            }

            ui.horizontal(|ui| {
                ui.label(RichText::new("🎯 Destino:").strong().size(15.5));

                if !is_running {
                    let pick_dst_btn = egui::Button::new(
                        RichText::new("📂 Escolher Destino")
                            .strong()
                            .size(13.0)
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(16, 185, 129))
                    .min_size(Vec2::new(140.0, 30.0));
                    if ui.add(pick_dst_btn).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            *destination = Some(folder);
                        }
                    }
                }

                if let Some(dst) = destination.as_ref() {
                    let display_dst = dst.to_string_lossy().to_string();
                    let space_str = disk_space_info.map_or_else(String::new, |(avail, total)| {
                        format!(
                            "Livre: {} / {}",
                            format_bytes(avail),
                            format_bytes(total)
                        )
                    });
                    let mut clear_dest = false;

                    egui::Frame::NONE
                        .fill(pal.dest_bg)
                        .stroke(Stroke::new(1.0_f32, pal.dest_border))
                        .corner_radius(CornerRadius::same(5))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Draw right-aligned controls FIRST so long paths truncate safely without ever pushing '✖' off-screen!
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if !is_running
                                            && ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new("✖")
                                                            .color(pal.error)
                                                            .size(11.0)
                                                            .strong(),
                                                    )
                                                    .frame(false),
                                                )
                                                .on_hover_text("Limpar destino")
                                                .clicked()
                                        {
                                            clear_dest = true;
                                        }

                                        if !space_str.is_empty() {
                                            ui.label(
                                                RichText::new(format!("({space_str})"))
                                                    .size(12.5)
                                                    .color(pal.dest_subtext),
                                            );
                                        }

                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(format!("📂 {display_dst}"))
                                                            .strong()
                                                            .size(13.0)
                                                            .color(pal.dest_text),
                                                    )
                                                    .truncate(),
                                                )
                                                .on_hover_text(&display_dst);
                                            },
                                        );
                                    },
                                );
                            });
                        });

                    if clear_dest {
                        *destination = None;
                    }
                } else {
                    ui.add(
                        egui::Label::new(
                            RichText::new("Selecione a pasta de destino para o backup...")
                                .italics()
                                .size(13.5)
                                .color(pal.text_muted),
                        )
                        .truncate(),
                    );
                }
            });

            // Pre-Flight Disk Space Warning Alert
            if let Some((avail, _total)) = disk_space_info {
                if total_bytes > 0 && total_bytes > avail {
                    ui.add_space(4.0);
                    egui::Frame::NONE
                        .fill(pal.alert_error_bg)
                        .stroke(Stroke::new(1.0_f32, pal.error))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "⚠ ATENÇÃO: Espaço insuficiente no destino! Necessário: {} | Disponível: {}",
                                        format_bytes(total_bytes),
                                        format_bytes(avail)
                                    ))
                                    .color(pal.error)
                                    .strong()
                                    .size(13.0),
                                );
                            });
                        });
                }
            }

            ui.add_space(5.0);
            ui.separator();
            ui.add_space(5.0);

            // --- Row 3: Action Controls Bar (Compact labels + bounded ComboBoxes = Zero overlap at 800px!) ---
            ui.horizontal(|ui| {
                // 1. Botão Principal de Transferência
                if !is_running {
                    let can_start = !sources.is_empty() && destination.is_some();
                    let (start_label, start_color) = match config.transfer_mode {
                        crate::core::types::TransferMode::Copy => {
                            ("▶ INICIAR CÓPIA", Color32::from_rgb(22, 163, 74))
                        }
                        crate::core::types::TransferMode::Move => {
                            ("▶ MOVER ARQUIVOS", Color32::from_rgb(217, 119, 6))
                        }
                        crate::core::types::TransferMode::Sync => {
                            ("▶ SINCRONIZAR", Color32::from_rgb(14, 165, 233))
                        }
                    };
                    let start_btn = egui::Button::new(
                        RichText::new(start_label)
                            .strong()
                            .size(14.5)
                            .color(if can_start {
                                Color32::WHITE
                            } else {
                                pal.text_muted
                            }),
                    )
                    .fill(if can_start {
                        start_color
                    } else {
                        pal.btn_disabled_bg
                    })
                    .min_size(Vec2::new(165.0, 34.0));

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
                        RichText::new(pause_label)
                            .strong()
                            .size(13.5)
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(202, 138, 4))
                    .min_size(Vec2::new(118.0, 34.0));
                    if ui.add(pause_btn).clicked() {
                        on_toggle_pause();
                    }

                    ui.add_space(4.0);

                    let cancel_btn = egui::Button::new(
                        RichText::new("⏹ CANCELAR")
                            .strong()
                            .size(13.5)
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(185, 28, 28))
                    .min_size(Vec2::new(118.0, 34.0));
                    if ui.add(cancel_btn).clicked() {
                        on_cancel();
                    }
                }

                ui.add_space(6.0);

                // 2. Seletor de Modo de Transferência com largura controlada
                ui.add_enabled_ui(!is_running, |ui| {
                    egui::ComboBox::from_id_salt("header_mode_combo")
                        .width(190.0)
                        .selected_text(
                            RichText::new(config.transfer_mode.label())
                                .size(13.0)
                                .strong(),
                        )
                        .show_ui(ui, |ui| {
                            for mode in crate::core::types::TransferMode::all() {
                                ui.selectable_value(
                                    &mut config.transfer_mode,
                                    mode,
                                    RichText::new(mode.label()).size(13.0),
                                );
                            }
                        });
                });

                // 3. Regra de Sobrescrita alinhada à direita com short_label para nunca colidir
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(!is_running, |ui| {
                        egui::ComboBox::from_id_salt("header_overwrite_combo")
                            .width(175.0)
                            .selected_text(
                                RichText::new(config.overwrite_policy.short_label()).size(13.0),
                            )
                            .show_ui(ui, |ui| {
                                for policy in OverwritePolicy::all() {
                                    ui.selectable_value(
                                        &mut config.overwrite_policy,
                                        policy,
                                        RichText::new(policy.label()).size(13.0),
                                    );
                                }
                            });
                        ui.label(
                            RichText::new("Regra:")
                                .size(13.0)
                                .strong()
                                .color(pal.text_secondary),
                        );
                    });
                });
            });

            ui.add_space(5.0);

            // --- Row 3.5: Opções Rápidas de Transferência (Checkboxes alinhados e responsivos) ---
            ui.add_enabled_ui(!is_running, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Opções:").size(13.0).color(pal.text_muted));
                    ui.add_space(4.0);

                    ui.checkbox(
                        &mut config.verify_checksum,
                        RichText::new("Verificar xxHash3").size(13.0),
                    );
                    ui.add_space(10.0);

                    ui.checkbox(
                        &mut config.ignore_errors,
                        RichText::new("Ignorar erros").size(13.0),
                    );
                    ui.add_space(10.0);

                    ui.checkbox(
                        &mut config.preserve_metadata,
                        RichText::new("Preservar metadados").size(13.0),
                    );
                });
            });

            ui.add_space(6.0);

            // --- Row 4: Dual Progress Bars (Safe right-first layout -> Zero text collision!) ---
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
            } else if total_files > 0 && (copied_files + skipped_files + error_files == total_files)
            {
                (1.0, "Transferência Concluída!", "100.0%".to_string())
            } else {
                (0.0, "Aguardando início...", "-".to_string())
            };

            // Render right-aligned percentage FIRST inside a single horizontal row so header never expands vertically!
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(&file_pct_str).size(13.5).monospace());
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Arquivo:").size(13.5));
                        ui.add(
                            egui::Label::new(
                                RichText::new(file_label_str)
                                    .size(13.5)
                                    .strong()
                                    .color(pal.text_primary),
                            )
                            .truncate(),
                        )
                        .on_hover_text(file_label_str);
                    });
                });
            });
            ui.add(
                ProgressBar::new(file_progress)
                    .animate(is_running && !is_paused)
                    .fill(pal.accent),
            );

            ui.add_space(4.0);

            // Progress Bar 2: Total Transfer
            let processed_files_count = copied_files + skipped_files + error_files;
            let is_finished =
                total_files > 0 && processed_files_count == total_files && !is_running;

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
            let total_display_bytes = if is_finished {
                total_bytes
            } else {
                processed_bytes.min(total_bytes)
            };

            // Render right-aligned speed/ETA FIRST inside a single horizontal row so header never expands vertically!
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_running {
                        ui.label(
                            RichText::new(format!("⏳ {eta_str}  |  ⚡ {speed_str}"))
                                .size(13.5)
                                .color(pal.warning)
                                .strong(),
                        );
                    } else if is_finished {
                        ui.label(
                            RichText::new(format!(
                                "✔ {} ({} ok, {} pul., {} err.)",
                                format_duration(elapsed),
                                copied_files,
                                skipped_files,
                                error_files
                            ))
                            .size(13.5)
                            .color(pal.success)
                            .strong(),
                        );
                    }

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Total:").size(13.5).strong());
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!(
                                    "{} / {} ({:.1}%)  •  {}/{} itens",
                                    format_bytes(total_display_bytes),
                                    format_bytes(total_bytes),
                                    total_progress * 100.0,
                                    processed_files_count,
                                    total_files,
                                ))
                                .size(13.5)
                                .strong(),
                            )
                            .truncate(),
                        );
                    });
                });
            });
            ui.add(
                ProgressBar::new(total_progress)
                    .animate(is_running && !is_paused)
                    .fill(pal.success),
            );
        });

    if open_sources_manager_requested {
        on_open_sources_manager();
    }
}
