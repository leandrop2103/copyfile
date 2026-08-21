//! Options and settings configuration panel.

use crate::core::types::{CopyConfig, OverwritePolicy};
use eframe::egui::{RichText, Slider, Ui};

/// Renders the options / preferences view.
pub fn render_options(ui: &mut Ui, config: &mut CopyConfig, is_running: bool) {
    ui.add_enabled_ui(!is_running, |ui| {
        ui.group(|ui| {
            ui.label(RichText::new("⚡ Desempenho e Concorrência").strong().size(16.0));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Threads paralelas:").size(14.0));
                ui.add(Slider::new(&mut config.num_threads, 1..=32).text("workers"));
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Tamanho do Buffer de I/O:").size(14.0));
                let mut buffer_mb = (config.buffer_size_kb / 1024).max(1);
                if ui.add(Slider::new(&mut buffer_mb, 1..=16).text("MB por thread")).changed() {
                    config.buffer_size_kb = buffer_mb * 1024;
                }
            });
        });

        ui.add_space(12.0);

        ui.group(|ui| {
            ui.label(RichText::new("🔄 Regra de Substituição de Arquivos Existentes").strong().size(16.0));
            ui.add_space(6.0);

            for policy in OverwritePolicy::all() {
                ui.radio_value(
                    &mut config.overwrite_policy,
                    policy,
                    RichText::new(policy.label()).size(14.0),
                );
            }
        });

        ui.add_space(12.0);

        ui.group(|ui| {
            ui.label(RichText::new("🛡 Integridade e Resiliência").strong().size(16.0));
            ui.add_space(6.0);

            ui.checkbox(
                &mut config.verify_checksum,
                RichText::new("Verificar integridade com xxHash3 (XXH3 - SIMD 30+ GB/s)").size(14.0),
            );
            ui.add_space(4.0);
            ui.checkbox(
                &mut config.ignore_errors,
                RichText::new("Ignorar erros durante a cópia (continua transferindo os demais arquivos)").size(14.0),
            );
            ui.add_space(4.0);
            ui.checkbox(
                &mut config.preserve_metadata,
                RichText::new("Preservar metadados completos (Data de Criação, Modificação, Atributos do Windows)").size(14.0),
            );
        });
    });

    if is_running {
        ui.add_space(12.0);
        ui.label(
            RichText::new(
                "ℹ As configurações não podem ser alteradas durante uma transferência ativa.",
            )
            .italics()
            .size(14.0)
            .color(eframe::egui::Color32::from_rgb(148, 163, 184)),
        );
    }
}
