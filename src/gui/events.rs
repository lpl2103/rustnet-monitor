use eframe::egui::{self, Color32, RichText, Ui};
use rusqlite::Connection;
use std::path::Path;

use crate::database::connection::open_optimized_connection;
use crate::database::models::NetworkEventRecord;

#[derive(Default)]
pub struct EventsState {
    pub cached_events: Vec<NetworkEventRecord>,
    pub last_refresh: Option<std::time::Instant>,
}

pub fn render_events(ui: &mut Ui, state: &mut EventsState, db_path: &Path) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Eventos de Rede").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🔄 Atualizar").clicked() {
                state.last_refresh = None;
            }
        });
    });

    ui.separator();

    if state.last_refresh.is_none() {
        let conn_res = open_optimized_connection(db_path);
        if let Ok(conn) = conn_res {
            state.cached_events = query_network_events(&conn).unwrap_or_default();
            state.last_refresh = Some(std::time::Instant::now());
        }
    }

    if state.cached_events.is_empty() {
        ui.label("Nenhum evento de rede registrado até o momento.");
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("events_table")
                .striped(true)
                .spacing([24.0, 8.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Data / Hora (UTC)").strong());
                    ui.label(RichText::new("Evento").strong());
                    ui.label(RichText::new("Interface").strong());
                    ui.label(RichText::new("Tipo").strong());
                    ui.label(RichText::new("Detalhes").strong());
                    ui.end_row();

                    for event in state.cached_events.iter().rev() {
                        ui.label(&event.timestamp);

                        let (badge_color, badge_text) = match event.event_type.as_str() {
                            "CONNECTED" | "LINK_UP" => {
                                (Color32::from_rgb(46, 204, 113), "● CONNECTED")
                            }
                            "DISCONNECTED" | "LINK_DOWN" => {
                                (Color32::from_rgb(231, 76, 60), "● DISCONNECTED")
                            }
                            "IP_CHANGED" => (Color32::from_rgb(52, 152, 219), "● IP CHANGED"),
                            "GATEWAY_CHANGED" => {
                                (Color32::from_rgb(241, 196, 15), "● GATEWAY CHANGED")
                            }
                            _ => (Color32::GRAY, event.event_type.as_str()),
                        };

                        ui.label(RichText::new(badge_text).color(badge_color).strong());
                        ui.label(&event.interface_name);
                        ui.label(&event.interface_type);
                        ui.label(event.details.as_deref().unwrap_or("-"));
                        ui.end_row();
                    }
                });
        });
}

fn query_network_events(conn: &Connection) -> Result<Vec<NetworkEventRecord>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, interface_name, interface_type, event_type, details
         FROM network_events
         ORDER BY id ASC",
    )?;

    let iter = stmt.query_map([], |row| {
        Ok(NetworkEventRecord {
            id: Some(row.get(0)?),
            timestamp: row.get(1)?,
            interface_name: row.get(2)?,
            interface_type: row.get(3)?,
            event_type: row.get(4)?,
            details: row.get(5)?,
        })
    })?;

    let mut events = Vec::new();
    for event in iter {
        events.push(event?);
    }
    Ok(events)
}
