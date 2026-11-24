//! File list view rendered as a virtualized table with O(1) unfiltered fast-path,
//! fast ASCII case-insensitive filtering, clipped columns, and smart auto-scrolling.

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

/// Fast case-insensitive substring search (O(n) byte-level fast path for ASCII).
#[inline]
fn contains_case_insensitive(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle_lower.is_ascii() {
        let h = haystack.as_bytes();
        let n = needle_lower.as_bytes();
        if n.len() > h.len() {
            return false;
        }
        return h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n));
    }
    haystack.to_lowercase().contains(needle_lower)
}

/// Renders the file list table with O(1) counts and zero-allocation fast path when unfiltered.
#[allow(clippy::too_many_arguments)]
pub fn render_file_list(
    ui: &mut Ui,
    entries: &[FileEntry],
    success_count: usize,
    skipped_count: usize,
    error_count: usize,
    last_active_entry_id: Option<usize>,
    filter: &mut ListFilter,
    search_query: &mut String,
    auto_scroll: &mut bool,
    is_running: bool,
    on_retry_failed: impl FnOnce(),
) {
    let total_count = entries.len();

    // --- Responsive Toolbar (Never overflows or collides at 800px!) ---
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new("Filtrar:").strong().size(14.0));

        ui.selectable_value(
            filter,
            ListFilter::All,
            RichText::new(format!("Todos ({total_count})")).size(13.5),
        );
        ui.selectable_value(
            filter,
            ListFilter::SuccessOnly,
            RichText::new(format!("✔ OK ({success_count})"))
                .size(13.5)
                .color(COLOR_SUCCESS),
        );
        ui.selectable_value(
            filter,
            ListFilter::SkippedOnly,
            RichText::new(format!("⚠ Pul. ({skipped_count})"))
                .size(13.5)
                .color(COLOR_WARNING),
        );
        ui.selectable_value(
            filter,
            ListFilter::ErrorsOnly,
            RichText::new(format!("✖ Erros ({error_count})"))
                .size(13.5)
                .color(COLOR_ERROR),
        );

        if !is_running && error_count > 0 {
            let retry_btn = egui::Button::new(
                RichText::new(format!("🔁 Retentar ({error_count})"))
                    .size(13.0)
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

        ui.label(RichText::new("🔍").size(14.0));
        ui.add(
            egui::TextEdit::singleline(search_query)
                .hint_text("Buscar arquivo...")
                .desired_width(160.0),
        );

        ui.checkbox(
            auto_scroll,
            RichText::new("⬇ Auto rolagem").strong().size(13.5),
        );
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // O(1) Fast Path when showing all entries without search query (avoids allocating 50k+ Vec every frame!)
    let is_unfiltered = *filter == ListFilter::All && search_query.is_empty();
    let filtered_indices: Vec<usize> = if is_unfiltered {
        Vec::new()
    } else {
        let q = search_query.to_lowercase();
        entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                let status_match = match filter {
                    ListFilter::All => true,
                    ListFilter::SuccessOnly => matches!(entry.status, FileStatus::Success { .. }),
                    ListFilter::SkippedOnly => matches!(entry.status, FileStatus::Skipped { .. }),
                    ListFilter::ErrorsOnly => matches!(entry.status, FileStatus::Error { .. }),
                };
                if !status_match {
                    return false;
                }
                if q.is_empty() {
                    true
                } else {
                    contains_case_insensitive(&entry.source_path.to_string_lossy(), &q)
                        || contains_case_insensitive(&entry.dest_path.to_string_lossy(), &q)
                }
            })
            .map(|(i, _)| i)
            .collect()
    };

    let row_count = if is_unfiltered {
        entries.len()
    } else {
        filtered_indices.len()
    };

    if row_count == 0 {
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

    // O(1) active row lookup when unfiltered, binary-search lookup when filtered!
    let active_row_idx = if is_running && *auto_scroll {
        if let Some(active_id) = last_active_entry_id {
            if is_unfiltered {
                if active_id < entries.len() {
                    Some(active_id)
                } else {
                    None
                }
            } else {
                filtered_indices.binary_search(&active_id).ok()
            }
        } else {
            None
        }
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

    // --- Excel-like Scrollable Table with Clipped Columns (Zero bleed!) ---
    ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(36.0)) // Status Icon
                .column(Column::initial(320.0).at_least(140.0).clip(true)) // Origem / Caminho
                .column(Column::exact(112.0).clip(true)) // xxHash3
                .column(Column::exact(85.0)) // Tamanho
                .column(Column::exact(60.0)) // Atributos
                .column(Column::remainder().at_least(140.0).clip(true)); // Detalhes / Motivo

            if *auto_scroll {
                if let Some(active_idx) = active_row_idx {
                    table = table.scroll_to_row(active_idx, Some(egui::Align::Center));
                }
            }

            table
                .header(26.0, |mut header| {
                    header.col(|ui| {
                        ui.label(
                            RichText::new("⚡")
                                .size(14.0)
                                .color(COLOR_ACCENT)
                                .strong(),
                        )
                        .on_hover_text(
                            "Status da transferência (✔ Concluído, ⚡ Copiando, 🔍 Verificando, ❌ Erro, ⏭ Ignorado)",
                        );
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("Arquivo / Origem").size(13.5));
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("xxHash3").size(13.5));
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("Tamanho").size(13.5));
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("Atrib.").size(13.5));
                    });
                    header.col(|ui| {
                        ui.strong(RichText::new("Detalhes / Erro").size(13.5));
                    });
                })
                .body(|body| {
                    let row_height = 25.0;
                    body.rows(row_height, row_count, |mut row| {
                        let row_index = row.index();
                        let entry_idx = if is_unfiltered {
                            row_index
                        } else {
                            filtered_indices[row_index]
                        };
                        let entry = &entries[entry_idx];

                        let (status_text, status_color, status_icon) = match &entry.status {
                            FileStatus::Pending => ("Pendente", COLOR_MUTED, "⏳"),
                            FileStatus::Copying { .. } => ("Copiando...", COLOR_ACCENT, "⚡"),
                            FileStatus::Verifying => ("Verificando xxHash3...", COLOR_WARNING, "🔍"),
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
                                    .size(14.5)
                                    .strong(),
                            )
                            .on_hover_text(status_text);
                        });

                        // Column 2: Source Path
                        row.col(|ui| {
                            let text = entry.source_path.to_string_lossy();
                            let resp = ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(text.as_ref())
                                            .color(status_color)
                                            .size(13.0),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(text.as_ref());
                            resp.context_menu(|ui| {
                                if ui.button("📂 Abrir local da origem").clicked() {
                                    if let Some(parent) = entry.source_path.parent() {
                                        let _ =
                                            std::process::Command::new("explorer").arg(parent).spawn();
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("🎯 Abrir pasta de destino").clicked() {
                                    if let Some(parent) = entry.dest_path.parent() {
                                        let _ =
                                            std::process::Command::new("explorer").arg(parent).spawn();
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("📋 Copiar caminho completo").clicked() {
                                    ui.ctx()
                                        .copy_text(entry.source_path.to_string_lossy().to_string());
                                    ui.close_menu();
                                }
                                if let Some(hash) = &entry.hash {
                                    if !hash.is_empty() && ui.button("#️⃣ Copiar hash xxHash3").clicked() {
                                        ui.ctx().copy_text(hash.clone());
                                        ui.close_menu();
                                    }
                                }
                            });
                        });

                        // Column 3: Checksum Hash
                        row.col(|ui| {
                            let hash_str = entry
                                .hash
                                .as_deref()
                                .filter(|s| !s.is_empty())
                                .unwrap_or("-");
                            ui.add(
                                egui::Label::new(
                                    RichText::new(hash_str)
                                        .monospace()
                                        .size(12.5)
                                        .color(COLOR_ACCENT),
                                )
                                .truncate(),
                            );
                        });

                        // Column 4: File Size
                        row.col(|ui| {
                            ui.label(
                                RichText::new(format_bytes(entry.size))
                                    .size(12.5)
                                    .monospace(),
                            );
                        });

                        // Column 5: Attributes
                        row.col(|ui| {
                            ui.label(
                                RichText::new(&entry.attributes)
                                    .monospace()
                                    .size(12.5)
                                    .color(COLOR_MUTED),
                            );
                        });

                        // Column 6: Details / Error Reason
                        row.col(|ui| match &entry.status {
                            FileStatus::Error { error } => {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(error).color(COLOR_ERROR).size(12.5),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(error);
                            }
                            FileStatus::Skipped { reason } => {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(reason).color(COLOR_WARNING).size(12.5),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(reason);
                            }
                            FileStatus::Verifying => {
                                ui.label(
                                    RichText::new("Verificando integridade...")
                                        .color(COLOR_WARNING)
                                        .size(12.5)
                                        .strong(),
                                );
                            }
                            FileStatus::Success { duration_ms, .. } => {
                                ui.label(
                                    RichText::new(format!("{duration_ms}ms"))
                                        .color(COLOR_SUCCESS)
                                        .size(12.5),
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
                                        .size(12.5)
                                        .strong(),
                                );
                            }
                            FileStatus::Pending => {
                                ui.label(RichText::new("-").color(COLOR_MUTED).size(12.5));
                            }
                        });
                    });
                });
        });
}
