//! Options, settings configuration panel, and About developer information.

use crate::core::paste_hook::{is_autostart_registered, set_autostart_registered};
use crate::core::shell::{
    install_to_system_windows, is_context_menu_registered, is_installed_in_windows,
    register_context_menu, unregister_context_menu,
};
use crate::core::types::{CopyConfig, OverwritePolicy, PostCopyAction};
use crate::gui::theme::{COLOR_ACCENT, COLOR_ERROR, COLOR_MUTED, COLOR_SUCCESS, COLOR_WARNING};
use eframe::egui::{self, Color32, CornerRadius, RichText, ScrollArea, Slider, Stroke, Ui};

/// Renders the options / preferences view and the About card.
#[allow(clippy::too_many_arguments)]
pub fn render_options(
    ui: &mut Ui,
    config: &mut CopyConfig,
    is_running: bool,
    available_update: &Option<crate::core::RemoteVersionInfo>,
    update_status: &crate::core::UpdateStatus,
    on_trigger_update: impl FnOnce(),
    on_check_update: impl FnOnce(),
) {
    let mut trigger_update_called = false;
    let mut check_update_called = false;
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
                            ui.label("Threads Paralelas:");
                            ui.add(Slider::new(&mut config.num_threads, 1..=32).text("núcleos"));
                        });
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Buffer por Thread:");
                            ui.add(
                                Slider::new(&mut config.buffer_size_kb, 64..=16384)
                                    .step_by(64.0)
                                    .text("KiB"),
                            );
                        });
                        ui.add_space(4.0);

                        ui.checkbox(
                            &mut config.direct_io,
                            RichText::new("Direct I/O em arquivos grandes (>64MB - ignora cache de RAM)")
                                .size(13.5),
                        );
                    });

                    ui.add_space(10.0);

                    // Group 2: Transfer Mode & Overwrite Policy
                    ui.group(|ui| {
                        ui.label(RichText::new("📋 Regras de Transferência").strong().size(16.0));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Modo Padrão:");
                            egui::ComboBox::from_id_salt("opt_transfer_mode")
                                .selected_text(config.transfer_mode.label())
                                .show_ui(ui, |ui| {
                                    for mode in crate::core::types::TransferMode::all() {
                                        ui.selectable_value(
                                            &mut config.transfer_mode,
                                            mode,
                                            mode.label(),
                                        );
                                    }
                                });
                        });
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Sobrescrita:");
                            egui::ComboBox::from_id_salt("opt_overwrite_policy")
                                .selected_text(config.overwrite_policy.label())
                                .show_ui(ui, |ui| {
                                    for policy in OverwritePolicy::all() {
                                        ui.selectable_value(
                                            &mut config.overwrite_policy,
                                            policy,
                                            policy.label(),
                                        );
                                    }
                                });
                        });
                    });

                    ui.add_space(10.0);

                    // Group 3: Post-Copy Automation Actions
                    ui.group(|ui| {
                        ui.label(RichText::new("⚡ Ações ao Concluir a Transferência").strong().size(16.0));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Ao finalizar o lote:");
                            egui::ComboBox::from_id_salt("opt_post_action")
                                .selected_text(config.post_action.label())
                                .show_ui(ui, |ui| {
                                    for action in PostCopyAction::all() {
                                        ui.selectable_value(
                                            &mut config.post_action,
                                            action,
                                            action.label(),
                                        );
                                    }
                                });
                        });
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

                    // Group 5: Notifications & Audio Chimes
                    ui.group(|ui| {
                        ui.label(RichText::new("🔔 Notificações e Alertas Sonoros").strong().size(16.0));
                        ui.add_space(6.0);

                        ui.checkbox(
                            &mut config.enable_notifications,
                            RichText::new("Exibir notificação nativa do Windows (Toast) ao concluir").size(13.5),
                        );
                        ui.add_space(4.0);
                        ui.checkbox(
                            &mut config.enable_sound,
                            RichText::new("Tocar som de alerta discreto ao finalizar a transferência").size(13.5),
                        );
                    });

                    ui.add_space(10.0);

                    // Group 6: Windows Explorer & System PATH Integration
                    ui.group(|ui| {
                        ui.label(RichText::new("🖱 Integração com Windows Explorer e Sistema").strong().size(16.0));
                        ui.add_space(6.0);

                        let (is_registered, mut is_autostart, in_windows) = {
                            thread_local! {
                                static OPT_CACHE: std::cell::RefCell<(std::time::Instant, bool, bool, bool)> =
                                    std::cell::RefCell::new((std::time::Instant::now() - std::time::Duration::from_secs(100), false, false, false));
                            }
                            OPT_CACHE.with(|c| {
                                let mut cache = c.borrow_mut();
                                if cache.0.elapsed().as_secs() >= 2 {
                                    cache.1 = is_context_menu_registered();
                                    cache.2 = is_autostart_registered();
                                    cache.3 = is_installed_in_windows();
                                    cache.0 = std::time::Instant::now();
                                }
                                (cache.1, cache.2, cache.3)
                            })
                        };

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Menu de Contexto (Copiar & Colar):").size(14.0));
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

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        if ui.checkbox(&mut is_autostart, RichText::new("⚡ Interceptar Ctrl+V no Explorer (estilo TeraCopy)").size(13.5)).on_hover_text("Inicia o assistente de colar automaticamente ao pressionar Ctrl+V dentro de pastas do Windows").changed() {
                            let _ = set_autostart_registered(is_autostart);
                            if is_autostart {
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                                    let exe_path = if crate::core::shell::is_installed_in_windows() {
                                        std::path::PathBuf::from(r"C:\Windows\copyfile.exe")
                                    } else {
                                        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("copyfile.exe"))
                                    };
                                    let _ = std::process::Command::new(exe_path)
                                        .arg("--watch-paste")
                                        .creation_flags(CREATE_NO_WINDOW)
                                        .spawn();
                                }
                            }
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Instalação C:\\Windows\\copyfile.exe:").size(14.0));
                            if in_windows {
                                ui.label(RichText::new("✔ Instalado no PATH").color(COLOR_SUCCESS).strong().size(14.0));
                            } else {
                                ui.label(RichText::new("✖ Não copiado").color(COLOR_MUTED).size(14.0));
                            }
                        });

                        ui.add_space(6.0);
                        if ui.button(RichText::new("📁 Copiar para C:\\Windows\\ (PATH global)").size(13.5)).on_hover_text("Copia o executável para a pasta do Windows para executar 'copyfile' de qualquer lugar").clicked() {
                            let _ = install_to_system_windows();
                        }
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
                            ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).small().color(COLOR_MUTED));
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Ferramenta profissional para backup e cópia ultrarrápida de arquivos com verificação criptográfica e preservação integral de metadados.")
                                .size(13.5)
                                .color(Color32::from_rgb(203, 213, 225)),
                        );

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // --- Card de Atualização Automática ---
                        ui.label(RichText::new("🔄 Atualização do Sistema").strong().size(15.0));
                        ui.add_space(4.0);
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(30, 41, 59))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(71, 85, 105)))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Versão Instalada:").size(13.5));
                                    ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).monospace().strong());
                                });

                                if let Some(info) = available_update {
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Versão no GitHub:").size(13.5));
                                        ui.label(RichText::new(format!("v{}", info.version)).monospace().strong().color(COLOR_SUCCESS));
                                        ui.label(RichText::new("(Nova versão disponível!)").small().color(COLOR_SUCCESS));
                                    });
                                    if !info.release_notes.is_empty() {
                                        ui.add_space(2.0);
                                        ui.label(RichText::new(&info.release_notes).small().italics().color(Color32::from_rgb(148, 163, 184)));
                                    }
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Status:").size(13.5));
                                        ui.label(RichText::new("Você está na versão mais recente.").size(13.5).color(COLOR_SUCCESS));
                                    });
                                }

                                ui.add_space(8.0);
                                match update_status {
                                    crate::core::UpdateStatus::Idle => {
                                        ui.horizontal(|ui| {
                                            if let Some(info) = available_update {
                                                let btn_update = egui::Button::new(
                                                    RichText::new(format!("⬇ Baixar e Atualizar para v{}", info.version))
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(Color32::from_rgb(0, 140, 60));

                                                if ui.add(btn_update).clicked() {
                                                    trigger_update_called = true;
                                                }
                                            }

                                            if ui.button(RichText::new("🔄 Verificar Atualizações").size(13.0)).clicked() {
                                                check_update_called = true;
                                            }
                                        });
                                    }
                                    crate::core::UpdateStatus::Downloading(prog) => {
                                        ui.label(RichText::new(format!("Baixando nova versão... ({:.0}%)", prog * 100.0)).size(13.5));
                                        ui.add(egui::ProgressBar::new(*prog).animate(true));
                                    }
                                    crate::core::UpdateStatus::Success(msg) => {
                                        ui.label(RichText::new(msg).color(COLOR_SUCCESS).strong().size(13.5));
                                    }
                                    crate::core::UpdateStatus::Error(err) => {
                                        ui.label(RichText::new(format!("Erro ao atualizar: {err}")).color(COLOR_ERROR).size(13.0));
                                        ui.add_space(4.0);
                                        if ui.button(RichText::new("🔁 Tentar Novamente").size(13.0)).clicked() {
                                            trigger_update_called = true;
                                        }
                                    }
                                }
                            });

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

            if trigger_update_called {
                on_trigger_update();
            }
            if check_update_called {
                on_check_update();
            }

            ui.add_space(20.0);
        });
}
