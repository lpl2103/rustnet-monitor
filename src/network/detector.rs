use thiserror::Error;
use tracing::{debug, info, warn};

use crate::network::adapters::{AdapterError, get_network_adapters};
use crate::network::routes::{
    RouteError, find_best_ipv4_default_route, find_best_ipv6_default_route, get_default_routes,
};
use crate::network::types::{NetworkAdapter, NetworkDiagnostic, OperStatus, RouteEntry};

#[derive(Error, Debug)]
pub enum DetectorError {
    #[error("Erro ao obter adaptadores de rede: {0}")]
    Adapter(#[from] AdapterError),
    #[error("Erro ao obter tabela de rotas: {0}")]
    Route(#[from] RouteError),
}

/// Orquestrador de detecção de rede.
/// Analisa adaptadores, tabela de rotas, identifica interfaces físicas e filtra adaptadores virtuais.
pub fn inspect_network() -> Result<NetworkDiagnostic, DetectorError> {
    debug!("Iniciando varredura nativa de rede do Windows...");

    let adapters = get_network_adapters()?;
    let mut routes = get_default_routes().unwrap_or_default();

    // Vincula o nome amigável do adaptador às entradas da tabela de rotas
    for route in routes.iter_mut() {
        if let Some(adapter) = adapters
            .iter()
            .find(|a| a.if_index == route.interface_index)
        {
            route.interface_alias = Some(adapter.friendly_name.clone());
        }
    }

    let default_route_ipv4 = find_best_ipv4_default_route(&routes).cloned();
    let default_route_ipv6 = find_best_ipv6_default_route(&routes).cloned();

    // Separação de interfaces virtuais ignoradas
    let mut ignored_virtual_adapters = Vec::new();
    for adapter in &adapters {
        if adapter.is_virtual {
            ignored_virtual_adapters.push(adapter.clone());
        }
    }

    // Seleção da interface física ativa
    let active_physical_interface =
        select_active_physical_interface(&adapters, &default_route_ipv4);

    if let Some(ref iface) = active_physical_interface {
        info!(
            "Interface física ativa detectada: \"{}\" ({}) - Status: {}",
            iface.friendly_name, iface.description, iface.oper_status
        );
    } else {
        warn!("Nenhuma interface física ativa com rota válida para a Internet foi encontrada!");
    }

    Ok(NetworkDiagnostic {
        active_physical_interface,
        default_route_ipv4,
        default_route_ipv6,
        all_adapters: adapters,
        ignored_virtual_adapters,
    })
}

/// Determina a interface física ativa utilizando a rota padrão e validações de hardware NDIS.
fn select_active_physical_interface(
    adapters: &[NetworkAdapter],
    default_route_ipv4: &Option<RouteEntry>,
) -> Option<NetworkAdapter> {
    // 1. Tenta identificar pelo índice da melhor rota padrão IPv4
    if let Some(route) = default_route_ipv4 {
        let matching_adapter = adapters
            .iter()
            .find(|a| a.if_index == route.interface_index);
        if let Some(adapter) = matching_adapter {
            // Se o adaptador da rota padrão for físico e estiver conectado, é o candidato perfeito
            if !adapter.is_virtual && adapter.oper_status == OperStatus::Up {
                let mut chosen = adapter.clone();
                // Garante que o gateway da rota padrão esteja listado
                if !chosen.gateway_addresses.contains(&route.next_hop)
                    && route.next_hop != "0.0.0.0"
                {
                    chosen.gateway_addresses.insert(0, route.next_hop.clone());
                }
                return Some(chosen);
            }
        }
    }

    // 2. Se a rota padrão apontava para uma interface virtual (ex: VPN ativa)
    // ou se não foi possível casar o índice, localizamos a interface física uplink
    // que esteja conectada (Up), possua hardware real e possua endereço IP configurado.
    adapters
        .iter()
        .filter(|a| {
            !a.is_virtual && a.oper_status == OperStatus::Up && !a.ipv4_addresses.is_empty()
        })
        .max_by_key(|a| a.transmit_link_speed.max(a.receive_link_speed))
        .cloned()
}
