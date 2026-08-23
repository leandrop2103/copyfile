//! File list view rendered as a virtualized table with status color coding and auto-scrolling.

use crate::core::format::format_bytes;
use crate::core::types::{FileEntry, FileStatus};
use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS, COLOR_WARNING};
use eframe::egui::{self, RichText, ScrollArea, Ui};
use egui_extras::{Column, TableBuilder};

/// Filter options for viewing files in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListFilter {
    #[default]
    All,
    SuccessOnly,
    SkippedOnly,
    ErrorsOnly,
}

fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.char_indices().any(|(i, _)| {
        haystack[i..]
            .chars()
            .zip(needle.chars())
            .all(|(h, n)| h.to_lowercase().eq(n.to_lowercase()))
    })
}

/// Renders the file list table.
pub fn render_file_list(
    ui: &mut Ui,
    entries: &[FileEntry],
    filter: &mut ListFilter,
    search_query: &mut String,
    auto_scroll: &mut bool,
    is_running: bool,
    on_retry_failed: impl FnOnce(),
) {
    // Count items by status
    let (mut success_count, mut skipped_count, mut error_count) = (0, 0, 0);
    for entry in entries {
        match &entry.status {
            FileStatus::Success { .. } => success_count += 1,
            FileStatus::Skipped { .. } => skipped_count += 1,
            FileStatus::Error { .. } => error_count += 1,
            _ => {}
        }
    }
    let total_count = entries.len();

    // --- Toolbar ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filtrar:").strong().size(15.0));

        ui.selectable_value(
            filter,
            ListFilter::All,
            RichText::new(format!("Todos ({total_count})")).size(14.0),
        );
        ui.selectable_value(
            filter,
            ListFilter::SuccessOnly,
            RichText::new(format!("✔ Concluídos ({success_count})"))
                .size(14.0)
                .color(COLOR_SUCCESS),
        );
        ui.selectable_value(
            filter,
            ListFilter::SkippedOnly,
            RichText::new(format!("⚠ Ignorados ({skipped_count})"))
                .size(14.0)
                .color(COLOR_WARNING),
        );
        ui.selectable_value(
            filter,
            ListFilter::ErrorsOnly,
            RichText::new(format!("✖ Erros ({error_count})"))
                .size(14.0)
                .color(COLOR_ERROR),
        );

        if !is_running && error_count > 0 {
            ui.add_space(8.0);
            let retry_btn = egui::Button::new(
                RichText::new(format!("🔁 Tentar Novamente com Falhas ({error_count})"))
                    .size(13.5)
                    .strong()
                    .color(eframe::egui::Color32::WHITE),
            )
            .fill(COLOR_ERROR);

            if ui
                .add(retry_btn)
                .on_hover_text("Reprocessar apenas os arquivos que apresentaram erro")
                .clicked()
            {
                on_retry_failed();
            }
        }

        ui.separator();

        ui.label(RichText::new("🔍").size(15.0));
        ui.add(
            egui::TextEdit::singleline(search_query)
                .hint_text("Buscar por nome de arquivo...")
                .desired_width(200.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(
                auto_scroll,
                RichText::new("⬇ Auto rolagem").strong().size(14.0),
            );
        });
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // Filter items based on active criteria and search query
    let filtered_indices: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            // Status filter
            let status_match = match filter {
                ListFilter::All => true,
                ListFilter::SuccessOnly => matches!(entry.status, FileStatus::Success { .. }),
                ListFilter::SkippedOnly => matches!(entry.status, FileStatus::Skipped { .. }),
                ListFilter::ErrorsOnly => matches!(entry.status, FileStatus::Error { .. }),
            };

            if !status_match {
                return false;
            }

            // Search filter
            if search_query.is_empty() {
                true
            } else {
                let q = search_query.to_lowercase();
                contains_case_insensitive(&entry.source_path.to_string_lossy(), &q)
                    || contains_case_insensitive(&entry.dest_path.to_string_lossy(), &q)
            }
        })
        .map(|(i, _)| i)
        .collect();

    if filtered_indices.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(
                RichText::new("Nenhum arquivo na fila de transferência.")
                    .color(COLOR_MUTED)
                    .size(16.0)
                    .italics(),
            );
        });
        return;
    }

    // Find active copying file or most recently processed file for auto-scrolling
    let active_row_idx = if is_running {
        // Priority 1: Currently copying or verifying file
        filtered_indices
            .iter()
            .position(|&idx| {
                matches!(
                    entries[idx].status,
                    FileStatus::Copying { .. } | FileStatus::Verifying
                )
            })
            // Priority 2: Most recently processed file (Success, Skipped with xxHash match, Error)
            .or_else(|| {
                filtered_indices
                    .iter()
                    .rposition(|&idx| !matches!(entries[idx].status, FileStatus::Pending))
            })
    } else {
        None
    };

    // Smart auto-scroll: pause if user scrolls manually
    if *auto_scroll && ui.rect_contains_pointer(ui.max_rect()) {
        ui.input(|i| {
            if i.raw_scroll_delta.y.abs() > 0.1
                || (i.pointer.primary_down() && i.pointer.is_moving())
            {
                *auto_scroll = false;
            }
        });
    }

    // --- Excel-like Scrollable Table with Horizontal & Vertical Freedom ---
    ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(36.0)) // Status Icon Only
                .column(Column::initial(300.0).at_least(140.0)) // Origem / Caminho
                .column(Column::exact(100.0)) // xxHash
                .column(Column::exact(85.0)) // Tamanho
                .column(Column::exact(65.0)) // Atributos
                .column(Column::initial(200.0).at_least(100.0)); // Detalhes / Motivo

            if *auto_scroll {
                if let Some(active_idx) = active_row_idx {
                    table = table.scroll_to_row(active_idx, Some(egui::Align::Center));
                }
            }

            table
                .header(28.0, |mut header| {
                    header.col(|ui| {
                        ui.label(
                            RichText::new("⚡")
                                .size(14.0)
                                .color(COLOR_ACCENT)
                                .strong(),
                        )
                        .on_hover_text("Status da transferência (✔ Concluído, ⚡ Copiando, ❌ Erro, ⏭ Ignorado)");
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("Arquivo / Origem").size(14.0));
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("xxHash3").size(14.0));
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("Tamanho").size(14.0));
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("Atrib.").size(14.0));
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("Detalhes / Erro").size(14.0));
                    });
                })
                .body(|body| {
                    let row_height = 26.0;
                    body.rows(row_height, filtered_indices.len(), |mut row| {
                        let row_index = row.index();
                        let entry_idx = filtered_indices[row_index];
                        let entry = &entries[entry_idx];

                        // Determine text color and icon based on status
                        let (status_text, status_color, status_icon) = match &entry.status {
                            FileStatus::Pending => ("Pendente", COLOR_MUTED, "⏳"),
                            FileStatus::Copying { .. } => ("Copiando...", COLOR_ACCENT, "⚡"),
                            FileStatus::Verifying => ("Verificando...", COLOR_ACCENT, "🔍"),
                            FileStatus::Success { .. } => {
                                ("Concluído com sucesso", COLOR_SUCCESS, "✔")
                            }
                            FileStatus::Skipped { .. } => ("Ignorado", COLOR_WARNING, "⏭"),
                            FileStatus::Error { .. } => ("Erro na cópia", COLOR_ERROR, "❌"),
                        };

                        // Column 1: Status Icon only
                        row.col(|ui| {
                            ui.label(
                                RichText::new(status_icon)
                                    .color(status_color)
                                    .size(15.0)
                                    .strong(),
                            )
                            .on_hover_text(status_text);
                        });

                        // Column 2: Source Path
                        row.col(|ui| {
                            let text = entry.source_path.to_string_lossy();
                            let resp = ui.label(RichText::new(text).color(status_color).size(13.0));
                            resp.context_menu(|ui| {
                                if ui.button("📂 Abrir local do arquivo").clicked() {
                                    if let Some(parent) = entry.source_path.parent() {
                                        let _ = std::process::Command::new("explorer").arg(parent).spawn();
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("📋 Copiar caminho completo").clicked() {
                                    ui.ctx().copy_text(entry.source_path.to_string_lossy().to_string());
                                    ui.close_menu();
                                }
                                if let Some(hash) = &entry.hash {
                                    if ui.button("#️⃣ Copiar hash xxHash3").clicked() {
                                        ui.ctx().copy_text(hash.clone());
                                        ui.close_menu();
                                    }
                                }
                            });
                        });

                        // Column 3: Checksum Hash
                        row.col(|ui| {
                            let hash_str = entry.hash.as_deref().unwrap_or("-");
                            ui.label(
                                RichText::new(hash_str)
                                    .monospace()
                                    .size(13.0)
                                    .color(COLOR_ACCENT),
                            );
                        });

                        // Column 5: File Size
                        row.col(|ui| {
                            ui.label(
                                RichText::new(format_bytes(entry.size))
                                    .size(13.0)
                                    .monospace(),
                            );
                        });

                        // Column 6: Attributes
                        row.col(|ui| {
                            ui.label(
                                RichText::new(&entry.attributes)
                                    .monospace()
                                    .size(13.0)
                                    .color(COLOR_MUTED),
                            );
                        });

                        // Column 7: Details / Error Reason
                        row.col(|ui| match &entry.status {
                            FileStatus::Error { error } => {
                                ui.label(RichText::new(error).color(COLOR_ERROR).size(13.0));
                            }
                            FileStatus::Skipped { reason } => {
                                ui.label(RichText::new(reason).color(COLOR_WARNING).size(13.0));
                            }
                            FileStatus::Success { duration_ms, .. } => {
                                ui.label(
                                    RichText::new(format!("{duration_ms}ms"))
                                        .color(COLOR_SUCCESS)
                                        .size(13.0),
                                );
                            }
                            FileStatus::Copying {
                                bytes_copied,
                                total_bytes,
                            } => {
                                let pct = if *total_bytes > 0 {
                                    (*bytes_copied as f64 / *total_bytes as f64) * 100.0
                                } else {
                                    0.0
                                };
                                ui.label(
                                    RichText::new(format!("{pct:.1}%"))
                                        .color(COLOR_ACCENT)
                                        .size(13.0)
                                        .strong(),
                                );
                            }
                            _ => {
                                ui.label(RichText::new("-").color(COLOR_MUTED).size(13.0));
                            }
                        });
                    });
                });
        });
}
