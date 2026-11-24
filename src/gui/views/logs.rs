//! Structured logging view with export, clipboard capabilities, clipped columns, and fast filtering.

use crate::core::types::{LogEntry, LogLevel};
use crate::gui::theme::AppPalette;
use eframe::egui::{self, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use std::collections::VecDeque;
use std::fs;

/// Log level filter for log tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFilter {
    #[default]
    All,
    ErrorsOnly,
    WarningsOnly,
}

#[inline]
fn contains_ci(haystack: &str, needle_lower: &str) -> bool {
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

/// Renders the structured event log panel.
pub fn render_logs(
    ui: &mut Ui,
    logs: &VecDeque<LogEntry>,
    filter: &mut LogFilter,
    search_query: &mut String,
    on_clear: impl FnOnce(),
) {
    let pal = AppPalette::from_ui(ui);

    // --- Responsive Toolbar (Zero overlap at 800px!) ---
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new("Nível:").strong().size(13.5));

        ui.selectable_value(filter, LogFilter::All, RichText::new("Todos").size(13.0));
        ui.selectable_value(
            filter,
            LogFilter::ErrorsOnly,
            RichText::new("Erros").size(13.0).color(pal.error),
        );
        ui.selectable_value(
            filter,
            LogFilter::WarningsOnly,
            RichText::new("Avisos").size(13.0).color(pal.warning),
        );

        ui.separator();
        ui.label(RichText::new("🔍").size(14.0));
        ui.add(
            egui::TextEdit::singleline(search_query)
                .hint_text("Buscar nos logs...")
                .desired_width(150.0),
        );

        ui.separator();

        if ui
            .button(RichText::new("📋 Copiar").size(13.0))
            .on_hover_text("Copiar todos os logs para a área de transferência")
            .clicked()
        {
            let mut content = String::new();
            for log in logs {
                let path_str = log
                    .path
                    .as_ref()
                    .map_or_else(String::new, |p| format!(" [{}]", p.display()));
                content.push_str(&format!(
                    "[{}] [{}] {}{}\n",
                    log.timestamp.format("%H:%M:%S"),
                    log.level.as_str(),
                    log.message,
                    path_str
                ));
            }
            ui.ctx().copy_text(content);
        }

        if ui
            .button(RichText::new("💾 .txt").size(13.0))
            .on_hover_text("Exportar registros para arquivo .txt")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Log Files", &["txt", "log"])
                .set_file_name("copyfile_log.txt")
                .save_file()
            {
                let mut content = String::new();
                for log in logs {
                    let path_str = log
                        .path
                        .as_ref()
                        .map_or_else(String::new, |p| format!(" [{}]", p.display()));
                    content.push_str(&format!(
                        "[{}] [{}] {}{}\n",
                        log.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        log.level.as_str(),
                        log.message,
                        path_str
                    ));
                }
                let _ = fs::write(path, content);
            }
        }

        if ui
            .button(RichText::new("🗑 Limpar").size(13.0))
            .on_hover_text("Limpar registros de log")
            .clicked()
        {
            on_clear();
        }
    });

    ui.separator();

    let is_unfiltered = *filter == LogFilter::All && search_query.is_empty();
    let filtered_logs: Vec<&LogEntry> = if is_unfiltered {
        Vec::new()
    } else {
        let q = search_query.to_lowercase();
        logs.iter()
            .filter(|log| {
                let level_match = match filter {
                    LogFilter::All => true,
                    LogFilter::ErrorsOnly => log.level == LogLevel::Error,
                    LogFilter::WarningsOnly => log.level == LogLevel::Warn,
                };
                if !level_match {
                    return false;
                }
                if q.is_empty() {
                    return true;
                }
                contains_ci(&log.message, &q)
                    || log
                        .path
                        .as_ref()
                        .is_some_and(|p| contains_ci(&p.to_string_lossy(), &q))
            })
            .collect()
    };

    let row_count = if is_unfiltered {
        logs.len()
    } else {
        filtered_logs.len()
    };

    if row_count == 0 {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Nenhum registro de log para o filtro atual.")
                    .color(pal.text_muted)
                    .italics(),
            );
        });
        return;
    }

    // --- Table of Logs with Clipped Columns ---
    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .stick_to_bottom(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(76.0)) // Timestamp
        .column(Column::exact(76.0)) // Nível
        .column(Column::initial(240.0).at_least(100.0).clip(true)) // Caminho
        .column(Column::remainder().at_least(150.0).clip(true)) // Mensagem
        .header(24.0, |mut header| {
            header.col(|ui| {
                ui.strong(RichText::new("Horário").size(13.0));
            });
            header.col(|ui| {
                ui.strong(RichText::new("Nível").size(13.0));
            });
            header.col(|ui| {
                ui.strong(RichText::new("Arquivo / Caminho").size(13.0));
            });
            header.col(|ui| {
                ui.strong(RichText::new("Mensagem / Causa").size(13.0));
            });
        })
        .body(|body| {
            let row_height = 22.0;
            body.rows(row_height, row_count, |mut row| {
                let log = if is_unfiltered {
                    &logs[row.index()]
                } else {
                    filtered_logs[row.index()]
                };

                let (level_color, level_str) = match log.level {
                    LogLevel::Info => (pal.text_muted, "INFO"),
                    LogLevel::Success => (pal.success, "SUCESSO"),
                    LogLevel::Warn => (pal.warning, "AVISO"),
                    LogLevel::Error => (pal.error, "ERRO"),
                };

                row.col(|ui| {
                    ui.label(
                        RichText::new(log.timestamp.format("%H:%M:%S").to_string())
                            .monospace()
                            .small()
                            .color(pal.text_muted),
                    );
                });

                row.col(|ui| {
                    ui.label(RichText::new(level_str).color(level_color).small().strong());
                });

                row.col(|ui| {
                    let path_text = log
                        .path
                        .as_ref()
                        .map_or_else(|| "-".to_string(), |p| p.to_string_lossy().to_string());
                    ui.add(egui::Label::new(RichText::new(&path_text).small()).truncate())
                        .on_hover_text(&path_text);
                });

                row.col(|ui| {
                    ui.add(
                        egui::Label::new(RichText::new(&log.message).color(level_color).small())
                            .truncate(),
                    )
                    .on_hover_text(&log.message);
                });
            });
        });
}
