use std::net::Ipv4Addr;
use std::time::Duration;
use thiserror::Error;
use windows_sys::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    ICMP_ECHO_REPLY, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
};

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum IcmpError {
    #[error("Falha ao abrir handle ICMP (IcmpCreateFile)")]
    CreateHandleFailed,
    #[error("Timeout ao aguardar resposta ICMP ({0} ms)")]
    Timeout(u32),
    #[error("Host inalcançável (código de status ICMP: {0})")]
    Unreachable(u32),
    #[error("Erro Win32 na chamada IcmpSendEcho (código de erro: {0})")]
    Win32(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PingStatus {
    Success,
    Timeout,
    Error,
}

#[derive(Debug, Clone)]
pub struct PingResult {
    pub target_ip: Ipv4Addr,
    pub rtt_ms: f64,
    pub status: PingStatus,
    pub error_message: Option<String>,
}

/// Executa um teste de conectividade ICMP Echo Request nativo para o IPv4 de destino.
pub fn ping_ipv4(target: Ipv4Addr, timeout: Duration) -> PingResult {
    let handle = unsafe { IcmpCreateFile() };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        let err = unsafe { GetLastError() };
        return PingResult {
            target_ip: target,
            rtt_ms: 0.0,
            status: PingStatus::Error,
            error_message: Some(format!("Falha ao inicializar handle ICMP (erro: {})", err)),
        };
    }

    let ip_octets = target.octets();
    // IPAddr no Windows é um u32 em network byte order (little-endian representation dos 4 octetos)
    let ip_addr = u32::from_ne_bytes(ip_octets);

    let send_data = b"RustNetMonitorEchoPayload32Bytes";
    let reply_buf_len = (size_of::<ICMP_ECHO_REPLY>() + send_data.len() + 32) as u32;
    let mut reply_buf = vec![0u8; reply_buf_len as usize];

    let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;

    let replies = unsafe {
        IcmpSendEcho(
            handle,
            ip_addr,
            send_data.as_ptr() as *const _,
            send_data.len() as u16,
            std::ptr::null(),
            reply_buf.as_mut_ptr() as *mut _,
            reply_buf_len,
            timeout_ms,
        )
    };

    unsafe { IcmpCloseHandle(handle) };

    if replies == 0 {
        let err = unsafe { GetLastError() };
        // 11010 = IP_REQ_TIMED_OUT
        if err == 11010 {
            return PingResult {
                target_ip: target,
                rtt_ms: timeout_ms as f64,
                status: PingStatus::Timeout,
                error_message: Some("Tempo limite esgotado (Timeout)".to_string()),
            };
        } else {
            return PingResult {
                target_ip: target,
                rtt_ms: 0.0,
                status: PingStatus::Error,
                error_message: Some(format!("Erro no envio ICMP (código: {})", err)),
            };
        }
    }

    let reply = unsafe { &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY) };

    // Status 0 = IP_SUCCESS
    if reply.Status == 0 {
        let rtt = reply.RoundTripTime as f64;
        PingResult {
            target_ip: target,
            rtt_ms: rtt,
            status: PingStatus::Success,
            error_message: None,
        }
    } else if reply.Status == 11010 {
        PingResult {
            target_ip: target,
            rtt_ms: timeout_ms as f64,
            status: PingStatus::Timeout,
            error_message: Some("Tempo limite esgotado".to_string()),
        }
    } else {
        PingResult {
            target_ip: target,
            rtt_ms: 0.0,
            status: PingStatus::Error,
            error_message: Some(format!(
                "Status ICMP diferente de sucesso: {}",
                reply.Status
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_loopback() {
        let res = ping_ipv4(Ipv4Addr::new(127, 0, 0, 1), Duration::from_millis(1000));
        assert_eq!(res.status, PingStatus::Success);
        assert!(res.rtt_ms >= 0.0);
    }
}
