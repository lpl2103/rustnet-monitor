use eframe::egui::{self, Color32, RichText, Ui};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::config::settings::{AppConfig, HostConfig};
use crate::updater::{RemoteVersionInfo, UpdateStatus};

pub struct SettingsState {
    pub new_host_name: String,
    pub new_host_address: String,
    #[allow(dead_code)]
    pub new_host_type: String,
    pub status_message: Option<(String, bool)>, // (mensagem, is_error)
    pub show_reset_confirmation: bool,
    pub clear_db_history: bool,
    pub request_reset_metrics: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            new_host_name: String::new(),
            new_host_address: String::new(),
            new_host_type: "custom".to_string(),
            status_message: None,
            show_reset_confirmation: false,
            clear_db_history: true,
            request_reset_metrics: false,
        }
    }
}

pub fn render_settings(
    ui: &mut Ui,
    config: &mut AppConfig,
    state: &mut SettingsState,
    config_path: &Path,
    available_update: &Arc<Mutex<Option<RemoteVersionInfo>>>,
    update_status: &Arc<Mutex<UpdateStatus>>,
) {
    ui.heading(RichText::new("Configurações do RustNet Monitor").strong());
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // 1. Configurações Gerais e Aparência
            ui.group(|ui| {
                ui.heading(RichText::new("Geral e Aparência").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Tema Visual:").strong());
                    ui.radio_value(&mut config.general.theme, "dark".to_string(), "🌙 Escuro");
                    ui.radio_value(&mut config.general.theme, "light".to_string(), "☀️ Claro");
                    ui.radio_value(
                        &mut config.general.theme,
                        "system".to_string(),
                        "💻 Seguir Sistema",
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut config.general.minimize_to_tray,
                        "Minimizar para a Bandeja do Sistema",
                    );
                    ui.checkbox(
                        &mut config.general.start_with_windows,
                        "Iniciar com o Windows",
                    );
                    ui.checkbox(
                        &mut config.general.sound_alerts,
                        "🔊 Alertas Sonoros em Quedas / Perdas",
                    );
                });
            });

            ui.add_space(8.0);

            // 2. Parâmetros de Monitoramento
            ui.group(|ui| {
                ui.heading(RichText::new("Parâmetros de Monitoramento").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Intervalo entre testes:").strong());
                    let intervals = [1, 2, 5, 10, 30, 60];
                    for sec in intervals {
                        ui.radio_value(
                            &mut config.monitoring.interval_secs,
                            sec,
                            format!("{}s", sec),
                        );
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Timeout do Ping:").strong());
                    let timeouts = [500, 1000, 2000, 3000];
                    for ms in timeouts {
                        ui.radio_value(&mut config.monitoring.timeout_ms, ms, format!("{} ms", ms));
                    }
                });
            });

            ui.add_space(8.0);

            // 3. Limites de Classificação de Latência
            ui.group(|ui| {
                ui.heading(RichText::new("Limites de Qualidade de Latência").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Limite Verde (BOM):")
                            .color(Color32::from_rgb(46, 204, 113))
                            .strong(),
                    );
                    ui.add(
                        egui::DragValue::new(&mut config.thresholds.green_max_ms)
                            .range(5..=200)
                            .suffix(" ms"),
                    );

                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("Limite Amarelo (MÉDIO):")
                            .color(Color32::from_rgb(241, 196, 15))
                            .strong(),
                    );
                    ui.add(
                        egui::DragValue::new(&mut config.thresholds.yellow_max_ms)
                            .range(20..=500)
                            .suffix(" ms"),
                    );
                });
                ui.label(
                    RichText::new(
                        "Latências acima do limite amarelo são classificadas como ALTO (Vermelho).",
                    )
                    .weak(),
                );
            });

            ui.add_space(8.0);

            // 4. Retenção do Banco de Dados
            ui.group(|ui| {
                ui.heading(RichText::new("Persistência e Retenção").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Retenção de histórico:").strong());
                    let retentions = [
                        (7, "7 dias"),
                        (30, "30 dias"),
                        (90, "90 dias"),
                        (365, "1 ano"),
                        (0, "Sem limite"),
                    ];
                    for (days, label) in retentions {
                        ui.radio_value(&mut config.database.retention_days, days, label);
                    }
                });
            });

            ui.add_space(8.0);

            // 5. Gerenciamento de Hosts Monitorados
            ui.group(|ui| {
                ui.heading(RichText::new("Gerenciamento de Hosts Monitorados").strong());
                ui.add_space(4.0);

                let mut to_remove = None;
                for (i, host) in config.hosts.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut host.enabled, "");
                        ui.label(RichText::new(&host.name).strong());
                        ui.label(format!("({})", host.address));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !host.is_dynamic_gateway {
                                if ui
                                    .button(
                                        RichText::new("Remover")
                                            .color(Color32::from_rgb(231, 76, 60)),
                                    )
                                    .clicked()
                                {
                                    to_remove = Some(i);
                                }
                            } else {
                                ui.label(RichText::new("Padrão Dinâmico").weak());
                            }
                        });
                    });
                }

                if let Some(idx) = to_remove {
                    config.hosts.remove(idx);
                }

                ui.separator();
                ui.label(RichText::new("Adicionar Novo Host:").strong());
                ui.horizontal(|ui| {
                    ui.label("Nome:");
                    ui.text_edit_singleline(&mut state.new_host_name);
                    ui.label("Endereço / IP:");
                    ui.text_edit_singleline(&mut state.new_host_address);

                    if ui.button("➕ Adicionar").clicked() {
                        let name = state.new_host_name.trim();
                        let addr = state.new_host_address.trim();
                        if !name.is_empty() && !addr.is_empty() {
                            config.hosts.push(HostConfig {
                                name: name.to_string(),
                                address: addr.to_string(),
                                host_type: "custom".to_string(),
                                enabled: true,
                                is_dynamic_gateway: false,
                            });
                            state.new_host_name.clear();
                            state.new_host_address.clear();
                            state.status_message =
                                Some(("Host adicionado com sucesso!".to_string(), false));
                        } else {
                            state.status_message =
                                Some(("Preencha o nome e o endereço do host.".to_string(), true));
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // 6. Manutenção de Dados e Reinicialização de Métricas
            ui.group(|ui| {
                ui.heading(RichText::new("Manutenção e Métricas").strong());
                ui.add_space(4.0);

                if !state.show_reset_confirmation {
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                RichText::new("🔄 Zerar Todas as Métricas")
                                    .color(Color32::from_rgb(231, 76, 60))
                                    .strong(),
                            )
                            .clicked()
                        {
                            state.show_reset_confirmation = true;
                        }

                        ui.label(
                            RichText::new(
                                "Reinicia contadores de latência, jitter, taxas de perda e dados em tempo real.",
                            )
                            .weak(),
                        );
                    });
                } else {
                    egui::Frame::group(ui.style())
                        .fill(Color32::from_rgba_unmultiplied(231, 76, 60, 20))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(231, 76, 60)))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("⚠️").size(24.0));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new("Confirmar Reinicialização de Métricas")
                                            .size(16.0)
                                            .color(Color32::from_rgb(231, 76, 60))
                                            .strong(),
                                    );
                                    ui.label(
                                        "Deseja realmente zerar todos os contadores de latência, jitter e perda de pacotes?",
                                    );
                                    ui.label(
                                        "Esta operação reiniciará o monitoramento em tempo real a partir do zero.",
                                    );
                                    ui.add_space(4.0);
                                    ui.checkbox(
                                        &mut state.clear_db_history,
                                        "Também excluir histórico de amostras persistido no SQLite (rustnet.db)",
                                    );
                                });
                            });

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("⚠️ Sim, Zerar Métricas")
                                                .color(Color32::WHITE)
                                                .strong(),
                                        )
                                        .fill(Color32::from_rgb(192, 57, 43)),
                                    )
                                    .clicked()
                                {
                                    state.request_reset_metrics = true;
                                    state.show_reset_confirmation = false;
                                }

                                if ui.button("Cancelar").clicked() {
                                    state.show_reset_confirmation = false;
                                }
                            });
                        });
                }
            });

            ui.add_space(8.0);

            // 7. Atualização Automática do Aplicativo
            ui.group(|ui| {
                ui.heading(RichText::new("Atualizações do Aplicativo").strong());
                ui.add_space(4.0);

                let current_version = env!("CARGO_PKG_VERSION");
                let status = update_status
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or(crate::updater::UpdateStatus::Idle);
                let opt_info = available_update
                    .lock()
                    .map(|u| u.clone())
                    .unwrap_or(None);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Versão Instalada: v{}", current_version)).strong());
                    ui.label(
                        RichText::new(format!("(Repositório: {})", crate::updater::GITHUB_REPO))
                            .weak(),
                    );
                });

                ui.add_space(4.0);

                match &status {
                    crate::updater::UpdateStatus::Idle => {
                        if opt_info.is_none()
                            && ui
                                .button(RichText::new("🔍 Verificar Atualizações").strong())
                                .clicked()
                        {
                            let upd_clone = available_update.clone();
                            let status_clone = update_status.clone();
                            std::thread::spawn(move || {
                                if let Ok(mut st) = status_clone.lock() {
                                    *st = crate::updater::UpdateStatus::Checking;
                                }
                                if let Some(info) = crate::updater::check_for_updates() {
                                    if let Ok(mut u) = upd_clone.lock() {
                                        *u = Some(info);
                                    }
                                    if let Ok(mut st) = status_clone.lock() {
                                        *st = crate::updater::UpdateStatus::Idle;
                                    }
                                } else if let Ok(mut st) = status_clone.lock() {
                                    *st = crate::updater::UpdateStatus::UpToDate;
                                }
                            });
                        }
                    }
                    crate::updater::UpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new("Verificando se há nova versão no GitHub Releases...")
                                    .color(Color32::from_rgb(52, 152, 219)),
                            );
                        });
                    }
                    crate::updater::UpdateStatus::UpToDate => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("✅ Você já está utilizando a versão mais recente!")
                                    .color(Color32::from_rgb(46, 204, 113))
                                    .strong(),
                            );
                            if ui.button("🔍 Checar Novamente").clicked() {
                                let upd_clone = available_update.clone();
                                let status_clone = update_status.clone();
                                std::thread::spawn(move || {
                                    if let Ok(mut st) = status_clone.lock() {
                                        *st = crate::updater::UpdateStatus::Checking;
                                    }
                                    if let Some(info) = crate::updater::check_for_updates() {
                                        if let Ok(mut u) = upd_clone.lock() {
                                            *u = Some(info);
                                        }
                                        if let Ok(mut st) = status_clone.lock() {
                                            *st = crate::updater::UpdateStatus::Idle;
                                        }
                                    } else {
                                        if let Ok(mut st) = status_clone.lock() {
                                            *st = crate::updater::UpdateStatus::UpToDate;
                                        }
                                    }
                                });
                            }
                        });
                    }
                    crate::updater::UpdateStatus::Downloading(p) => {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Baixando atualização... {:.0}%",
                                    p * 100.0
                                ))
                                .strong(),
                            );
                            ui.add(egui::ProgressBar::new(*p).show_percentage());
                        });
                    }
                    crate::updater::UpdateStatus::Success(msg) => {
                        ui.label(
                            RichText::new(format!("🎉 {}", msg))
                                .color(Color32::from_rgb(46, 204, 113))
                                .strong(),
                        );
                    }
                    crate::updater::UpdateStatus::Error(err) => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("❌ {}", err))
                                .color(Color32::from_rgb(231, 76, 60))
                                .strong(),
                            );
                            if ui.button("Tentar Novamente").clicked()
                                && let Ok(mut st) = update_status.lock()
                            {
                                *st = crate::updater::UpdateStatus::Idle;
                            }
                        });
                    }
                }

                if let Some(info) = opt_info {
                    ui.add_space(6.0);
                    egui::Frame::group(ui.style())
                        .fill(Color32::from_rgba_unmultiplied(46, 204, 113, 25))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(46, 204, 113)))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🚀").size(24.0));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Nova versão v{} disponível para instalação!",
                                            info.version
                                        ))
                                        .size(16.0)
                                        .color(Color32::from_rgb(46, 204, 113))
                                        .strong(),
                                    );
                                    if !info.release_notes.is_empty() {
                                        ui.label(
                                            RichText::new(format!(
                                                "Notas da versão: {}",
                                                info.release_notes
                                            ))
                                            .weak(),
                                        );
                                    }
                                });
                            });

                            ui.add_space(8.0);
                            if matches!(status, crate::updater::UpdateStatus::Downloading(_)) {
                                // Já está realizando o download
                            } else if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("⬇️ Atualizar e Reiniciar Agora")
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(46, 204, 113)),
                                )
                                .clicked()
                            {
                                let dl_url = info.download_url.clone();
                                let status_cb_clone = update_status.clone();
                                std::thread::spawn(move || {
                                    let status_inner = status_cb_clone.clone();
                                    let result = crate::updater::perform_auto_update(
                                        Some(dl_url),
                                        move |st| {
                                            if let Ok(mut guard) = status_inner.lock() {
                                                *guard = st;
                                            }
                                        },
                                    );
                                    if let Err(e) = result
                                        && let Ok(mut guard) = status_cb_clone.lock()
                                    {
                                        *guard = crate::updater::UpdateStatus::Error(e);
                                    }
                                });
                            }
                        });
                }
            });

            ui.add_space(12.0);

            // Botão de Salvar
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("💾 Salvar Configurações").strong())
                    .clicked()
                {
                    match config.save(config_path) {
                        Ok(()) => {
                            state.status_message = Some((
                                "Configurações salvas em config.toml com sucesso!".to_string(),
                                false,
                            ));
                        }
                        Err(e) => {
                            state.status_message =
                                Some((format!("Erro ao salvar configurações: {}", e), true));
                        }
                    }
                }

                if let Some((msg, is_error)) = &state.status_message {
                    let color = if *is_error {
                        Color32::from_rgb(231, 76, 60)
                    } else {
                        Color32::from_rgb(46, 204, 113)
                    };
                    ui.label(RichText::new(msg).color(color).strong());
                }
            });
        });
}
