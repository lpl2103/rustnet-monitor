//! # Aba de Ferramentas Avançadas de Diagnóstico (`tools.rs`)
//!
//! Reúne utilitários profissionais de engenharia de redes:
//! 1. Visual MTR (Traceroute Hop-by-Hop)
//! 2. Path MTU Discovery (DF ICMP)
//! 3. Benchmark de Servidores DNS
//! 4. Teste de Bufferbloat (Loaded Latency)
//! 5. Emissor de Laudo Técnico e Relatório de SLA

use eframe::egui::{self, Color32, RichText, Ui};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::network::bufferbloat::{BufferbloatResult, run_bufferbloat_test};
use crate::network::dns::{DnsBenchmarkResult, run_dns_benchmark};
use crate::network::mtr::{MtrHop, run_mtr_cycle};
use crate::network::mtu::{MtuTestResult, discover_path_mtu};
use crate::network::stats::HostStats;
use crate::network::types::NetworkDiagnostic;
use crate::network::wan::WanInfo;
use crate::network::wifi::WifiInfo;
use crate::reporting::report::{ReportData, generate_technical_report, save_report_to_file};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsSubTab {
    Mtr,
    Mtu,
    Dns,
    Bufferbloat,
    Report,
}

pub struct ToolsState {
    pub active_subtab: ToolsSubTab,

    // 1. MTR State
    pub mtr_target_str: String,
    pub mtr_hops: Arc<Mutex<Vec<MtrHop>>>,
    pub mtr_running: Arc<AtomicBool>,

    // 2. MTU State
    pub mtu_target_str: String,
    pub mtu_result: Arc<Mutex<Option<MtuTestResult>>>,
    pub mtu_running: Arc<AtomicBool>,

    // 3. DNS State
    pub dns_results: Arc<Mutex<Vec<DnsBenchmarkResult>>>,
    pub dns_running: Arc<AtomicBool>,

    // 4. Bufferbloat State
    pub bufferbloat_target_str: String,
    pub bufferbloat_result: Arc<Mutex<Option<BufferbloatResult>>>,
    pub bufferbloat_progress: Arc<Mutex<(f32, String)>>,
    pub bufferbloat_running: Arc<AtomicBool>,

    // 5. Laudo State
    pub generated_report: String,
    pub report_status_message: Option<(String, bool)>,
}

impl Default for ToolsState {
    fn default() -> Self {
        Self {
            active_subtab: ToolsSubTab::Mtr,
            mtr_target_str: "1.1.1.1".to_string(),
            mtr_hops: Arc::new(Mutex::new(Vec::new())),
            mtr_running: Arc::new(AtomicBool::new(false)),

            mtu_target_str: "1.1.1.1".to_string(),
            mtu_result: Arc::new(Mutex::new(None)),
            mtu_running: Arc::new(AtomicBool::new(false)),

            dns_results: Arc::new(Mutex::new(Vec::new())),
            dns_running: Arc::new(AtomicBool::new(false)),

            bufferbloat_target_str: "1.1.1.1".to_string(),
            bufferbloat_result: Arc::new(Mutex::new(None)),
            bufferbloat_progress: Arc::new(Mutex::new((0.0, "Pronto para iniciar".to_string()))),
            bufferbloat_running: Arc::new(AtomicBool::new(false)),

            generated_report: String::new(),
            report_status_message: None,
        }
    }
}

pub fn render_tools(
    ui: &mut Ui,
    state: &mut ToolsState,
    diagnostic: &NetworkDiagnostic,
    stats_list: &[HostStats],
    wan_info: &Option<WanInfo>,
    wifi_info: &Option<WifiInfo>,
) {
    ui.heading(RichText::new("🛠 Ferramentas Profissionais de Diagnóstico").strong());
    ui.add_space(4.0);

    // Barra de Navegação entre Sub-Ferramentas
    ui.horizontal(|ui| {
        let tabs = [
            (ToolsSubTab::Mtr, "🌐 Visual MTR (Traceroute)"),
            (ToolsSubTab::Mtu, "📦 Path MTU Discovery"),
            (ToolsSubTab::Dns, "🚀 Benchmark DNS"),
            (ToolsSubTab::Bufferbloat, "🌊 Teste Bufferbloat"),
            (ToolsSubTab::Report, "📋 Laudo Técnico SLA"),
        ];

        for (tab, label) in tabs {
            let is_active = state.active_subtab == tab;
            if ui
                .selectable_label(is_active, RichText::new(label).strong())
                .clicked()
            {
                state.active_subtab = tab;
            }
        }
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| match state.active_subtab {
            ToolsSubTab::Mtr => render_mtr_subtab(ui, state),
            ToolsSubTab::Mtu => render_mtu_subtab(ui, state),
            ToolsSubTab::Dns => render_dns_subtab(ui, state, diagnostic),
            ToolsSubTab::Bufferbloat => render_bufferbloat_subtab(ui, state),
            ToolsSubTab::Report => {
                render_report_subtab(ui, state, diagnostic, stats_list, wan_info, wifi_info)
            }
        });
}

fn render_mtr_subtab(ui: &mut Ui, state: &mut ToolsState) {
    ui.group(|ui| {
        ui.heading(RichText::new("Visual MTR - Rastreamento Contínuo Salto a Salto").strong());
        ui.label(
            RichText::new(
                "Mede a latência e a perda de pacotes em cada roteador ao longo da rota até o destino.",
            )
            .weak(),
        );
        ui.add_space(6.0);

        let is_running = state.mtr_running.load(Ordering::Relaxed);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Destino (IPv4):").strong());
            ui.add_enabled(
                !is_running,
                egui::TextEdit::singleline(&mut state.mtr_target_str).desired_width(140.0),
            );

            if !is_running {
                let start_btn = ui.button(
                    RichText::new("▶ Iniciar Rastreamento MTR")
                        .color(Color32::from_rgb(46, 204, 113))
                        .strong(),
                );
                if start_btn.clicked()
                    && let Ok(target) = Ipv4Addr::from_str(state.mtr_target_str.trim())
                {
                    state.mtr_running.store(true, Ordering::Relaxed);
                    let hops_clone = state.mtr_hops.clone();
                    let running_clone = state.mtr_running.clone();

                    // Limpa amostras anteriores
                    if let Ok(mut h) = hops_clone.lock() {
                        h.clear();
                    }

                    std::thread::spawn(move || {
                        while running_clone.load(Ordering::Relaxed) {
                            let mut local_hops = Vec::new();
                            if let Ok(guard) = hops_clone.lock() {
                                local_hops = guard.clone();
                            }

                            run_mtr_cycle(target, &mut local_hops, 30);

                            if let Ok(mut guard) = hops_clone.lock() {
                                *guard = local_hops;
                            }

                            std::thread::sleep(std::time::Duration::from_millis(1000));
                        }
                    });
                }
            } else if ui
                .button(
                    RichText::new("⏹ Parar MTR")
                        .color(Color32::from_rgb(231, 76, 60))
                        .strong(),
                )
                .clicked()
            {
                state.mtr_running.store(false, Ordering::Relaxed);
            }

            if is_running {
                ui.spinner();
                ui.label(
                    RichText::new("Sondando rota continuamente a cada 1s...")
                        .color(Color32::from_rgb(52, 152, 219)),
                );
            }
        });
    });

    ui.add_space(6.0);

    // Tabela de Saltos MTR
    let hops = state
        .mtr_hops
        .lock()
        .map(|h| h.clone())
        .unwrap_or_default();

    if hops.is_empty() {
        ui.label(RichText::new("Clique em 'Iniciar Rastreamento MTR' para mapear a rota.").weak());
    } else {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Salto").strong().monospace());
                ui.separator();
                ui.label(RichText::new(format!("{:<16}", "Endereço IP")).strong().monospace());
                ui.separator();
                ui.label(RichText::new("Enviados").strong().monospace());
                ui.separator();
                ui.label(RichText::new("Perda %").strong().monospace());
                ui.separator();
                ui.label(RichText::new("Atual").strong().monospace());
                ui.separator();
                ui.label(RichText::new("Média").strong().monospace());
                ui.separator();
                ui.label(RichText::new("Mín").strong().monospace());
                ui.separator();
                ui.label(RichText::new("Máx").strong().monospace());
            });
            ui.separator();

            for hop in &hops {
                let ip_str = hop
                    .ip
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "* * * (Sem resposta)".to_string());
                let cur = hop
                    .last_ms
                    .map(|v| format!("{:.0} ms", v))
                    .unwrap_or_else(|| "-".to_string());
                let avg = hop
                    .avg_ms
                    .map(|v| format!("{:.0} ms", v))
                    .unwrap_or_else(|| "-".to_string());
                let min = hop
                    .min_ms
                    .map(|v| format!("{:.0} ms", v))
                    .unwrap_or_else(|| "-".to_string());
                let max = hop
                    .max_ms
                    .map(|v| format!("{:.0} ms", v))
                    .unwrap_or_else(|| "-".to_string());

                let loss_color = if hop.loss_pct == 0.0 {
                    Color32::from_rgb(46, 204, 113)
                } else if hop.loss_pct < 5.0 {
                    Color32::from_rgb(241, 196, 15)
                } else {
                    Color32::from_rgb(231, 76, 60)
                };

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("#{:02}", hop.hop)).monospace().strong());
                    ui.separator();
                    ui.label(RichText::new(format!("{:<16}", ip_str)).monospace());
                    ui.separator();
                    ui.label(RichText::new(format!("{:>4}", hop.sent)).monospace());
                    ui.separator();
                    ui.label(
                        RichText::new(format!("{:>5.1}%", hop.loss_pct))
                            .monospace()
                            .color(loss_color)
                            .strong(),
                    );
                    ui.separator();
                    ui.label(RichText::new(format!("{:>6}", cur)).monospace());
                    ui.separator();
                    ui.label(RichText::new(format!("{:>6}", avg)).monospace());
                    ui.separator();
                    ui.label(RichText::new(format!("{:>6}", min)).monospace());
                    ui.separator();
                    ui.label(RichText::new(format!("{:>6}", max)).monospace());
                });
            }
        });
    }
}

fn render_mtu_subtab(ui: &mut Ui, state: &mut ToolsState) {
    ui.group(|ui| {
        ui.heading(RichText::new("Path MTU Discovery - Detecção de MTU e Black Hole").strong());
        ui.label(
            RichText::new(
                "Envia pacotes ICMP com a flag DF (Don't Fragment) para descobrir o tamanho máximo de quadro que passa pelo enlace sem descarte.",
            )
            .weak(),
        );
        ui.add_space(6.0);

        let is_running = state.mtu_running.load(Ordering::Relaxed);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Destino do Teste:").strong());
            ui.add_enabled(
                !is_running,
                egui::TextEdit::singleline(&mut state.mtu_target_str).desired_width(140.0),
            );

            if !is_running {
                let start_btn = ui.button(
                    RichText::new("🔍 Iniciar Teste de MTU")
                        .color(Color32::from_rgb(52, 152, 219))
                        .strong(),
                );
                if start_btn.clicked()
                    && let Ok(target) = Ipv4Addr::from_str(state.mtu_target_str.trim())
                {
                    state.mtu_running.store(true, Ordering::Relaxed);
                    let result_clone = state.mtu_result.clone();
                    let running_clone = state.mtu_running.clone();

                    std::thread::spawn(move || {
                        let res = discover_path_mtu(target);
                        if let Ok(mut guard) = result_clone.lock() {
                            *guard = Some(res);
                        }
                        running_clone.store(false, Ordering::Relaxed);
                    });
                }
            } else {
                ui.spinner();
                ui.label(
                    RichText::new("Executando busca binária de Path MTU...")
                        .color(Color32::from_rgb(52, 152, 219)),
                );
            }
        });
    });

    ui.add_space(8.0);

    let opt_res = state.mtu_result.lock().ok().and_then(|r| r.clone());
    if let Some(res) = opt_res {
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgba_unmultiplied(52, 152, 219, 20))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(52, 152, 219)))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📦").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "MTU Máximo Suportado: {} bytes ({})",
                                res.max_mtu, res.classification
                            ))
                            .size(17.0)
                            .color(Color32::from_rgb(52, 152, 219))
                            .strong(),
                        );
                        ui.label(RichText::new(&res.details));
                        ui.label(
                            RichText::new(format!(
                                "Sondas enviadas: {} pacotes ICMP com flag DF.",
                                res.packets_tested
                            ))
                            .weak(),
                        );
                    });
                });
            });
    }
}

fn render_dns_subtab(ui: &mut Ui, state: &mut ToolsState, diagnostic: &NetworkDiagnostic) {
    ui.group(|ui| {
        ui.heading(RichText::new("Benchmark e Diagnóstico de Resolução DNS").strong());
        ui.label(
            RichText::new(
                "Compara diretamente a latência de resolução RFC 1035 UDP 53 entre o provedor local e resolvedores globais.",
            )
            .weak(),
        );
        ui.add_space(6.0);

        let is_running = state.dns_running.load(Ordering::Relaxed);

        ui.horizontal(|ui| {
            if !is_running {
                if ui
                    .button(
                        RichText::new("🚀 Executar Benchmark DNS")
                            .color(Color32::from_rgb(46, 204, 113))
                            .strong(),
                    )
                    .clicked()
                {
                    state.dns_running.store(true, Ordering::Relaxed);
                    let results_clone = state.dns_results.clone();
                    let running_clone = state.dns_running.clone();

                    let local_dns = diagnostic
                        .active_physical_interface
                        .as_ref()
                        .and_then(|i| i.dns_addresses.first())
                        .and_then(|d| Ipv4Addr::from_str(d).ok());

                    std::thread::spawn(move || {
                        let res = run_dns_benchmark(local_dns);
                        if let Ok(mut guard) = results_clone.lock() {
                            *guard = res;
                        }
                        running_clone.store(false, Ordering::Relaxed);
                    });
                }
            } else {
                ui.spinner();
                ui.label(
                    RichText::new("Consultando servidores DNS na porta 53 UDP...")
                        .color(Color32::from_rgb(52, 152, 219)),
                );
            }
        });
    });

    ui.add_space(8.0);

    let results = state
        .dns_results
        .lock()
        .map(|r| r.clone())
        .unwrap_or_default();

    if !results.is_empty() {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:<28}", "Servidor DNS")).strong().monospace());
                ui.separator();
                ui.label(RichText::new(format!("{:<15}", "Endereço IP")).strong().monospace());
                ui.separator();
                ui.label(RichText::new("Tempo Resposta").strong().monospace());
                ui.separator();
                ui.label(RichText::new("Status").strong().monospace());
            });
            ui.separator();

            for r in &results {
                let rtt_str = r
                    .rtt_ms
                    .map(|v| format!("{:.1} ms", v))
                    .unwrap_or_else(|| "-".to_string());
                let (status_str, color) = match &r.status {
                    crate::network::dns::DnsStatus::Ok => {
                        ("● OK (Resolvido)", Color32::from_rgb(46, 204, 113))
                    }
                    crate::network::dns::DnsStatus::Timeout => {
                        ("● Timeout (Sem resposta)", Color32::from_rgb(231, 76, 60))
                    }
                    crate::network::dns::DnsStatus::Error(msg) => {
                        (msg.as_str(), Color32::from_rgb(241, 196, 15))
                    }
                };

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{:<28}", r.server_name)).monospace());
                    ui.separator();
                    ui.label(RichText::new(format!("{:<15}", r.server_ip)).monospace());
                    ui.separator();
                    ui.label(RichText::new(format!("{:>14}", rtt_str)).monospace().strong());
                    ui.separator();
                    ui.label(RichText::new(status_str).color(color).monospace());
                });
            }
        });
    }
}

fn render_bufferbloat_subtab(ui: &mut Ui, state: &mut ToolsState) {
    ui.group(|ui| {
        ui.heading(RichText::new("Teste de Bufferbloat - Latência sob Carga").strong());
        ui.label(
            RichText::new(
                "Mede a retenção de pacotes no roteador quando a rede é saturada por downloads simultâneos.",
            )
            .weak(),
        );
        ui.add_space(6.0);

        let is_running = state.bufferbloat_running.load(Ordering::Relaxed);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Alvo do Ping:").strong());
            ui.add_enabled(
                !is_running,
                egui::TextEdit::singleline(&mut state.bufferbloat_target_str).desired_width(140.0),
            );

            if !is_running {
                let start_btn = ui.button(
                    RichText::new("🌊 Executar Teste de Bufferbloat (8s)")
                        .color(Color32::from_rgb(155, 89, 182))
                        .strong(),
                );
                if start_btn.clicked()
                    && let Ok(target) = Ipv4Addr::from_str(state.bufferbloat_target_str.trim())
                {
                    state.bufferbloat_running.store(true, Ordering::Relaxed);
                    let result_clone = state.bufferbloat_result.clone();
                    let progress_clone = state.bufferbloat_progress.clone();
                    let running_clone = state.bufferbloat_running.clone();

                    std::thread::spawn(move || {
                        let prog_cb = progress_clone.clone();
                        let res = run_bufferbloat_test(target, move |p, text| {
                            if let Ok(mut guard) = prog_cb.lock() {
                                *guard = (p, text.to_string());
                            }
                        });

                        if let Ok(mut guard) = result_clone.lock() {
                            *guard = Some(res);
                        }
                        running_clone.store(false, Ordering::Relaxed);
                    });
                }
            } else {
                let (prog, text) = state
                    .bufferbloat_progress
                    .lock()
                    .map(|p| p.clone())
                    .unwrap_or((0.5, "Executando...".to_string()));
                ui.spinner();
                ui.label(RichText::new(text).color(Color32::from_rgb(155, 89, 182)));
                ui.add(egui::ProgressBar::new(prog).show_percentage());
            }
        });
    });

    ui.add_space(8.0);

    let opt_res = state.bufferbloat_result.lock().ok().and_then(|r| r.clone());
    if let Some(res) = opt_res {
        let grade_color = match res.grade {
            "A+" | "A" => Color32::from_rgb(46, 204, 113),
            "B" => Color32::from_rgb(241, 196, 15),
            _ => Color32::from_rgb(231, 76, 60),
        };

        egui::Frame::group(ui.style())
            .fill(Color32::from_rgba_unmultiplied(155, 89, 182, 20))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(155, 89, 182)))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(res.grade).size(36.0).color(grade_color).strong());
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Classificação de Bufferbloat: Nota {} (+{:.1} ms)",
                                res.grade, res.delta_ms
                            ))
                            .size(17.0)
                            .strong(),
                        );
                        ui.label(RichText::new(res.description));
                        ui.label(
                            RichText::new(format!(
                                "Latência Ociosa: {:.1} ms | Latência sob Carga: {:.1} ms",
                                res.unloaded_avg_ms, res.loaded_avg_ms
                            ))
                            .weak(),
                        );
                    });
                });
            });
    }
}

fn render_report_subtab(
    ui: &mut Ui,
    state: &mut ToolsState,
    diagnostic: &NetworkDiagnostic,
    stats_list: &[HostStats],
    wan_info: &Option<WanInfo>,
    wifi_info: &Option<WifiInfo>,
) {
    ui.group(|ui| {
        ui.heading(RichText::new("Emissor de Laudo Técnico e Relatório de SLA").strong());
        ui.label(
            RichText::new(
                "Compila todos os dados da conexão, métricas de estabilidade, dados de rádio Wi-Fi e histórico de incidentes em um laudo formal.",
            )
            .weak(),
        );
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("📝 Gerar Novo Laudo Completo").strong())
                .clicked()
            {
                let data = ReportData {
                    diagnostic,
                    stats_list,
                    wan_info,
                    wifi_info,
                    app_version: env!("CARGO_PKG_VERSION"),
                };
                state.generated_report = generate_technical_report(&data);
                state.report_status_message =
                    Some(("Laudo técnico gerado com sucesso!".to_string(), false));
            }

            if !state.generated_report.is_empty() {
                if ui
                    .button(RichText::new("📋 Copiar para Área de Transferência").strong())
                    .clicked()
                {
                    ui.ctx().copy_text(state.generated_report.clone());
                    state.report_status_message =
                        Some(("Laudo copiado para a Área de Transferência!".to_string(), false));
                }

                if ui
                    .button(RichText::new("💾 Salvar Arquivo (laudo_rede.md)").strong())
                    .clicked()
                {
                    let path = PathBuf::from("laudo_rede.md");
                    match save_report_to_file(&state.generated_report, &path) {
                        Ok(()) => {
                            state.report_status_message = Some((
                                "Arquivo laudo_rede.md salvo com sucesso na pasta do executável!"
                                    .to_string(),
                                false,
                            ));
                        }
                        Err(e) => {
                            state.report_status_message =
                                Some((format!("Erro ao gravar arquivo: {}", e), true));
                        }
                    }
                }
            }

            if let Some((msg, is_error)) = &state.report_status_message {
                let color = if *is_error {
                    Color32::from_rgb(231, 76, 60)
                } else {
                    Color32::from_rgb(46, 204, 113)
                };
                ui.label(RichText::new(msg).color(color).strong());
            }
        });
    });

    ui.add_space(8.0);

    if !state.generated_report.is_empty() {
        ui.label(RichText::new("Pré-visualização do Laudo Técnico:").strong());
        egui::ScrollArea::vertical()
            .max_height(350.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut state.generated_report)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
    }
}
