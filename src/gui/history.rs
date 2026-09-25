use eframe::egui::{self, Color32, RichText, Ui};
use rusqlite::Connection;
use std::path::Path;

use crate::database::connection::open_optimized_connection;
use crate::database::models::LatencySampleRecord;
use crate::database::repository::get_all_hosts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFilter {
    Last5Minutes,
    Last15Minutes,
    LastHour,
    Last6Hours,
    Last24Hours,
    Last7Days,
    All,
}

impl TimeFilter {
    pub fn sql_modifier(&self) -> Option<&'static str> {
        match self {
            TimeFilter::Last5Minutes => Some("-5 minutes"),
            TimeFilter::Last15Minutes => Some("-15 minutes"),
            TimeFilter::LastHour => Some("-1 hours"),
            TimeFilter::Last6Hours => Some("-6 hours"),
            TimeFilter::Last24Hours => Some("-24 hours"),
            TimeFilter::Last7Days => Some("-7 days"),
            TimeFilter::All => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TimeFilter::Last5Minutes => "Últimos 5 minutos",
            TimeFilter::Last15Minutes => "Últimos 15 minutos",
            TimeFilter::LastHour => "Última hora",
            TimeFilter::Last6Hours => "Últimas 6 horas",
            TimeFilter::Last24Hours => "Últimas 24 horas",
            TimeFilter::Last7Days => "Últimos 7 dias",
            TimeFilter::All => "Todo o histórico",
        }
    }
}

pub struct HistoryState {
    pub selected_filter: TimeFilter,
    pub selected_host_id: Option<i64>,
    pub cached_samples: Vec<LatencySampleRecord>,
    pub last_query_time: Option<std::time::Instant>,
}

impl Default for HistoryState {
    fn default() -> Self {
        Self {
            selected_filter: TimeFilter::Last15Minutes,
            selected_host_id: None,
            cached_samples: Vec::new(),
            last_query_time: None,
        }
    }
}

pub fn render_history(ui: &mut Ui, state: &mut HistoryState, db_path: &Path) {
    ui.heading(RichText::new("Histórico de Latência").strong());
    ui.separator();

    let conn = match open_optimized_connection(db_path) {
        Ok(c) => c,
        Err(e) => {
            ui.label(format!("Erro ao acessar banco de dados: {}", e));
            return;
        }
    };

    let hosts = get_all_hosts(&conn).unwrap_or_default();
    if state.selected_host_id.is_none() && !hosts.is_empty() {
        state.selected_host_id = Some(hosts[0].id);
    }

    // Controles de Filtro
    let mut need_refresh = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Host:").strong());
        let current_host_name = hosts
            .iter()
            .find(|h| Some(h.id) == state.selected_host_id)
            .map(|h| h.name.as_str())
            .unwrap_or("Selecione");

        egui::ComboBox::from_id_salt("history_host_combo")
            .selected_text(current_host_name)
            .show_ui(ui, |ui| {
                for host in &hosts {
                    let is_selected = state.selected_host_id == Some(host.id);
                    if ui.selectable_label(is_selected, &host.name).clicked() {
                        state.selected_host_id = Some(host.id);
                        need_refresh = true;
                    }
                }
            });

        ui.add_space(16.0);
        ui.label(RichText::new("Período:").strong());
        egui::ComboBox::from_id_salt("history_time_combo")
            .selected_text(state.selected_filter.label())
            .show_ui(ui, |ui| {
                let filters = [
                    TimeFilter::Last5Minutes,
                    TimeFilter::Last15Minutes,
                    TimeFilter::LastHour,
                    TimeFilter::Last6Hours,
                    TimeFilter::Last24Hours,
                    TimeFilter::Last7Days,
                    TimeFilter::All,
                ];
                for f in filters {
                    let is_selected = state.selected_filter == f;
                    if ui.selectable_label(is_selected, f.label()).clicked() {
                        state.selected_filter = f;
                        need_refresh = true;
                    }
                }
            });

        if ui.button("🔄 Atualizar").clicked() {
            need_refresh = true;
        }
    });

    // Atualização da consulta
    if let (true, Some(host_id)) = (
        need_refresh || state.last_query_time.is_none(),
        state.selected_host_id,
    ) {
        state.cached_samples =
            query_samples(&conn, host_id, state.selected_filter).unwrap_or_default();
        state.last_query_time = Some(std::time::Instant::now());
    }

    ui.add_space(8.0);

    // Resumo Estatístico do Período
    let total_samples = state.cached_samples.len();
    let success_samples = state.cached_samples.iter().filter(|s| s.success).count();
    let lost_samples = total_samples - success_samples;
    let loss_pct = if total_samples > 0 {
        (lost_samples as f64 / total_samples as f64) * 100.0
    } else {
        0.0
    };

    let (min_rtt, max_rtt, avg_rtt) = if success_samples > 0 {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut sum = 0.0;
        for s in state.cached_samples.iter().filter(|s| s.success) {
            min = min.min(s.latency_ms);
            max = max.max(s.latency_ms);
            sum += s.latency_ms;
        }
        (min, max, sum / success_samples as f64)
    } else {
        (0.0, 0.0, 0.0)
    };

    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            egui::Grid::new("history_summary_grid")
                .num_columns(5)
                .spacing([28.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Amostras:").strong());
                    ui.label(RichText::new("Latência Média:").strong());
                    ui.label(RichText::new("Menor Latência:").strong());
                    ui.label(RichText::new("Maior Latência:").strong());
                    ui.label(RichText::new("Perda de Pacotes:").strong());
                    ui.end_row();

                    ui.label(format!("{} registradas", total_samples));
                    ui.label(format!("{:.1} ms", avg_rtt));
                    ui.label(format!("{:.0} ms", min_rtt));
                    ui.label(format!("{:.0} ms", max_rtt));
                    let loss_color = if loss_pct > 0.0 {
                        Color32::from_rgb(231, 76, 60)
                    } else {
                        Color32::from_rgb(46, 204, 113)
                    };
                    ui.label(
                        RichText::new(format!("{:.1}% ({} perdidos)", loss_pct, lost_samples))
                            .color(loss_color),
                    );
                    ui.end_row();
                });
        });

    ui.add_space(8.0);

    // Tabela de Amostras
    ui.heading(RichText::new("Registros de Amostras").strong());

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("history_samples_table")
                .striped(true)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Data / Hora (UTC)").strong());
                    ui.label(RichText::new("Latência").strong());
                    ui.label(RichText::new("Status").strong());
                    ui.label(RichText::new("Detalhe / Erro").strong());
                    ui.end_row();

                    for sample in state.cached_samples.iter().rev().take(200) {
                        ui.label(&sample.timestamp);
                        if sample.success {
                            ui.label(format!("{:.1} ms", sample.latency_ms));
                            ui.label(
                                RichText::new("SUCCESS").color(Color32::from_rgb(46, 204, 113)),
                            );
                        } else {
                            ui.label(RichText::new("-").weak());
                            ui.label(
                                RichText::new("TIMEOUT / ERRO")
                                    .color(Color32::from_rgb(231, 76, 60))
                                    .strong(),
                            );
                        }
                        ui.label(sample.error_type.as_deref().unwrap_or("-"));
                        ui.end_row();
                    }
                });
        });
}

fn query_samples(
    conn: &Connection,
    host_id: i64,
    filter: TimeFilter,
) -> Result<Vec<LatencySampleRecord>, rusqlite::Error> {
    let sql = if let Some(modifier) = filter.sql_modifier() {
        format!(
            "SELECT id, host_id, timestamp, latency_ms, success, error_type
             FROM latency_samples
             WHERE host_id = ?1 AND timestamp >= datetime('now', '{}')
             ORDER BY timestamp ASC",
            modifier
        )
    } else {
        "SELECT id, host_id, timestamp, latency_ms, success, error_type
         FROM latency_samples
         WHERE host_id = ?1
         ORDER BY timestamp ASC"
            .to_string()
    };

    let mut stmt = conn.prepare(&sql)?;
    let iter = stmt.query_map([host_id], |row| {
        Ok(LatencySampleRecord {
            id: Some(row.get(0)?),
            host_id: row.get(1)?,
            timestamp: row.get(2)?,
            latency_ms: row.get(3)?,
            success: row.get::<_, i32>(4)? != 0,
            error_type: row.get(5)?,
        })
    })?;

    let mut samples = Vec::new();
    for sample in iter {
        samples.push(sample?);
    }
    Ok(samples)
}
