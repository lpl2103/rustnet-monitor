//! # Detector de Path MTU & Black Hole (`mtu.rs`)
//!
//! Descobre o MTU máximo suportado no caminho de rede até um alvo
//! utilizando ICMP com a flag DF (*Don't Fragment* - RFC 1191).

use std::mem::size_of;
use std::net::Ipv4Addr;
use std::time::Duration;
use windows_sys::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    ICMP_ECHO_REPLY, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
};

use crate::network::mtr::IpOptionInformation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MtuTestResult {
    pub target: Ipv4Addr,
    pub max_mtu: u16,
    pub classification: &'static str,
    pub details: String,
    pub packets_tested: u32,
}

/// Envia um pacote ICMP com a flag DF (*Don't Fragment*) para testar se um determinado MTU passa.
/// `total_packet_size`: tamanho total do pacote IP (inclui cabeçalho IP 20B + ICMP 8B = 28B).
pub fn probe_mtu_size(target: Ipv4Addr, total_packet_size: u16, timeout: Duration) -> bool {
    if total_packet_size <= 28 {
        return false;
    }

    let handle = unsafe { IcmpCreateFile() };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return false;
    }

    let payload_len = (total_packet_size - 28) as usize;
    let send_data = vec![0x41u8; payload_len];

    let reply_buf_len = (size_of::<ICMP_ECHO_REPLY>() + payload_len + 32) as u32;
    let mut reply_buf = vec![0u8; reply_buf_len as usize];

    // Flags: 0x02 = IP_FLAG_DF (Don't Fragment)
    let mut options = IpOptionInformation {
        ttl: 64,
        tos: 0,
        flags: 0x02,
        options_size: 0,
        options_data: std::ptr::null_mut(),
    };

    let ip_octets = target.octets();
    let ip_addr = u32::from_ne_bytes(ip_octets);
    let timeout_ms = timeout.as_millis().min(1500) as u32;

    let replies = unsafe {
        IcmpSendEcho(
            handle,
            ip_addr,
            send_data.as_ptr() as *const _,
            send_data.len() as u16,
            &mut options as *mut _ as *const _,
            reply_buf.as_mut_ptr() as *mut _,
            reply_buf_len,
            timeout_ms,
        )
    };

    unsafe { IcmpCloseHandle(handle) };

    if replies == 0 {
        let _ = unsafe { GetLastError() };
        return false;
    }

    let reply = unsafe { &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY) };
    reply.Status == 0
}

/// Executa busca binária de Path MTU Discovery entre 1280 e 1500 bytes.
pub fn discover_path_mtu(target: Ipv4Addr) -> MtuTestResult {
    let timeout = Duration::from_millis(600);
    let mut low = 1280u16; // MTU mínimo padrão IPv6 / VPN
    let mut max_success = 1280u16;
    let mut tested_count = 0u32;

    // Primeiro teste rápido: o padrão Ethernet (1500) passa de primeira?
    tested_count += 1;
    if probe_mtu_size(target, 1500, timeout) {
        return MtuTestResult {
            target,
            max_mtu: 1500,
            classification: "Ethernet Padrão (1500B)",
            details: "Caminho suporta quadros Ethernet integrais de 1500 bytes sem fragmentação."
                .to_string(),
            packets_tested: tested_count,
        };
    }

    // Busca binária para encontrar o MTU exato
    let mut high = 1499u16;
    while low <= high {
        let mid = low + (high - low) / 2;
        tested_count += 1;

        if probe_mtu_size(target, mid, timeout) {
            max_success = mid;
            low = mid + 1; // Tenta maior
        } else {
            if mid == 0 {
                break;
            }
            high = mid - 1; // Reduz tamanho
        }
    }

    let (classification, details) = match max_success {
        1500 => (
            "Ethernet Padrão (1500B)",
            "Caminho suporta quadros integrais de 1500 bytes.".to_string(),
        ),
        1492 => (
            "PPPoE Fibra (1492B)",
            "MTU típico de conexões de banda larga com cabeçalho PPPoE de 8 bytes.".to_string(),
        ),
        1400..=1491 => (
            "Túnel / VPN / MTU Reduzido",
            format!(
                "MTU reduzido a {} bytes (típico de túneis WireGuard, IPsec ou VLANs).",
                max_success
            ),
        ),
        _ => (
            "MTU Baixo / Restritivo",
            format!(
                "MTU restrito a {} bytes. Pode causar retransmissões ou problemas de carregamento.",
                max_success
            ),
        ),
    };

    MtuTestResult {
        target,
        max_mtu: max_success,
        classification,
        details,
        packets_tested: tested_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mtu_size_validation() {
        assert!(!probe_mtu_size(Ipv4Addr::new(127, 0, 0, 1), 20, Duration::from_millis(100)));
    }
}
