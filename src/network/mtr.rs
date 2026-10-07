//! # Visual MTR & Hop-by-Hop Traceroute (`mtr.rs`)
//!
//! Executa rastreamento de rota e cálculo contínuo de latência e perda por salto (MTR)
//! utilizando a API Win32 `IcmpSendEcho` com TTL incremental e `IP_OPTION_INFORMATION`.

use std::mem::size_of;
use std::net::Ipv4Addr;
use std::time::Duration;
use windows_sys::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    ICMP_ECHO_REPLY, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IpOptionInformation {
    pub ttl: u8,
    pub tos: u8,
    pub flags: u8,
    pub options_size: u8,
    pub options_data: *mut u8,
}

#[derive(Debug, Clone)]
pub struct MtrHop {
    pub hop: u8,
    pub ip: Option<Ipv4Addr>,
    pub sent: u32,
    pub received: u32,
    pub loss_pct: f32,
    pub last_ms: Option<f64>,
    pub avg_ms: Option<f64>,
    pub min_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub rtt_history: Vec<f64>,
}

impl MtrHop {
    pub fn new(hop: u8) -> Self {
        Self {
            hop,
            ip: None,
            sent: 0,
            received: 0,
            loss_pct: 0.0,
            last_ms: None,
            avg_ms: None,
            min_ms: None,
            max_ms: None,
            rtt_history: Vec::new(),
        }
    }

    pub fn record_result(&mut self, found_ip: Option<Ipv4Addr>, rtt: Option<f64>) {
        self.sent += 1;
        if let Some(ip) = found_ip {
            self.ip = Some(ip);
        }

        if let Some(ms) = rtt {
            self.received += 1;
            self.last_ms = Some(ms);
            self.rtt_history.push(ms);
            if self.rtt_history.len() > 50 {
                self.rtt_history.remove(0);
            }

            let sum: f64 = self.rtt_history.iter().sum();
            self.avg_ms = Some(sum / self.rtt_history.len() as f64);
            self.min_ms = Some(
                self.min_ms
                    .map(|m| m.min(ms))
                    .unwrap_or(ms),
            );
            self.max_ms = Some(
                self.max_ms
                    .map(|m| m.max(ms))
                    .unwrap_or(ms),
            );
        }

        let lost = self.sent.saturating_sub(self.received);
        self.loss_pct = (lost as f32 / self.sent as f32) * 100.0;
    }

    /// Retorna a quantidade absoluta de pacotes perdidos no salto.
    pub fn lost(&self) -> u32 {
        self.sent.saturating_sub(self.received)
    }
}

/// Dispara uma única rodada de sondagem para um TTL específico.
/// Retorna `(Option<Ipv4Addr>, Option<f64>, bool_alvo_alcancado)`.
pub fn probe_hop(target: Ipv4Addr, ttl: u8, timeout: Duration) -> (Option<Ipv4Addr>, Option<f64>, bool) {
    let handle = unsafe { IcmpCreateFile() };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return (None, None, false);
    }

    let ip_octets = target.octets();
    let ip_addr = u32::from_ne_bytes(ip_octets);

    let send_data = b"RustNetMtrProbeHop32BytesDataPayload";
    let reply_buf_len = (size_of::<ICMP_ECHO_REPLY>() + send_data.len() + 32) as u32;
    let mut reply_buf = vec![0u8; reply_buf_len as usize];

    let mut options = IpOptionInformation {
        ttl,
        tos: 0,
        flags: 0,
        options_size: 0,
        options_data: std::ptr::null_mut(),
    };

    let timeout_ms = timeout.as_millis().min(2000) as u32;

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
        return (None, None, false);
    }

    let reply = unsafe { &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY) };

    // Converte o endereço retornado do roteador (u32 em network byte order)
    let router_bytes = reply.Address.to_ne_bytes();
    let router_ip = Ipv4Addr::from(router_bytes);

    let rtt = reply.RoundTripTime as f64;

    // 0 = IP_SUCCESS (Alvo final alcançado)
    if reply.Status == 0 {
        (Some(router_ip), Some(rtt), true)
    } else if reply.Status == 11013 {
        // 11013 = IP_TTL_EXPIRED_TRANSIT (Salto intermediário)
        (Some(router_ip), Some(rtt), false)
    } else {
        // Timeout ou outro código de erro ICMP
        (None, None, false)
    }
}

/// Executa um ciclo completo de sondagem MTR para até `max_hops` saltos.
pub fn run_mtr_cycle(target: Ipv4Addr, hops: &mut Vec<MtrHop>, max_hops: u8) {
    if hops.is_empty() {
        for h in 1..=max_hops {
            hops.push(MtrHop::new(h));
        }
    }

    let timeout = Duration::from_millis(800);

    for hop_idx in 0..hops.len() {
        let ttl = (hop_idx + 1) as u8;
        let (ip_opt, rtt_opt, target_reached) = probe_hop(target, ttl, timeout);

        hops[hop_idx].record_result(ip_opt, rtt_opt);

        if target_reached {
            // Trunca a exibição para os saltos até alcançar o alvo
            hops.truncate(hop_idx + 1);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mtr_hop_stats() {
        let mut hop = MtrHop::new(1);
        assert_eq!(hop.hop, 1);
        assert_eq!(hop.loss_pct, 0.0);

        hop.record_result(Some(Ipv4Addr::new(192, 168, 1, 1)), Some(2.0));
        assert_eq!(hop.received, 1);
        assert_eq!(hop.lost(), 0);
        assert_eq!(hop.loss_pct, 0.0);
        assert_eq!(hop.min_ms, Some(2.0));

        hop.record_result(None, None);
        assert_eq!(hop.sent, 2);
        assert_eq!(hop.received, 1);
        assert_eq!(hop.lost(), 1);
        assert_eq!(hop.loss_pct, 50.0);
    }
}
