mod config;
mod network;
mod utils;

use std::path::Path;
use tracing::{error, info};
use windows_sys::Win32::System::Console::{SetConsoleCP, SetConsoleOutputCP};

use crate::config::AppConfig;
use crate::network::inspect_network;
use crate::utils::logging::init_logging;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    // Garante que o console do Windows utilize UTF-8 nativo (Code Page 65001)
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
    }

    // 1. Carrega ou cria as configurações locais portáteis (config.toml)
    let config_path = Path::new("config.toml");
    let config = match AppConfig::load_or_create(config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!(
                "Aviso: Falha ao carregar config.toml (utilizando padrões): {}",
                e
            );
            AppConfig::default()
        }
    };

    // 2. Inicializa o sistema de telemetria e gravação de logs (logs/rustnet.log)
    let _log_guard = match init_logging("logs", &config.general.log_level) {
        Ok(guard) => Some(guard),
        Err(e) => {
            eprintln!("Aviso: Falha ao inicializar logging em arquivo: {}", e);
            None
        }
    };

    info!("Iniciando RustNet Monitor v{}...", VERSION);

    // 3. Inspeção nativa de interfaces e rotas do Windows
    let diagnostic = match inspect_network() {
        Ok(diag) => diag,
        Err(err) => {
            error!("Erro crítico na inspeção de rede: {}", err);
            eprintln!("Erro ao inspecionar a rede: {}", err);
            return;
        }
    };

    // 4. Renderização do relatório CLI formatado
    println!();
    println!("RustNet Monitor v{}", VERSION);
    println!();

    println!("Network Interface");
    println!("--------------------------------");
    if let Some(iface) = &diagnostic.active_physical_interface {
        println!("Name:        {}", iface.friendly_name);
        println!("Description: {}", iface.description);
        println!("Type:        {} (Física)", iface.if_type);
        println!("Status:      {}", iface.oper_status);
        println!(
            "IPv4:        {}",
            if iface.ipv4_addresses.is_empty() {
                "Não atribuído".to_string()
            } else {
                iface.ipv4_addresses.join(", ")
            }
        );
        if !iface.ipv6_addresses.is_empty() {
            println!("IPv6:        {}", iface.ipv6_addresses.join(", "));
        }
        println!(
            "Gateway:     {}",
            if iface.gateway_addresses.is_empty() {
                "Nenhum".to_string()
            } else {
                iface.gateway_addresses.join(", ")
            }
        );
        println!(
            "MAC:         {}",
            iface.mac_address.as_deref().unwrap_or("Não disponível")
        );
        println!("Speed:       {}", iface.formatted_speed());
        if !iface.dns_addresses.is_empty() {
            println!("DNS:         {}", iface.dns_addresses.join(", "));
        }
    } else {
        println!("Nenhuma interface física ativa identificada.");
    }
    println!();

    println!("Virtual Interfaces Ignored");
    println!("--------------------------------");
    if diagnostic.ignored_virtual_adapters.is_empty() {
        println!("Nenhuma interface virtual detectada.");
    } else {
        for virt in &diagnostic.ignored_virtual_adapters {
            let reason = virt
                .virtual_reason
                .as_deref()
                .unwrap_or("Classificado como virtual");
            println!("{:<30} ({})", virt.friendly_name, reason);
        }
    }
    println!();

    println!("Default Route");
    println!("--------------------------------");
    if let Some(route) = &diagnostic.default_route_ipv4 {
        let if_name = route.interface_alias.as_deref().unwrap_or("Desconhecida");
        println!(
            "Interface:   {} (Índice: {})",
            if_name, route.interface_index
        );
        println!("Gateway:     {}", route.next_hop);
        println!("RouteMetric: {}", route.route_metric);
        println!("IfMetric:    {}", route.interface_metric);
        println!("Metric:      {}", route.total_metric);
    } else {
        println!("Nenhuma rota padrão IPv4 ativa encontrada.");
    }

    if let Some(route6) = &diagnostic.default_route_ipv6 {
        println!();
        println!("Default Route IPv6");
        println!("--------------------------------");
        let if_name = route6.interface_alias.as_deref().unwrap_or("Desconhecida");
        println!(
            "Interface:   {} (Índice: {})",
            if_name, route6.interface_index
        );
        println!("Gateway:     {}", route6.next_hop);
        println!("Metric:      {}", route6.total_metric);
    }
    println!();
}
