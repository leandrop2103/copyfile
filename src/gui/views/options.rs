//! Options, settings configuration panel, and About developer information.

use crate::core::shell::{
    is_context_menu_registered, register_context_menu, unregister_context_menu,
};
use crate::core::types::{CopyConfig, OverwritePolicy, PostCopyAction};
use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS, COLOR_WARNING};
use eframe::egui::{self, Color32, CornerRadius, RichText, ScrollArea, Slider, Stroke, Ui};

/// Renders the options / preferences view and the About card.
pub fn render_options(ui: &mut Ui, config: &mut CopyConfig, is_running: bool) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(6.0);

            ui.columns(2, |columns| {
                // ==========================================
                // COLUMN 1 (LEFT): Settings & Preferences
                // ==========================================
                let col_left = &mut columns[0];
                col_left.add_enabled_ui(!is_running, |ui| {
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
                            RichText::new("Direct I/O em arquivos grandes (>64MB - contorna cache de RAM e maximiza barramento USB/NVMe)").size(13.5),
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
                                RichText::new(policy.label()).size(13.5),
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
                                RichText::new(action.label()).size(13.5),
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
                            RichText::new("Verificar integridade com xxHash3 (XXH3 - SIMD 30+ GB/s)").size(13.5),
                        );
                        ui.add_space(4.0);
                        ui.checkbox(
                            &mut config.ignore_errors,
                            RichText::new("Ignorar erros durante a cópia (continua transferindo os demais arquivos)").size(13.5),
                        );
                        ui.add_space(4.0);
                        ui.checkbox(
                            &mut config.preserve_metadata,
                            RichText::new("Preservar metadados completos (Data de Criação, Modificação, Atributos do Windows)").size(13.5),
                        );
                    });

                    ui.add_space(10.0);

                    // Group 5: Windows Context Menu Integration
                    ui.group(|ui| {
                        ui.label(RichText::new("🖱 Integração com Windows Explorer").strong().size(16.0));
                        ui.add_space(6.0);

                        let is_registered = is_context_menu_registered();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Menu de Contexto:").size(14.0));
                            if is_registered {
                                ui.label(RichText::new("✔ Ativo (com ícone)").color(COLOR_SUCCESS).strong().size(14.0));
                            } else {
                                ui.label(RichText::new("✖ Não instalado").color(COLOR_MUTED).size(14.0));
                            }
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if !is_registered {
                                if ui.button(RichText::new("➕ Adicionar ao Menu de Contexto").strong().size(13.5)).clicked() {
                                    let _ = register_context_menu();
                                }
                            } else if ui.button(RichText::new("➖ Remover do Menu de Contexto").color(COLOR_ERROR).size(13.5)).clicked() {
                                let _ = unregister_context_menu();
                            }
                        });
                    });
                });

                if is_running {
                    col_left.add_space(10.0);
                    col_left.label(
                        RichText::new("ℹ As configurações não podem ser alteradas durante uma transferência ativa.")
                            .italics()
                            .size(13.5)
                            .color(COLOR_MUTED),
                    );
                }

                // ==========================================
                // COLUMN 2 (RIGHT): About Developer & System
                // ==========================================
                let col_right = &mut columns[1];
                egui::Frame::NONE
                    .fill(Color32::from_rgb(24, 33, 47))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(16))
                    .show(col_right, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⚡ CopyFile").strong().size(22.0).color(COLOR_ACCENT));
                            ui.label(RichText::new("v0.1.0").small().color(COLOR_MUTED));
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Ferramenta profissional para backup e cópia ultrarrápida de arquivos com verificação criptográfica e preservação integral de metadados.")
                                .size(13.5)
                                .color(Color32::from_rgb(203, 213, 225)),
                        );

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Developer info card
                        ui.label(RichText::new("👨‍💻 Desenvolvido por:").strong().size(15.0));
                        ui.add_space(2.0);
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(30, 41, 59))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(71, 85, 105)))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("👤 Leandro").strong().size(15.0).color(Color32::WHITE));
                                    ui.label(RichText::new("+").size(14.0).color(COLOR_MUTED));
                                    ui.label(RichText::new("✨ Gemini (Google DeepMind)").strong().size(15.0).color(COLOR_WARNING));
                                });
                            });

                        ui.add_space(12.0);
                        ui.label(RichText::new("🚀 Destaques de Arquitetura & Recursos:").strong().size(15.0));
                        ui.add_space(4.0);

                        let features = [
                            ("⚡ Rayon Multi-Threading", "Pipeline de cópia paralela de alta taxa"),
                            ("🛡 xxHash3 (XXH3)", "Hashing vetorizado em SIMD (>30 GB/s)"),
                            ("🚀 Direct I/O", "Bypass de cache de RAM para arquivos gigantes"),
                            ("📁 Preservação de Metadados", "Criação, modificação, acesso e atributos Win32"),
                            ("⚡ Presets de Usuário", "Backup de Desktop, Docs, Imagens e Chrome com 1 clique"),
                            ("⏏ Ações Pós-Cópia", "Desligar PC, suspender ou ejetar HD externo"),
                            ("🖱 Menu de Contexto", "Integração nativa com Windows Explorer com ícone"),
                            ("🎨 GUI em GPU Nativa", "Renderização com egui/eframe (< 25 MB RAM)"),
                        ];

                        for (title, desc) in features {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(title).strong().size(13.0).color(COLOR_ACCENT));
                                ui.label(RichText::new(format!("- {desc}")).size(13.0).color(Color32::from_rgb(148, 163, 184)));
                            });
                            ui.add_space(2.0);
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // System telemetry badge
                        let cpus = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🖥 Sistema:").small().color(COLOR_MUTED));
                            ui.label(RichText::new(format!("Windows x86_64 ({cpus} núcleos lógicos)")).small().strong());
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("💾 Compilação:").small().color(COLOR_MUTED));
                            ui.label(RichText::new("Rust Release + LTO Fat + CRT Estático").small().strong().color(COLOR_SUCCESS));
                        });
                    });
            });

            ui.add_space(20.0);
        });
}
