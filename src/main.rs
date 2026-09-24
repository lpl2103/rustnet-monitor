mod config;
mod database;
mod network;
mod utils;

use std::env;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tracing::{error, info};
use windows_sys::Win32::System::Console::{
    SetConsoleCP, SetConsoleCtrlHandler, SetConsoleOutputCP,
};

use crate::config::AppConfig;
use crate::database::{DatabaseHandle, start_database_worker};
use crate::network::icmp::PingStatus;
use crate::network::inspect_network;
use crate::network::pinger::{PingUpdate, PingerService};
use crate::network::stats::{HostStats, LatencyQuality};
use crate::network::types::NetworkDiagnostic;
use crate::utils::logging::init_logging;

const VERSION: &str = env!("CARGO_PKG_VERSION");

static RUNNING: AtomicBool = AtomicBool::new(true);

unsafe extern "system" fn console_ctrl_handler(ctrl_type: u32) -> i32 {
    // 0 = CTRL_C_EVENT, 1 = CTRL_BREAK_EVENT, 2 = CTRL_CLOSE_EVENT
    if ctrl_type <= 2 {
        RUNNING.store(false, Ordering::SeqCst);
        1 // TRUE: informa ao Windows que o evento foi tratado
    } else {
        0 // FALSE
    }
}

fn print_diagnostic(diagnostic: &NetworkDiagnostic) {
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
            println!("{:<32} ({})", virt.friendly_name, reason);
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

fn format_quality_badge(quality: LatencyQuality) -> String {
    match quality {
        LatencyQuality::Good => "\x1b[32m● BOM    \x1b[0m".to_string(),
        LatencyQuality::Fair => "\x1b[33m● MÉDIO  \x1b[0m".to_string(),
        LatencyQuality::Poor => "\x1b[31m● ALTO   \x1b[0m".to_string(),
        LatencyQuality::Offline => "\x1b[1;31m● OFFLINE\x1b[0m".to_string(),
    }
}

fn render_monitor_table(
    diagnostic: &NetworkDiagnostic,
    stats_list: &[HostStats],
    config: &AppConfig,
    cycle: u64,
) {
    let iface_desc = if let Some(iface) = &diagnostic.active_physical_interface {
        let ip = iface.ipv4_addresses.first().cloned().unwrap_or_default();
        let gw = iface.gateway_addresses.first().cloned().unwrap_or_default();
        format!(
            "{} ({}) | Gateway: {} | Link: {}",
            iface.friendly_name,
            ip,
            gw,
            iface.formatted_speed()
        )
    } else {
        "Nenhuma interface física ativa".to_string()
    };

    println!();
    println!(
        "┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐"
    );
    println!(
        "│ RustNet Monitor v{:<6} - Monitoramento em Tempo Real (Ciclo #{:<4} - Pressione Ctrl+C para sair)      │",
        VERSION, cycle
    );
    println!("│ Interface: {:<91} │", iface_desc);
    println!(
        "├──────────────────┬─────────────────┬──────────┬──────────┬──────────┬──────────┬─────────┬─────────┬─────────┬──────────┤"
    );
    println!(
        "│ Host             │ Endereço        │    Atual │    Média │      Mín │      Máx │  Jitter │ Perda % │ Enviados│ Status   │"
    );
    println!(
        "├──────────────────┼─────────────────┼──────────┼──────────┼──────────┼──────────┼─────────┼─────────┼─────────┼──────────┤"
    );

    for stat in stats_list {
        let addr_display = stat
            .resolved_ip
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| stat.target_str.clone());

        let cur_str = stat
            .last_rtt_ms
            .map(|r| format!("{:.0} ms", r))
            .unwrap_or_else(|| "-".to_string());

        let avg_str = stat
            .avg_rtt_ms
            .map(|r| format!("{:.0} ms", r))
            .unwrap_or_else(|| "-".to_string());

        let min_str = stat
            .min_rtt_ms
            .map(|r| format!("{:.0} ms", r))
            .unwrap_or_else(|| "-".to_string());

        let max_str = stat
            .max_rtt_ms
            .map(|r| format!("{:.0} ms", r))
            .unwrap_or_else(|| "-".to_string());

        let jitter_str = format!("{:.0} ms", stat.jitter_ms);
        let loss_str = format!("{:.1}%", stat.packet_loss_pct);
        let badge = format_quality_badge(stat.quality(&config.thresholds));

        println!(
            "│ {:<16} │ {:<15} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │ {:>7} │ {:>7} │ {:>7} │ {}│",
            stat.name,
            addr_display,
            cur_str,
            avg_str,
            min_str,
            max_str,
            jitter_str,
            loss_str,
            stat.sent_packets,
            badge
        );
    }

    println!(
        "└──────────────────┴─────────────────┴──────────┴──────────┴──────────┴──────────┴─────────┴─────────┴─────────┴──────────┘"
    );
}

fn persist_updates(db_handle: &Option<DatabaseHandle>, updates: &[PingUpdate]) {
    if let Some(db) = db_handle {
        for update in updates {
            let addr = update
                .stats
                .resolved_ip
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| update.stats.target_str.clone());

            let host_type = if update.host_name.to_lowercase().contains("gateway") {
                "gateway"
            } else if update.host_name.to_lowercase().contains("dns") {
                "dns"
            } else {
                "custom"
            };

            db.record_sample(
                update.host_name.clone(),
                addr,
                host_type.to_string(),
                update.result.rtt_ms,
                update.result.status == PingStatus::Success,
                update.result.error_message.clone(),
            );
        }
    }
}

fn main() {
    // 1. Configura UTF-8 nativo no console Windows
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
        SetConsoleCtrlHandler(Some(console_ctrl_handler), 1);
    }

    // 2. Carrega configurações locais portáteis (config.toml)
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

    // 3. Inicializa logging estruturado
    let _log_guard = match init_logging("logs", &config.general.log_level) {
        Ok(guard) => Some(guard),
        Err(e) => {
            eprintln!("Aviso: Falha ao inicializar logging em arquivo: {}", e);
            None
        }
    };

    info!("Iniciando RustNet Monitor v{}...", VERSION);

    // 4. Inicializa o Database Worker assíncrono (SQLite em rustnet.db)
    let db_path = PathBuf::from(&config.database.path);
    let (db_handle, db_thread) =
        match start_database_worker(db_path, config.database.retention_days) {
            Ok((handle, thread)) => (Some(handle), Some(thread)),
            Err(e) => {
                error!("Erro ao inicializar banco de dados SQLite: {}", e);
                eprintln!("Aviso: Falha ao iniciar persistência SQLite: {}", e);
                (None, None)
            }
        };

    // 5. Executa inspeção de rede nativa do Windows
    let diagnostic = match inspect_network() {
        Ok(diag) => diag,
        Err(err) => {
            error!("Erro crítico na inspeção de rede: {}", err);
            eprintln!("Erro ao inspecionar a rede: {}", err);
            return;
        }
    };

    // Registra evento de rede inicial no banco
    if let (Some(db), Some(iface)) = (&db_handle, &diagnostic.active_physical_interface) {
        db.record_event(
            iface.friendly_name.clone(),
            iface.if_type.to_string(),
            "CONNECTED".to_string(),
            Some(format!(
                "Link Speed: {}, IPv4: {}",
                iface.formatted_speed(),
                iface.ipv4_addresses.join(", ")
            )),
        );
    }

    let args: Vec<String> = env::args().collect();
    let diagnostic_only = args.iter().any(|a| a == "--diagnostic");
    let once_only = args.iter().any(|a| a == "--once");

    // Exibe o diagnóstico detalhado
    print_diagnostic(&diagnostic);

    if diagnostic_only {
        if let Some(ref db) = db_handle {
            db.stop();
        }
        if let Some(thread) = db_thread {
            let _ = thread.join();
        }
        return;
    }

    // Extrai o IP do gateway detectado para monitoramento dinâmico
    let detected_gateway = diagnostic
        .default_route_ipv4
        .as_ref()
        .and_then(|r| Ipv4Addr::from_str(&r.next_hop).ok())
        .or_else(|| {
            diagnostic
                .active_physical_interface
                .as_ref()
                .and_then(|iface| iface.gateway_addresses.first())
                .and_then(|gw| Ipv4Addr::from_str(gw).ok())
        });

    let mut pinger = PingerService::new(config.clone(), detected_gateway);

    if once_only {
        println!("Executando teste único de conectividade ICMP e persistência no banco...");
        let updates = pinger.probe_all();
        persist_updates(&db_handle, &updates);
        let stats = pinger.current_stats();
        render_monitor_table(&diagnostic, &stats, &config, 1);

        if let Some(ref db) = db_handle {
            db.stop();
        }
        if let Some(thread) = db_thread {
            let _ = thread.join();
        }
        return;
    }

    println!(
        "Iniciando monitoramento em tempo real (intervalo: {}s)...",
        config.monitoring.interval_secs
    );
    let interval = Duration::from_secs(config.monitoring.interval_secs.max(1));
    let mut cycle = 0;

    while RUNNING.load(Ordering::Relaxed) {
        cycle += 1;
        let updates = pinger.probe_all();
        persist_updates(&db_handle, &updates);

        let stats = pinger.current_stats();
        render_monitor_table(&diagnostic, &stats, &config, cycle);

        // Espera pelo próximo ciclo com checagem de Ctrl+C a cada 100ms
        let steps = (interval.as_millis() / 100).max(1);
        for _ in 0..steps {
            if !RUNNING.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    println!();
    println!("Finalizando persistência de dados no SQLite...");
    if let Some(ref db) = db_handle {
        db.stop();
    }
    if let Some(thread) = db_thread {
        let _ = thread.join();
    }

    println!("Monitoramento encerrado pelo usuário.");
    info!("RustNet Monitor finalizado.");
}
