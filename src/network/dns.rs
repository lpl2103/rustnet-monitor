//! # Benchmark e Diagnóstico de Resolução DNS (`dns.rs`)
//!
//! Envia requisições diretas RFC 1035 via UDP na porta 53 para medir
//! o tempo de resposta (ms) e a saúde dos servidores DNS.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DnsBenchmarkResult {
    pub server_name: &'static str,
    pub server_ip: Ipv4Addr,
    pub rtt_ms: Option<f64>,
    pub status: DnsStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsStatus {
    Ok,
    Timeout,
    Error(String),
}

impl DnsStatus {
    #[allow(dead_code)]
    pub fn is_ok(&self) -> bool {
        matches!(self, DnsStatus::Ok)
    }
}

/// Constrói um pacote de consulta DNS RFC 1035 para registro do tipo A.
pub fn build_dns_query(domain: &str, transaction_id: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(64);

    // Header (12 bytes)
    packet.extend_from_slice(&transaction_id.to_be_bytes()); // ID
    packet.extend_from_slice(&[0x01, 0x00]); // Flags: RD = 1 (Recursion Desired)
    packet.extend_from_slice(&[0x00, 0x01]); // QDCOUNT = 1
    packet.extend_from_slice(&[0x00, 0x00]); // ANCOUNT = 0
    packet.extend_from_slice(&[0x00, 0x00]); // NSCOUNT = 0
    packet.extend_from_slice(&[0x00, 0x00]); // ARCOUNT = 0

    // Question: QNAME
    for label in domain.split('.') {
        if label.is_empty() {
            continue;
        }
        let len = label.len().min(63) as u8;
        packet.push(len);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0x00); // Fim do QNAME

    // QTYPE = 1 (A)
    packet.extend_from_slice(&[0x00, 0x01]);
    // QCLASS = 1 (IN)
    packet.extend_from_slice(&[0x00, 0x01]);

    packet
}

/// Dispara uma consulta DNS direta via UDP para o servidor e mede o tempo de resposta.
pub fn probe_dns_server(server_ip: Ipv4Addr, domain: &str, timeout: Duration) -> (Option<f64>, DnsStatus) {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => return (None, DnsStatus::Error(format!("Erro ao criar socket: {}", e))),
    };

    if socket.set_read_timeout(Some(timeout)).is_err() || socket.set_write_timeout(Some(timeout)).is_err() {
        return (None, DnsStatus::Error("Falha ao configurar timeout".to_string()));
    }

    let tx_id = 0x42A1;
    let query_bytes = build_dns_query(domain, tx_id);
    let dest_addr = SocketAddr::new(server_ip.into(), 53);

    let start = Instant::now();
    if socket.send_to(&query_bytes, dest_addr).is_err() {
        return (None, DnsStatus::Error("Falha ao enviar datagrama UDP".to_string()));
    }

    let mut buf = [0u8; 512];
    match socket.recv_from(&mut buf) {
        Ok((len, _)) => {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            if len >= 12 {
                // Valida Transaction ID retornado
                let resp_id = u16::from_be_bytes([buf[0], buf[1]]);
                let flags = u16::from_be_bytes([buf[2], buf[3]]);
                let rcode = flags & 0x000F;

                if resp_id == tx_id && rcode == 0 {
                    (Some(elapsed), DnsStatus::Ok)
                } else if rcode == 3 {
                    (Some(elapsed), DnsStatus::Error("NXDOMAIN (Domínio não existe)".to_string()))
                } else {
                    (Some(elapsed), DnsStatus::Error(format!("RCODE erro: {}", rcode)))
                }
            } else {
                (None, DnsStatus::Error("Resposta DNS truncada".to_string()))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
                (None, DnsStatus::Timeout)
            } else {
                (None, DnsStatus::Error(format!("Erro de rede: {}", e)))
            }
        }
    }
}

/// Executa bateria comparativa de testes nos principais resolvedores DNS globais e no DNS local.
pub fn run_dns_benchmark(configured_dns: Option<Ipv4Addr>) -> Vec<DnsBenchmarkResult> {
    let mut servers = Vec::new();

    if let Some(local_dns) = configured_dns {
        servers.push(("DNS do Adaptador / Roteador", local_dns));
    }
    servers.push(("Cloudflare DNS", Ipv4Addr::new(1, 1, 1, 1)));
    servers.push(("Google Public DNS", Ipv4Addr::new(8, 8, 8, 8)));
    servers.push(("Quad9 Secure DNS", Ipv4Addr::new(9, 9, 9, 9)));
    servers.push(("OpenDNS", Ipv4Addr::new(208, 67, 222, 222)));

    let timeout = Duration::from_millis(1500);
    let mut results = Vec::new();

    for (name, ip) in servers {
        let (rtt, status) = probe_dns_server(ip, "google.com", timeout);
        results.push(DnsBenchmarkResult {
            server_name: name,
            server_ip: ip,
            rtt_ms: rtt,
            status,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_packet_generation() {
        let packet = build_dns_query("example.com", 0x1234);
        assert!(packet.len() > 12);
        assert_eq!(packet[0], 0x12);
        assert_eq!(packet[1], 0x34);
        // Label 'example' = 7 bytes
        assert_eq!(packet[12], 7);
    }
}
