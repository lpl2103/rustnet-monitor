use eframe::egui::{self, Color32, RichText};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::config::AppConfig;
use crate::database::DatabaseHandle;
use crate::gui::charts::render_charts;
use crate::gui::dashboard::render_dashboard;
use crate::gui::events::{EventsState, render_events};
use crate::gui::history::{HistoryState, render_history};
use crate::gui::settings::{SettingsState, render_settings};
use crate::network::icmp::PingStatus;
use crate::network::pinger::{PingUpdate, PingerService};
use crate::network::stats::HostStats;
use crate::network::types::NetworkDiagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard,
    Charts,
    History,
    Events,
    Settings,
}

pub struct RustNetApp {
    config: AppConfig,
    config_path: PathBuf,
    diagnostic: NetworkDiagnostic,
    active_tab: ActiveTab,
    stats_list: Vec<HostStats>,
    history_points: HashMap<String, Vec<(f64, f64)>>, // Host -> [(segundos_decorridos, rtt_ms)]
    active_hosts_in_chart: HashMap<String, bool>,
    start_time: Instant,
    last_update_time: Option<Instant>,
    rx_updates: Option<Receiver<PingUpdate>>,
    stop_signal: Option<Arc<AtomicBool>>,
    db_handle: Option<DatabaseHandle>,
    history_state: HistoryState,
    events_state: EventsState,
    settings_state: SettingsState,
}

impl RustNetApp {
    pub fn new(
        config: AppConfig,
        config_path: PathBuf,
        diagnostic: NetworkDiagnostic,
        db_handle: Option<DatabaseHandle>,
    ) -> Self {
        let detected_gateway = diagnostic
            .default_route_ipv4
            .as_ref()
            .and_then(|r| Ipv4Addr::from_str(&r.next_hop).ok())
            .or_else(|| {
                diagnostic
                    .active_physical_interface
                    .as_ref()
                    .and_then(|iface| iface.gateway_addresses.first())
                    .and_then(|gw| Ipv4Addr::from_str(gw).ok())
            });

        let pinger = PingerService::new(config.clone(), detected_gateway);
        let initial_stats = pinger.current_stats();
        let (rx, stop_signal, _worker) = pinger.start_worker();

        let mut history_points = HashMap::new();
        let mut active_hosts_in_chart = HashMap::new();
        for stat in &initial_stats {
            history_points.insert(stat.name.clone(), Vec::new());
            active_hosts_in_chart.insert(stat.name.clone(), true);
        }

        Self {
            config,
            config_path,
            diagnostic,
            active_tab: ActiveTab::Dashboard,
            stats_list: initial_stats,
            history_points,
            active_hosts_in_chart,
            start_time: Instant::now(),
            last_update_time: None,
            rx_updates: Some(rx),
            stop_signal: Some(stop_signal),
            db_handle,
            history_state: HistoryState::default(),
            events_state: EventsState::default(),
            settings_state: SettingsState::default(),
        }
    }
}

impl Drop for RustNetApp {
    fn drop(&mut self) {
        if let Some(ref stop) = self.stop_signal {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(ref db) = self.db_handle {
            db.stop();
        }
    }
}

impl eframe::App for RustNetApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 1. Processa amostras assíncronas recebidas do pinger worker
        if let Some(ref rx) = self.rx_updates {
            while let Ok(update) = rx.try_recv() {
                self.last_update_time = Some(Instant::now());

                // Atualiza ou insere nas estatísticas em memória
                if let Some(existing) = self
                    .stats_list
                    .iter_mut()
                    .find(|s| s.name == update.host_name)
                {
                    *existing = update.stats.clone();
                } else {
                    self.stats_list.push(update.stats.clone());
                }

                // Adiciona ponto na série temporal do gráfico
                let elapsed_secs = self.start_time.elapsed().as_secs_f64();
                let points = self
                    .history_points
                    .entry(update.host_name.clone())
                    .or_default();
                points.push((elapsed_secs, update.result.rtt_ms));
                // Mantém até 300 amostras na memória para o gráfico em tempo real
                if points.len() > 300 {
                    points.remove(0);
                }

                // Persiste amostra no SQLite via Database Worker
                if let Some(ref db) = self.db_handle {
                    let addr = update
                        .stats
                        .resolved_ip
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|| update.stats.target_str.clone());

                    let host_type = if update.host_name.to_lowercase().contains("gateway") {
                        "gateway"
                    } else if update.host_name.to_lowercase().contains("dns") {
                        "dns"
                    } else {
                        "custom"
                    };

                    db.record_sample(
                        update.host_name,
                        addr,
                        host_type.to_string(),
                        update.result.rtt_ms,
                        update.result.status == PingStatus::Success,
                        update.result.error_message,
                    );
                }
            }
        }

        // 2. Aplica tema configurado (Dark / Light / System)
        match self.config.general.theme.as_str() {
            "dark" => ctx.set_visuals(egui::Visuals::dark()),
            "light" => ctx.set_visuals(egui::Visuals::light()),
            _ => {} // Mantém padrão do sistema
        }

        // 3. Barra Superior com Navegação em Abas
        egui::Panel::top("top_navigation_bar").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("🦀 RustNet Monitor")
                        .size(18.0)
                        .strong()
                        .color(Color32::from_rgb(230, 126, 34)),
                );

                ui.add_space(16.0);

                let tabs = [
                    (ActiveTab::Dashboard, "📊 Dashboard"),
                    (ActiveTab::Charts, "📈 Gráficos"),
                    (ActiveTab::History, "🕒 Histórico"),
                    (ActiveTab::Events, "🔔 Eventos"),
                    (ActiveTab::Settings, "⚙ Configurações"),
                ];

                for (tab, label) in tabs {
                    let is_active = self.active_tab == tab;
                    if ui
                        .selectable_label(is_active, RichText::new(label).strong())
                        .clicked()
                    {
                        self.active_tab = tab;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(last) = self.last_update_time {
                        let ago = last.elapsed().as_secs();
                        ui.label(RichText::new(format!("Atualizado há {}s", ago)).weak());
                    } else {
                        ui.label(RichText::new("Aguardando amostras...").weak());
                    }
                });
            });
            ui.add_space(4.0);
        });

        // 4. Barra Inferior de Status
        egui::Panel::bottom("bottom_status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(iface) = &self.diagnostic.active_physical_interface {
                    ui.label(RichText::new(format!("Conexão: {}", iface.friendly_name)).weak());
                    ui.separator();
                    ui.label(RichText::new(format!("Link: {}", iface.formatted_speed())).weak());
                    ui.separator();
                    ui.label(
                        RichText::new(format!("Hosts Ativos: {}", self.stats_list.len())).weak(),
                    );
                } else {
                    ui.label(
                        RichText::new("Conexão Offline").color(Color32::from_rgb(231, 76, 60)),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("RustNet Monitor v0.1.0").weak());
                });
            });
        });

        // 5. Painel Central (Conteúdo da Aba Ativa)
        egui::CentralPanel::default().show(ui, |ui| match self.active_tab {
            ActiveTab::Dashboard => {
                render_dashboard(
                    ui,
                    &self.diagnostic,
                    &self.stats_list,
                    &self.config.thresholds,
                );
            }
            ActiveTab::Charts => {
                render_charts(ui, &self.history_points, &mut self.active_hosts_in_chart);
            }
            ActiveTab::History => {
                let db_path = PathBuf::from(&self.config.database.path);
                render_history(ui, &mut self.history_state, &db_path);
            }
            ActiveTab::Events => {
                let db_path = PathBuf::from(&self.config.database.path);
                render_events(ui, &mut self.events_state, &db_path);
            }
            ActiveTab::Settings => {
                render_settings(
                    ui,
                    &mut self.config,
                    &mut self.settings_state,
                    &self.config_path,
                );
            }
        });

        // Requisita repaint a cada 500ms para manter animações e gráficos suaves sem consumo excessivo de CPU
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
