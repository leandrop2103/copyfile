//! Options and settings configuration panel.

use crate::core::types::{CopyConfig, OverwritePolicy};
use eframe::egui::{RichText, Slider, Ui};

/// Renders the options / preferences view.
pub fn render_options(ui: &mut Ui, config: &mut CopyConfig, is_running: bool) {
    ui.add_enabled_ui(!is_running, |ui| {
        ui.group(|ui| {
            ui.label(RichText::new("⚡ Desempenho e Concorrência").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Threads paralelas:");
                ui.add(Slider::new(&mut config.num_threads, 1..=32).text("workers"));
            });

            ui.horizontal(|ui| {
                ui.label("Tamanho do Buffer de I/O:");
                let mut buffer_mb = (config.buffer_size_kb / 1024).max(1);
                if ui.add(Slider::new(&mut buffer_mb, 1..=16).text("MB por thread")).changed() {
                    config.buffer_size_kb = buffer_mb * 1024;
                }
            });
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("🔄 Regra de Substituição de Arquivos Existentes").strong());
            ui.add_space(4.0);

            for policy in OverwritePolicy::all() {
                ui.radio_value(&mut config.overwrite_policy, policy, policy.label());
            }
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("🛡 Integridade e Resiliência").strong());
            ui.add_space(4.0);

            ui.checkbox(
                &mut config.verify_checksum,
                "Verificar integridade com xxHash3 (XXH3 - SIMD 30+ GB/s)",
            );
            ui.checkbox(
                &mut config.ignore_errors,
                "Ignorar erros durante a cópia (continua transferindo os demais arquivos)",
            );
            ui.checkbox(
                &mut config.preserve_metadata,
                "Preservar metadados completos (Data de Criação, Modificação, Atributos do Windows)",
            );
        });
    });

    if is_running {
        ui.add_space(10.0);
        ui.label(
            RichText::new(
                "ℹ As configurações não podem ser alteradas durante uma transferência ativa.",
            )
            .italics()
            .color(eframe::egui::Color32::from_rgb(148, 163, 184)),
        );
    }
}
