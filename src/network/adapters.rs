use std::ffi::CStr;
use std::net::{Ipv4Addr, Ipv6Addr};
use thiserror::Error;
use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_INCLUDE_PREFIX, GAA_FLAG_SKIP_ANYCAST,
    GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses, GetIfEntry2, IP_ADAPTER_ADDRESSES_LH,
    IP_ADAPTER_DNS_SERVER_ADDRESS_XP, IP_ADAPTER_GATEWAY_ADDRESS_LH, IP_ADAPTER_UNICAST_ADDRESS_LH,
    MIB_IF_ROW2,
};
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
};

use crate::network::types::{AdapterType, NetworkAdapter, OperStatus};
use crate::network::virtual_filter::classify_adapter;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("Falha ao invocar GetAdaptersAddresses (código Windows: {0})")]
    GetAdaptersFailed(u32),
}

/// Converte ponteiro de string UTF-16 terminada em nulo (PWSTR) para `String`.
unsafe fn pwstr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

/// Converte ponteiro de string ANSI/ASCII terminada em nulo (PSTR) para `String`.
unsafe fn pstr_to_string(ptr: *const u8) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr as *const std::os::raw::c_char) }
        .to_string_lossy()
        .into_owned()
}

/// Extrai endereço IP de um ponteiro `SOCKADDR` nativo do WinSock.
unsafe fn extract_ip_from_sockaddr(sockaddr_ptr: *const SOCKADDR) -> Option<String> {
    if sockaddr_ptr.is_null() {
        return None;
    }

    let family = unsafe { (*sockaddr_ptr).sa_family } as u32;
    if family == AF_INET as u32 {
        let sin = unsafe { &*(sockaddr_ptr as *const SOCKADDR_IN) };
        let bytes = unsafe { sin.sin_addr.S_un.S_un_b };
        let ipv4 = Ipv4Addr::new(bytes.s_b1, bytes.s_b2, bytes.s_b3, bytes.s_b4);
        Some(ipv4.to_string())
    } else if family == AF_INET6 as u32 {
        let sin6 = unsafe { &*(sockaddr_ptr as *const SOCKADDR_IN6) };
        let bytes = unsafe { sin6.sin6_addr.u.Byte };
        let ipv6 = Ipv6Addr::from(bytes);
        Some(ipv6.to_string())
    } else {
        None
    }
}

/// Consulta todos os adaptadores de rede registrados no Windows utilizando a API nativa `GetAdaptersAddresses`.
pub fn get_network_adapters() -> Result<Vec<NetworkAdapter>, AdapterError> {
    let mut buf_len: u32 = 16384;
    let mut buffer: Vec<u8> = vec![0; buf_len as usize];

    let flags = GAA_FLAG_INCLUDE_GATEWAYS
        | GAA_FLAG_INCLUDE_PREFIX
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST;

    let mut res = unsafe {
        GetAdaptersAddresses(
            AF_UNSPEC as u32,
            flags,
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
            &mut buf_len,
        )
    };

    if res == ERROR_BUFFER_OVERFLOW {
        buffer.resize(buf_len as usize, 0);
        res = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                flags,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                &mut buf_len,
            )
        };
    }

    if res != NO_ERROR {
        return Err(AdapterError::GetAdaptersFailed(res));
    }

    let mut adapters = Vec::new();
    let mut curr_ptr = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;

    while !curr_ptr.is_null() {
        let adapter = unsafe { &*curr_ptr };

        let adapter_name = unsafe { pstr_to_string(adapter.AdapterName) };
        let friendly_name = unsafe { pwstr_to_string(adapter.FriendlyName) };
        let description = unsafe { pwstr_to_string(adapter.Description) };

        let if_type = match adapter.IfType {
            6 => AdapterType::Ethernet,
            71 => AdapterType::Wireless,
            24 => AdapterType::Loopback,
            131 => AdapterType::Tunnel,
            53 => AdapterType::Virtual,
            other => AdapterType::Other(other),
        };

        let oper_status = match adapter.OperStatus {
            1 => OperStatus::Up,
            2 => OperStatus::Down,
            3 => OperStatus::Testing,
            4 => OperStatus::Unknown,
            5 => OperStatus::Dormant,
            6 => OperStatus::NotPresent,
            7 => OperStatus::LowerLayerDown,
            _ => OperStatus::Unknown,
        };

        let if_index = unsafe { adapter.Anonymous1.Anonymous.IfIndex };
        let ipv6_if_index = adapter.Ipv6IfIndex;

        // Endereço MAC
        let mac_address = if adapter.PhysicalAddressLength > 0 {
            let len = (adapter.PhysicalAddressLength as usize).min(8);
            let mac_str = adapter.PhysicalAddress[..len]
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(":");
            Some(mac_str)
        } else {
            None
        };

        // Endereços Unicast (IPv4 e IPv6)
        let mut ipv4_addresses = Vec::new();
        let mut ipv6_addresses = Vec::new();
        let mut curr_unicast: *mut IP_ADAPTER_UNICAST_ADDRESS_LH = adapter.FirstUnicastAddress;
        while !curr_unicast.is_null() {
            let u_addr = unsafe { &*curr_unicast };
            if let Some(ip_str) = unsafe { extract_ip_from_sockaddr(u_addr.Address.lpSockaddr) } {
                if ip_str.contains('.') {
                    ipv4_addresses.push(ip_str);
                } else if ip_str.contains(':') {
                    ipv6_addresses.push(ip_str);
                }
            }
            curr_unicast = u_addr.Next;
        }

        // Gateways
        let mut gateway_addresses = Vec::new();
        let mut curr_gw: *mut IP_ADAPTER_GATEWAY_ADDRESS_LH = adapter.FirstGatewayAddress;
        while !curr_gw.is_null() {
            let gw = unsafe { &*curr_gw };
            if let Some(gw_str) = unsafe { extract_ip_from_sockaddr(gw.Address.lpSockaddr) } {
                gateway_addresses.push(gw_str);
            }
            curr_gw = gw.Next;
        }

        // Servidores DNS
        let mut dns_addresses = Vec::new();
        let mut curr_dns: *mut IP_ADAPTER_DNS_SERVER_ADDRESS_XP = adapter.FirstDnsServerAddress;
        while !curr_dns.is_null() {
            let dns = unsafe { &*curr_dns };
            if let Some(dns_str) = unsafe { extract_ip_from_sockaddr(dns.Address.lpSockaddr) } {
                dns_addresses.push(dns_str);
            }
            curr_dns = dns.Next;
        }

        // Consulta NDIS GetIfEntry2 para flags de hardware e conector físico
        let mut row: MIB_IF_ROW2 = unsafe { std::mem::zeroed() };
        row.InterfaceIndex = if_index;
        let if_res = unsafe { GetIfEntry2(&mut row) };

        let (is_hardware, connector_present) = if if_res == NO_ERROR {
            let bf = row.InterfaceAndOperStatusFlags._bitfield;
            let hw = (bf & 0x01) != 0;
            let conn = (bf & 0x04) != 0;
            (hw, conn)
        } else {
            (false, false)
        };

        // Classificação física vs virtual
        let (is_virtual, virtual_reason) = classify_adapter(
            if_type,
            is_hardware,
            connector_present,
            &friendly_name,
            &description,
        );

        adapters.push(NetworkAdapter {
            adapter_name,
            friendly_name,
            description,
            if_type,
            oper_status,
            if_index,
            ipv6_if_index,
            mac_address,
            ipv4_addresses,
            ipv6_addresses,
            gateway_addresses,
            dns_addresses,
            transmit_link_speed: adapter.TransmitLinkSpeed,
            receive_link_speed: adapter.ReceiveLinkSpeed,
            is_hardware,
            connector_present,
            is_virtual,
            virtual_reason,
        });

        curr_ptr = adapter.Next;
    }

    Ok(adapters)
}
