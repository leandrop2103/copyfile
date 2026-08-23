//! Structured logging view with export and clipboard capabilities.

use crate::core::types::{LogEntry, LogLevel};
use crate::gui::theme::{COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS, COLOR_WARNING};
use eframe::egui::{self, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use std::fs;

/// Log level filter for log tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFilter {
    #[default]
    All,
    ErrorsOnly,
    WarningsOnly,
}

/// Renders the structured event log panel.
pub fn render_logs(
    ui: &mut Ui,
    logs: &[LogEntry],
    filter: &mut LogFilter,
    search_query: &mut String,
    on_clear: impl FnOnce(),
) {
    // --- Toolbar ---
    ui.horizontal(|ui| {
        ui.label(RichText::new("Filtrar Nível:").strong());

        ui.selectable_value(filter, LogFilter::All, "Todos");
        ui.selectable_value(
            filter,
            LogFilter::ErrorsOnly,
            RichText::new("Apenas Erros").color(COLOR_ERROR),
        );
        ui.selectable_value(
            filter,
            LogFilter::WarningsOnly,
            RichText::new("Avisos / Ignorados").color(COLOR_WARNING),
        );

        ui.separator();
        ui.label(RichText::new("🔍").size(15.0));
        ui.add(
            egui::TextEdit::singleline(search_query)
                .hint_text("Buscar nos logs...")
                .desired_width(150.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🗑 Limpar Logs").clicked() {
                on_clear();
            }

            if ui.button("💾 Exportar .txt").clicked() {
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

            if ui.button("📋 Copiar Logs").clicked() {
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
        });
    });

    ui.separator();

    let q = search_query.to_lowercase();
    let filtered_logs: Vec<&LogEntry> = logs
        .iter()
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
            log.message.to_lowercase().contains(&q)
                || log
                    .path
                    .as_ref()
                    .is_some_and(|p| p.to_string_lossy().to_lowercase().contains(&q))
        })
        .collect();

    if filtered_logs.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Nenhum registro de log para o filtro atual.")
                    .color(COLOR_MUTED)
                    .italics(),
            );
        });
        return;
    }

    // --- Table of Logs ---
    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(80.0)) // Timestamp
        .column(Column::exact(80.0)) // Nível
        .column(Column::initial(250.0).at_least(100.0)) // Caminho
        .column(Column::remainder()) // Mensagem
        .header(24.0, |mut header| {
            header.col(|ui| {
                ui.strong("Horário");
            });
            header.col(|ui| {
                ui.strong("Nível");
            });
            header.col(|ui| {
                ui.strong("Arquivo / Caminho");
            });
            header.col(|ui| {
                ui.strong("Mensagem / Causa");
            });
        })
        .body(|body| {
            let row_height = 20.0;
            body.rows(row_height, filtered_logs.len(), |mut row| {
                let log = filtered_logs[row.index()];

                let (level_color, level_str) = match log.level {
                    LogLevel::Info => (Color32::from_rgb(148, 163, 184), "INFO"),
                    LogLevel::Success => (COLOR_SUCCESS, "SUCESSO"),
                    LogLevel::Warn => (COLOR_WARNING, "AVISO"),
                    LogLevel::Error => (COLOR_ERROR, "ERRO"),
                };

                row.col(|ui| {
                    ui.label(
                        RichText::new(log.timestamp.format("%H:%M:%S").to_string())
                            .monospace()
                            .small()
                            .color(COLOR_MUTED),
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
                    ui.label(RichText::new(path_text).small());
                });

                row.col(|ui| {
                    ui.label(RichText::new(&log.message).color(level_color).small());
                });
            });
        });
}
