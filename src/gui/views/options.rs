//! Options and settings configuration panel.

use crate::core::shell::{
    is_context_menu_registered, register_context_menu, unregister_context_menu,
};
use crate::core::types::{CopyConfig, OverwritePolicy, PostCopyAction};
use crate::gui::theme::{COLOR_ERROR, COLOR_SUCCESS};
use eframe::egui::{Color32, RichText, ScrollArea, Slider, Ui};

/// Renders the options / preferences view inside a vertically scrollable container.
pub fn render_options(ui: &mut Ui, config: &mut CopyConfig, is_running: bool) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(4.0);

            ui.add_enabled_ui(!is_running, |ui| {
                // Group 1: Performance & Direct I/O
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

                    ui.add_space(4.0);
                    ui.checkbox(
                        &mut config.direct_io,
                        RichText::new("Direct I/O em arquivos grandes (>64MB - contorna cache de RAM e maximiza barramento USB/NVMe)").size(14.0),
                    );
                });

                ui.add_space(10.0);

                // Group 2: Overwrite Policy
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

                ui.add_space(10.0);

                // Group 3: Post Copy Actions
                ui.group(|ui| {
                    ui.label(RichText::new("🔔 Ações ao Concluir a Transferência").strong().size(16.0));
                    ui.add_space(6.0);

                    for action in PostCopyAction::all() {
                        ui.radio_value(
                            &mut config.post_action,
                            action,
                            RichText::new(action.label()).size(14.0),
                        );
                    }
                });

                ui.add_space(10.0);

                // Group 4: Integrity & Metadata
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

                ui.add_space(10.0);

                // Group 5: Windows Context Menu Integration
                ui.group(|ui| {
                    ui.label(RichText::new("🖱 Integração com Windows Explorer (Menu de Contexto)").strong().size(16.0));
                    ui.add_space(6.0);

                    let is_registered = is_context_menu_registered();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Status no Windows Explorer:").size(14.0));
                        if is_registered {
                            ui.label(RichText::new("✔ Instalado (Ativo)").color(COLOR_SUCCESS).strong().size(14.0));
                        } else {
                            ui.label(RichText::new("✖ Não instalado").color(Color32::from_rgb(148, 163, 184)).size(14.0));
                        }
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if !is_registered {
                            if ui.button(RichText::new("➕ Adicionar 'Copiar com CopyFile' ao Menu de Contexto").strong().size(14.0)).clicked() {
                                let _ = register_context_menu();
                            }
                        } else if ui.button(RichText::new("➖ Remover do Menu de Contexto").color(COLOR_ERROR).size(14.0)).clicked() {
                            let _ = unregister_context_menu();
                        }
                    });
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
                    .color(Color32::from_rgb(148, 163, 184)),
                );
            }

            ui.add_space(20.0);
        });
}
