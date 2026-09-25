use eframe::egui::{self, Color32, RichText, Ui};

use crate::config::settings::LatencyThresholds;
use crate::network::stats::{HostStats, LatencyQuality};
use crate::network::types::NetworkDiagnostic;

/// Renderiza a aba principal do Dashboard.
pub fn render_dashboard(
    ui: &mut Ui,
    diagnostic: &NetworkDiagnostic,
    stats_list: &[HostStats],
    thresholds: &LatencyThresholds,
) {
    ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

    // 1. Cartão de Informações da Interface Ativa
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Interface de Rede Ativa").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(iface) = &diagnostic.active_physical_interface {
                        let (color, text) = match iface.oper_status {
                            crate::network::types::OperStatus::Up => {
                                (Color32::from_rgb(46, 204, 113), "● CONECTADO")
                            }
                            _ => (Color32::from_rgb(231, 76, 60), "● DESCONECTADO"),
                        };
                        ui.label(RichText::new(text).color(color).strong());
                    } else {
                        ui.label(
                            RichText::new("● SEM INTERFACE FÍSICA")
                                .color(Color32::from_rgb(231, 76, 60))
                                .strong(),
                        );
                    }
                });
            });

            ui.separator();

            if let Some(iface) = &diagnostic.active_physical_interface {
                egui::Grid::new("iface_grid")
                    .num_columns(4)
                    .spacing([24.0, 8.0])
                    .striped(false)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Adaptador:").strong());
                        ui.label(&iface.friendly_name);

                        ui.label(RichText::new("Descrição:").strong());
                        ui.label(&iface.description);
                        ui.end_row();

                        ui.label(RichText::new("Tipo:").strong());
                        ui.label(format!("{} (Física)", iface.if_type));

                        ui.label(RichText::new("Velocidade do Link:").strong());
                        ui.label(iface.formatted_speed());
                        ui.end_row();

                        ui.label(RichText::new("Endereço IPv4:").strong());
                        ui.label(if iface.ipv4_addresses.is_empty() {
                            "Não atribuído".to_string()
                        } else {
                            iface.ipv4_addresses.join(", ")
                        });

                        ui.label(RichText::new("Gateway Padrão:").strong());
                        ui.label(if iface.gateway_addresses.is_empty() {
                            "Nenhum".to_string()
                        } else {
                            iface.gateway_addresses.join(", ")
                        });
                        ui.end_row();

                        ui.label(RichText::new("Endereço MAC:").strong());
                        ui.label(iface.mac_address.as_deref().unwrap_or("Não disponível"));

                        ui.label(RichText::new("Servidores DNS:").strong());
                        ui.label(if iface.dns_addresses.is_empty() {
                            "Nenhum".to_string()
                        } else {
                            iface.dns_addresses.join(", ")
                        });
                        ui.end_row();
                    });
            } else {
                ui.label("Nenhuma interface física conectada foi identificada pelo sistema.");
            }
        });

    ui.add_space(8.0);

    // 2. Tabela de Hosts Monitorados
    ui.heading(RichText::new("Hosts Monitorados em Tempo Real").strong());

    egui::Frame::group(ui.style())
        .fill(ui.visuals().panel_fill)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("hosts_table")
                        .striped(true)
                        .spacing([18.0, 10.0])
                        .show(ui, |ui| {
                            // Cabeçalho da Tabela
                            ui.label(RichText::new("Host").strong());
                            ui.label(RichText::new("Endereço").strong());
                            ui.label(RichText::new("Latência Atual").strong());
                            ui.label(RichText::new("Média").strong());
                            ui.label(RichText::new("Mínima").strong());
                            ui.label(RichText::new("Máxima").strong());
                            ui.label(RichText::new("Jitter (RFC 3550)").strong());
                            ui.label(RichText::new("Perda %").strong());
                            ui.label(RichText::new("Enviados").strong());
                            ui.label(RichText::new("Status").strong());
                            ui.end_row();

                            for stat in stats_list {
                                let quality = stat.quality(thresholds);
                                let (badge_color, status_text) = match quality {
                                    LatencyQuality::Good => {
                                        (Color32::from_rgb(46, 204, 113), "● BOM")
                                    }
                                    LatencyQuality::Fair => {
                                        (Color32::from_rgb(241, 196, 15), "● MÉDIO")
                                    }
                                    LatencyQuality::Poor => {
                                        (Color32::from_rgb(230, 126, 34), "● ALTO")
                                    }
                                    LatencyQuality::Offline => {
                                        (Color32::from_rgb(231, 76, 60), "● OFFLINE")
                                    }
                                };

                                ui.label(RichText::new(&stat.name).strong());

                                let addr_display = stat
                                    .resolved_ip
                                    .map(|ip| ip.to_string())
                                    .unwrap_or_else(|| stat.target_str.clone());
                                ui.label(addr_display);

                                // Latência atual colorida
                                if let Some(rtt) = stat.last_rtt_ms {
                                    ui.label(
                                        RichText::new(format!("{:.0} ms", rtt))
                                            .color(badge_color)
                                            .strong(),
                                    );
                                } else {
                                    ui.label(RichText::new("-").color(Color32::GRAY));
                                }

                                ui.label(
                                    stat.avg_rtt_ms
                                        .map(|r| format!("{:.1} ms", r))
                                        .unwrap_or_else(|| "-".to_string()),
                                );

                                ui.label(
                                    stat.min_rtt_ms
                                        .map(|r| format!("{:.0} ms", r))
                                        .unwrap_or_else(|| "-".to_string()),
                                );

                                ui.label(
                                    stat.max_rtt_ms
                                        .map(|r| format!("{:.0} ms", r))
                                        .unwrap_or_else(|| "-".to_string()),
                                );

                                ui.label(format!("{:.1} ms", stat.jitter_ms));

                                let loss_color = if stat.packet_loss_pct > 5.0 {
                                    Color32::from_rgb(231, 76, 60)
                                } else if stat.packet_loss_pct > 0.0 {
                                    Color32::from_rgb(241, 196, 15)
                                } else {
                                    ui.visuals().text_color()
                                };
                                ui.label(
                                    RichText::new(format!("{:.1}%", stat.packet_loss_pct))
                                        .color(loss_color),
                                );

                                ui.label(stat.sent_packets.to_string());

                                ui.label(RichText::new(status_text).color(badge_color).strong());
                                ui.end_row();
                            }
                        });
                });
        });
}
