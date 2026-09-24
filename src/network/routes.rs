use std::net::{Ipv4Addr, Ipv6Addr};
use thiserror::Error;
use windows_sys::Win32::Foundation::NO_ERROR;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIpForwardTable2, GetIpInterfaceEntry, MIB_IPFORWARD_ROW2,
    MIB_IPFORWARD_TABLE2, MIB_IPINTERFACE_ROW,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, IN_ADDR, IN6_ADDR};

use crate::network::types::RouteEntry;

#[derive(Error, Debug)]
pub enum RouteError {
    #[error("Falha ao invocar GetIpForwardTable2 (código Windows: {0})")]
    GetForwardTableFailed(u32),
    #[error("Tabela de rotas retornou nula")]
    NullTable,
}

/// Obtém a métrica configurada para a interface através de `GetIpInterfaceEntry`.
fn get_interface_metric(if_index: u32, family: u16) -> u32 {
    let mut if_row: MIB_IPINTERFACE_ROW = unsafe { std::mem::zeroed() };
    if_row.Family = family;
    if_row.InterfaceIndex = if_index;

    let res = unsafe { GetIpInterfaceEntry(&mut if_row) };
    if res == NO_ERROR { if_row.Metric } else { 0 }
}

/// Consulta a tabela de rotas do Windows e retorna as rotas padrão (IPv4 e IPv6).
pub fn get_default_routes() -> Result<Vec<RouteEntry>, RouteError> {
    let mut table: *mut MIB_IPFORWARD_TABLE2 = std::ptr::null_mut();
    let res = unsafe { GetIpForwardTable2(AF_UNSPEC, &mut table) };

    if res != NO_ERROR {
        return Err(RouteError::GetForwardTableFailed(res));
    }
    if table.is_null() {
        return Err(RouteError::NullTable);
    }

    let mut routes = Vec::new();
    let num_entries = unsafe { (*table).NumEntries };
    let entries_ptr = unsafe { (*table).Table.as_ptr() };

    for i in 0..num_entries {
        let row: &MIB_IPFORWARD_ROW2 = unsafe { &*entries_ptr.add(i as usize) };
        let family = unsafe { row.DestinationPrefix.Prefix.si_family };
        let prefix_len = row.DestinationPrefix.PrefixLength;

        // Rota padrão possui prefix_len == 0 (0.0.0.0/0 ou ::/0)
        if prefix_len == 0 {
            let is_ipv4 = family == AF_INET;
            let (dst_str, next_hop_str) = if is_ipv4 {
                let sin_addr: IN_ADDR = unsafe { row.NextHop.Ipv4.sin_addr };
                let bytes = unsafe { sin_addr.S_un.S_un_b };
                let hop = Ipv4Addr::new(bytes.s_b1, bytes.s_b2, bytes.s_b3, bytes.s_b4).to_string();
                ("0.0.0.0/0".to_string(), hop)
            } else if family == AF_INET6 {
                let sin6_addr: IN6_ADDR = unsafe { row.NextHop.Ipv6.sin6_addr };
                let bytes = unsafe { sin6_addr.u.Byte };
                let ip6 = Ipv6Addr::from(bytes).to_string();
                ("::/0".to_string(), ip6)
            } else {
                continue;
            };

            // Ignora rotas com next_hop zerado se houver rota com gateway explícito
            let if_metric = get_interface_metric(row.InterfaceIndex, family);
            let total_metric = row.Metric.saturating_add(if_metric);

            routes.push(RouteEntry {
                destination: dst_str,
                next_hop: next_hop_str,
                interface_index: row.InterfaceIndex,
                interface_alias: None,
                route_metric: row.Metric,
                interface_metric: if_metric,
                total_metric,
                is_ipv4,
            });
        }
    }

    unsafe { FreeMibTable(table as *mut _) };

    // Ordena por métrica total crescente (menor métrica = maior prioridade no Windows)
    routes.sort_by_key(|r| r.total_metric);

    Ok(routes)
}

/// Identifica a melhor rota padrão IPv4 ativa (com next_hop válido e menor métrica).
pub fn find_best_ipv4_default_route(routes: &[RouteEntry]) -> Option<&RouteEntry> {
    routes
        .iter()
        .filter(|r| r.is_ipv4 && r.next_hop != "0.0.0.0" && !r.next_hop.is_empty())
        .min_by_key(|r| r.total_metric)
}

/// Identifica a melhor rota padrão IPv6 ativa.
pub fn find_best_ipv6_default_route(routes: &[RouteEntry]) -> Option<&RouteEntry> {
    routes
        .iter()
        .filter(|r| !r.is_ipv4 && r.next_hop != "::" && !r.next_hop.is_empty())
        .min_by_key(|r| r.total_metric)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_best_ipv4_route_selection() {
        let routes = vec![
            RouteEntry {
                destination: "0.0.0.0/0".to_string(),
                next_hop: "26.0.0.1".to_string(),
                interface_index: 23,
                interface_alias: Some("Radmin VPN".to_string()),
                route_metric: 9256,
                interface_metric: 1,
                total_metric: 9257,
                is_ipv4: true,
            },
            RouteEntry {
                destination: "0.0.0.0/0".to_string(),
                next_hop: "192.168.100.1".to_string(),
                interface_index: 7,
                interface_alias: Some("Ethernet 2".to_string()),
                route_metric: 0,
                interface_metric: 25,
                total_metric: 25,
                is_ipv4: true,
            },
            RouteEntry {
                destination: "0.0.0.0/0".to_string(),
                next_hop: "0.0.0.0".to_string(),
                interface_index: 7,
                interface_alias: Some("Ethernet 2".to_string()),
                route_metric: 0,
                interface_metric: 25,
                total_metric: 25,
                is_ipv4: true,
            },
        ];

        let best = find_best_ipv4_default_route(&routes).expect("deve encontrar melhor rota");
        assert_eq!(best.next_hop, "192.168.100.1");
        assert_eq!(best.interface_index, 7);
        assert_eq!(best.total_metric, 25);
    }
}
