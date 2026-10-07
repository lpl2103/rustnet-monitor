//! # Gerador de Laudo Técnico de Conectividade (`report.rs`)
//!
//! Compila dados de telemetria, estatísticas acumuladas e eventos de queda
//! em um relatório técnico formal formatado em Markdown para envio ao suporte.

use std::fs;
use std::path::Path;

use crate::network::stats::HostStats;
use crate::network::types::NetworkDiagnostic;
use crate::network::wan::WanInfo;
use crate::network::wifi::WifiInfo;

pub struct ReportData<'a> {
    pub diagnostic: &'a NetworkDiagnostic,
    pub stats_list: &'a [HostStats],
    pub wan_info: &'a Option<WanInfo>,
    pub wifi_info: &'a Option<WifiInfo>,
    pub app_version: &'a str,
}

/// Gera o laudo técnico completo em formato Markdown legível.
pub fn generate_technical_report(data: &ReportData) -> String {
    let mut out = String::with_capacity(4096);

    out.push_str("================================================================================\n");
    out.push_str("          RELATÓRIO TÉCNICO DE DIAGNÓSTICO DE REDE E CONECTIVIDADE (SLA)        \n");
    out.push_str("================================================================================\n\n");

    out.push_str(&format!("* Aplicativo: RustNet Monitor v{}\n", data.app_version));
    out.push_str("* Plataforma: Microsoft Windows (x86_64 Nativo)\n\n");

    // 1. Camada Física e Adaptador
    out.push_str("--------------------------------------------------------------------------------\n");
    out.push_str("1. ADAPTADOR DE REDE E CAMADA DE ENLACE\n");
    out.push_str("--------------------------------------------------------------------------------\n");

    if let Some(iface) = &data.diagnostic.active_physical_interface {
        out.push_str(&format!("* Nome da Interface:   {}\n", iface.friendly_name));
        out.push_str(&format!("* Descrição / Chipset: {}\n", iface.description));
        out.push_str(&format!("* Endereço MAC:        {}\n", iface.mac_address.as_deref().unwrap_or("N/D")));
        out.push_str(&format!("* Velocidade do Link:  {}\n", iface.formatted_speed()));
        out.push_str(&format!("* IPv4 Local:          {}\n", iface.ipv4_addresses.join(", ")));
        out.push_str(&format!("* Gateway Padrão:      {}\n", iface.gateway_addresses.join(", ")));
        out.push_str(&format!("* DNS Configurado:     {}\n", iface.dns_addresses.join(", ")));
    } else {
        out.push_str("* Nenhuma interface física ativa identificada.\n");
    }

    // Telemetria Wi-Fi (se aplicável)
    if let Some(wifi) = data.wifi_info {
        out.push_str("\n[Telemetria Wi-Fi 802.11]\n");
        out.push_str(&format!("* Rede (SSID):         {}\n", wifi.ssid));
        out.push_str(&format!("* BSSID (Access Point):{}\n", wifi.bssid));
        out.push_str(&format!("* Qualidade do Sinal:  {}% (RSSI: {} dBm - {})\n", wifi.signal_quality_pct, wifi.rssi_dbm, wifi.signal_badge()));
        out.push_str(&format!("* Padrão PHY:          {}\n", wifi.phy_type));
        out.push_str(&format!("* Tx / Rx Link Rate:   {:.1} Mbps / {:.1} Mbps\n", wifi.tx_rate_mbps, wifi.rx_rate_mbps));
    }

    out.push_str("\n--------------------------------------------------------------------------------\n");
    out.push_str("2. INFORMAÇÕES DE WAN, PROVEDOR E CGNAT\n");
    out.push_str("--------------------------------------------------------------------------------\n");

    if let Some(wan) = data.wan_info {
        out.push_str(&format!("* IP Público Externo:  {}\n", wan.public_ip.as_deref().unwrap_or("Não detectado")));
        out.push_str(&format!("* Provedor / ASN:      {}\n", wan.isp_organization.as_deref().unwrap_or("N/D")));
        out.push_str(&format!("* Localidade (PoP):    {}\n", wan.city_country.as_deref().unwrap_or("N/D")));
        out.push_str(&format!("* Topologia NAT:       {}\n", wan.nat_type.description()));
    } else {
        out.push_str("* Dados de WAN pública ainda não sincronizados.\n");
    }

    out.push_str("\n--------------------------------------------------------------------------------\n");
    out.push_str("3. MÉTRICAS ESTATÍSTICAS DE LATÊNCIA, JITTER E PERDA DE PACOTES\n");
    out.push_str("--------------------------------------------------------------------------------\n\n");

    out.push_str("Host                 | Destino         | Enviados | Perda % |  Atual |  Média |    Mín |    Máx | Jitter\n");
    out.push_str("---------------------+-----------------+----------+---------+--------+--------+--------+--------+--------\n");

    let mut total_sent = 0u64;
    let mut total_lost = 0u64;

    for s in data.stats_list {
        let addr = s.resolved_ip.map(|ip| ip.to_string()).unwrap_or_else(|| s.target_str.clone());
        let cur = s.last_rtt_ms.map(|v| format!("{:.0}ms", v)).unwrap_or_else(|| "-".to_string());
        let avg = s.avg_rtt_ms.map(|v| format!("{:.0}ms", v)).unwrap_or_else(|| "-".to_string());
        let min = s.min_rtt_ms.map(|v| format!("{:.0}ms", v)).unwrap_or_else(|| "-".to_string());
        let max = s.max_rtt_ms.map(|v| format!("{:.0}ms", v)).unwrap_or_else(|| "-".to_string());
        let jit = format!("{:.0}ms", s.jitter_ms);

        total_sent += s.sent_packets;
        total_lost += s.lost_packets;

        out.push_str(&format!(
            "{:<20} | {:<15} | {:>8} | {:>6.1}% | {:>6} | {:>6} | {:>6} | {:>6} | {:>6}\n",
            s.name, addr, s.sent_packets, s.packet_loss_pct, cur, avg, min, max, jit
        ));
    }

    out.push_str("\n--------------------------------------------------------------------------------\n");
    out.push_str("4. PARECER TÉCNICO DE CONFORMIDADE DE SLA\n");
    out.push_str("--------------------------------------------------------------------------------\n");

    let global_loss_pct = if total_sent > 0 {
        (total_lost as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };

    if global_loss_pct == 0.0 {
        out.push_str("[STATUS: CONFORME - REDE EXCELENTE]\n");
        out.push_str("- Perda de pacotes global registrada em 0.0%.\n");
        out.push_str("- Enlace estável com baixa dispersão de jitter.\n");
    } else if global_loss_pct < 2.0 {
        out.push_str("[STATUS: TOLERÁVEL - INSTABILIDADE LEVE]\n");
        out.push_str(&format!("- Perda de pacotes acumulada em {:.2}%.\n", global_loss_pct));
        out.push_str("- Oscilações residuais aceitáveis em banda larga comercial.\n");
    } else {
        out.push_str("[STATUS: NÃO CONFORME - DEGRADAÇÃO SEVERA DE SLA]\n");
        out.push_str(&format!("- ALERTA: Perda global de pacotes excessiva de {:.2}%!\n", global_loss_pct));
        out.push_str("- Recomenda-se acionamento do suporte da operadora para verificação da rota e atenuação óptica.\n");
    }

    out.push_str("\n================================================================================\n");
    out.push_str("                  FIM DO LAUDO TÉCNICO - RUSTNET MONITOR                       \n");
    out.push_str("================================================================================\n");

    out
}

/// Grava o laudo gerado em arquivo de texto no disco.
pub fn save_report_to_file(content: &str, output_path: &Path) -> Result<(), std::io::Error> {
    fs::write(output_path, content)
}
