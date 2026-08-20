//! File list view rendered as a virtualized table with status color coding.

use crate::core::types::{FileEntry, FileStatus};
use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS, COLOR_WARNING};
use crate::gui::views::header::format_bytes;
use eframe::egui::{self, Color32, RichText, Ui};
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

/// Renders the file list table.
pub fn render_file_list(
    ui: &mut Ui,
    entries: &[FileEntry],
    filter: &mut ListFilter,
    search_query: &mut String,
    auto_scroll: &mut bool,
) {
    // Count items by status
    let total_count = entries.len();
    let success_count = entries
        .iter()
        .filter(|e| matches!(e.status, FileStatus::Success { .. }))
        .count();
    let skipped_count = entries
        .iter()
        .filter(|e| matches!(e.status, FileStatus::Skipped { .. }))
        .count();
    let error_count = entries
        .iter()
        .filter(|e| matches!(e.status, FileStatus::Error { .. }))
        .count();

    // --- Toolbar ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filtrar:").strong());

        ui.selectable_value(filter, ListFilter::All, format!("Todos ({total_count})"));
        ui.selectable_value(
            filter,
            ListFilter::SuccessOnly,
            RichText::new(format!("✔ Concluídos ({success_count})")).color(COLOR_SUCCESS),
        );
        ui.selectable_value(
            filter,
            ListFilter::SkippedOnly,
            RichText::new(format!("⚠ Ignorados ({skipped_count})")).color(COLOR_WARNING),
        );
        ui.selectable_value(
            filter,
            ListFilter::ErrorsOnly,
            RichText::new(format!("✖ Erros ({error_count})")).color(COLOR_ERROR),
        );

        ui.separator();

        ui.label("🔍");
        ui.add(egui::TextEdit::singleline(search_query).hint_text("Buscar arquivo..."));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(auto_scroll, "Auto rolagem");
        });
    });

    ui.separator();

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
                entry
                    .source_path
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&q)
                    || entry
                        .dest_path
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&q)
            }
        })
        .map(|(i, _)| i)
        .collect();

    if filtered_indices.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Nenhum arquivo para exibir na lista.")
                    .color(COLOR_MUTED)
                    .italics(),
            );
        });
        return;
    }

    // --- Virtualized Table Builder ---
    let table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(85.0)) // Status
        .column(Column::initial(300.0).at_least(150.0)) // Origem
        .column(Column::initial(300.0).at_least(150.0)) // Destino
        .column(Column::exact(95.0)) // xxHash
        .column(Column::exact(85.0)) // Tamanho
        .column(Column::exact(65.0)) // Atributos
        .column(Column::remainder()); // Detalhes / Motivo

    table
        .header(24.0, |mut header| {
            header.col(|ui| {
                ui.strong("Status");
            });
            header.col(|ui| {
                ui.strong("Origem");
            });
            header.col(|ui| {
                ui.strong("Destino");
            });
            header.col(|ui| {
                ui.strong("xxHash");
            });
            header.col(|ui| {
                ui.strong("Tamanho");
            });
            header.col(|ui| {
                ui.strong("Atrib.");
            });
            header.col(|ui| {
                ui.strong("Detalhes / Erro");
            });
        })
        .body(|body| {
            let row_height = 22.0;
            body.rows(row_height, filtered_indices.len(), |mut row| {
                let row_index = row.index();
                let entry_idx = filtered_indices[row_index];
                let entry = &entries[entry_idx];

                // Determine text color and icon based on status
                let (status_text, status_color, status_icon) = match &entry.status {
                    FileStatus::Pending => ("Pendente", COLOR_MUTED, "⏳"),
                    FileStatus::Copying { .. } => ("Copiando", COLOR_ACCENT, "🔄"),
                    FileStatus::Verifying => ("Verificando", COLOR_ACCENT, "🔍"),
                    FileStatus::Success { .. } => ("Concluído", COLOR_SUCCESS, "✔"),
                    FileStatus::Skipped { .. } => ("Ignorado", COLOR_WARNING, "⚠"),
                    FileStatus::Error { .. } => ("Erro", COLOR_ERROR, "✖"),
                };

                // Column 1: Status badge
                row.col(|ui| {
                    ui.label(
                        RichText::new(format!("{status_icon} {status_text}"))
                            .color(status_color)
                            .small()
                            .strong(),
                    );
                });

                // Column 2: Source Path
                row.col(|ui| {
                    let text = entry.source_path.to_string_lossy();
                    ui.label(RichText::new(text).color(status_color).small());
                });

                // Column 3: Destination Path
                row.col(|ui| {
                    let text = entry.dest_path.to_string_lossy();
                    ui.label(
                        RichText::new(text)
                            .small()
                            .color(Color32::from_rgb(203, 213, 225)),
                    );
                });

                // Column 4: Checksum Hash
                row.col(|ui| {
                    let hash_str = entry.hash.as_deref().unwrap_or("-");
                    ui.label(
                        RichText::new(hash_str)
                            .monospace()
                            .small()
                            .color(COLOR_ACCENT),
                    );
                });

                // Column 5: File Size
                row.col(|ui| {
                    ui.label(RichText::new(format_bytes(entry.size)).small());
                });

                // Column 6: Attributes
                row.col(|ui| {
                    ui.label(
                        RichText::new(&entry.attributes)
                            .monospace()
                            .small()
                            .color(COLOR_MUTED),
                    );
                });

                // Column 7: Details / Error Reason
                row.col(|ui| match &entry.status {
                    FileStatus::Error { error } => {
                        ui.label(RichText::new(error).color(COLOR_ERROR).small());
                    }
                    FileStatus::Skipped { reason } => {
                        ui.label(RichText::new(reason).color(COLOR_WARNING).small());
                    }
                    FileStatus::Success { duration_ms, .. } => {
                        ui.label(
                            RichText::new(format!("{duration_ms}ms"))
                                .color(COLOR_SUCCESS)
                                .small(),
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
                                .small(),
                        );
                    }
                    _ => {
                        ui.label(RichText::new("-").color(COLOR_MUTED).small());
                    }
                });
            });
        });
}
