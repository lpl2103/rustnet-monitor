//! # Módulo de Diagnóstico WAN, CGNAT e IP Público (`wan.rs`)
//!
//! Identifica se a conexão está sob CGNAT (RFC 6598) ou NAT Duplo (RFC 1918),
//! e consulta de forma assíncrona o IP público externo, ASN e localização.

use std::net::Ipv4Addr;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NatType {
    /// IP público atribuído diretamente na interface
    DirectPublicIp,
    /// Operadora utiliza CGNAT RFC 6598 (100.64.0.0/10)
    CgnatRfc6598,
    /// Roteador atrás de outro roteador ou NAT privado (10.x, 192.168.x, 172.16-31.x)
    DoubleNatRfc1918,
    /// Não foi possível determinar
    Unknown,
}

impl NatType {
    pub fn description(&self) -> &'static str {
        match self {
            NatType::DirectPublicIp => "IP Público Dedicado (Sem CGNAT)",
            NatType::CgnatRfc6598 => "CGNAT Detectado (RFC 6598 100.64.0.0/10)",
            NatType::DoubleNatRfc1918 => "NAT Duplo / Rede Privada (RFC 1918)",
            NatType::Unknown => "Indeterminado",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WanInfo {
    pub public_ip: Option<String>,
    pub isp_organization: Option<String>,
    pub city_country: Option<String>,
    pub nat_type: NatType,
    pub local_ip: Option<String>,
    pub last_checked_secs_ago: Option<u64>,
}

impl Default for WanInfo {
    fn default() -> Self {
        Self {
            public_ip: None,
            isp_organization: None,
            city_country: None,
            nat_type: NatType::Unknown,
            local_ip: None,
            last_checked_secs_ago: None,
        }
    }
}

/// Avalia se um endereço IPv4 pertence à faixa de CGNAT RFC 6598 (100.64.0.0/10).
pub fn is_rfc6598_cgnat(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

/// Avalia se um endereço IPv4 pertence a faixas privadas RFC 1918.
pub fn is_rfc1918_private(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();
    match octets[0] {
        10 => true,
        172 => (16..=31).contains(&octets[1]),
        192 => octets[1] == 168,
        _ => false,
    }
}

/// Classifica o tipo de NAT comparando o IP da interface local com o IP público externo.
pub fn classify_nat(local_ip_opt: Option<&Ipv4Addr>, public_ip_opt: Option<&str>) -> NatType {
    let Some(local_ip) = local_ip_opt else {
        return NatType::Unknown;
    };

    if is_rfc6598_cgnat(local_ip) {
        return NatType::CgnatRfc6598;
    }

    if let Some(pub_str) = public_ip_opt
        && let Ok(pub_ip) = pub_str.trim().parse::<Ipv4Addr>()
        && *local_ip == pub_ip
        && !is_rfc1918_private(local_ip)
    {
        return NatType::DirectPublicIp;
    }

    if is_rfc1918_private(local_ip) {
        return NatType::DoubleNatRfc1918;
    }

    NatType::Unknown
}

/// Consulta o IP público e metadados de WAN via endpoint leve da Cloudflare CDN trace.
pub fn query_wan_info(local_ip_opt: Option<Ipv4Addr>) -> WanInfo {
    info!("Consultando informações de WAN pública...");

    let agent = ureq::builder()
        .timeout(Duration::from_secs(4))
        .build();

    let mut public_ip = None;
    let mut city_country = None;

    // 1. Consulta rápida ao Cloudflare trace (altamente resiliente e rápido)
    if let Ok(resp) = agent
        .get("https://1.1.1.1/cdn-cgi/trace")
        .set("User-Agent", "RustNetMonitor/Diagnostic")
        .call()
        && let Ok(text) = resp.into_string()
    {
        let mut loc = "";
        let mut colo = "";
        for line in text.lines() {
            if let Some(ip) = line.strip_prefix("ip=") {
                public_ip = Some(ip.trim().to_string());
            } else if let Some(l) = line.strip_prefix("loc=") {
                loc = l.trim();
            } else if let Some(c) = line.strip_prefix("colo=") {
                colo = c.trim();
            }
        }
        if !loc.is_empty() {
            city_country = Some(format!("{} (PoP: {})", loc, colo));
        }
    }

    // 2. Se obteve IP público, tenta obter o ASN / Nome do Provedor via ipapi de fallback
    let mut isp_organization = None;
    if let Some(ref ip) = public_ip {
        let url = format!("https://ipwho.is/{}", ip);
        if let Ok(resp) = agent.get(&url).timeout(Duration::from_secs(3)).call()
            && let Ok(text) = resp.into_string()
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
        {
            if let Some(connection) = json.get("connection") {
                let isp = connection.get("isp").and_then(|v| v.as_str()).unwrap_or("");
                let asn = connection
                    .get("asn")
                    .and_then(|v| v.as_u64())
                    .map(|a| format!("AS{}", a))
                    .unwrap_or_default();
                if !isp.is_empty() {
                    isp_organization = Some(format!("{} ({})", isp, asn));
                }
            }
            if city_country.is_none() {
                let city = json.get("city").and_then(|v| v.as_str()).unwrap_or("");
                let country = json.get("country").and_then(|v| v.as_str()).unwrap_or("");
                if !city.is_empty() {
                    city_country = Some(format!("{}, {}", city, country));
                }
            }
        }
    } else {
        warn!("Não foi possível obter o IP público externo.");
    }

    let nat_type = classify_nat(local_ip_opt.as_ref(), public_ip.as_deref());

    WanInfo {
        public_ip,
        isp_organization,
        city_country,
        nat_type,
        local_ip: local_ip_opt.map(|ip| ip.to_string()),
        last_checked_secs_ago: Some(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgnat_rfc6598_detection() {
        assert!(is_rfc6598_cgnat(&Ipv4Addr::new(100, 64, 0, 1)));
        assert!(is_rfc6598_cgnat(&Ipv4Addr::new(100, 100, 50, 1)));
        assert!(is_rfc6598_cgnat(&Ipv4Addr::new(100, 127, 255, 254)));
        assert!(!is_rfc6598_cgnat(&Ipv4Addr::new(100, 63, 255, 255)));
        assert!(!is_rfc6598_cgnat(&Ipv4Addr::new(100, 128, 0, 1)));
        assert!(!is_rfc6598_cgnat(&Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn test_rfc1918_private_detection() {
        assert!(is_rfc1918_private(&Ipv4Addr::new(192, 168, 0, 1)));
        assert!(is_rfc1918_private(&Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_rfc1918_private(&Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_rfc1918_private(&Ipv4Addr::new(172, 31, 255, 255)));
        assert!(!is_rfc1918_private(&Ipv4Addr::new(172, 32, 0, 1)));
        assert!(!is_rfc1918_private(&Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn test_classify_nat() {
        let cgnat_ip = Ipv4Addr::new(100, 64, 10, 2);
        assert_eq!(classify_nat(Some(&cgnat_ip), Some("177.10.10.10")), NatType::CgnatRfc6598);

        let priv_ip = Ipv4Addr::new(192, 168, 1, 50);
        assert_eq!(classify_nat(Some(&priv_ip), Some("177.10.10.10")), NatType::DoubleNatRfc1918);

        let pub_ip = Ipv4Addr::new(177, 10, 10, 10);
        assert_eq!(classify_nat(Some(&pub_ip), Some("177.10.10.10")), NatType::DirectPublicIp);
    }
}
