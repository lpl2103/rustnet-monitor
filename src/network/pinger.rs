use std::net::{Ipv4Addr, ToSocketAddrs};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;
use tracing::{debug, info};

use crate::config::settings::{AppConfig, HostConfig};
use crate::network::icmp::{PingResult, PingStatus, ping_ipv4};
use crate::network::stats::HostStats;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PingUpdate {
    pub host_index: usize,
    pub host_name: String,
    pub result: PingResult,
    pub stats: HostStats,
}

/// Resolve o destino configurado para um endereço IPv4 válido.
pub fn resolve_target(target_str: &str, dynamic_gateway: Option<Ipv4Addr>) -> Option<Ipv4Addr> {
    if target_str.eq_ignore_ascii_case("auto") {
        return dynamic_gateway;
    }

    if let Ok(ipv4) = Ipv4Addr::from_str(target_str) {
        return Some(ipv4);
    }

    // Tenta resolução DNS para hostnames (ex: "google.com")
    let socket_str = format!("{}:80", target_str);
    if let Ok(iter) = socket_str.to_socket_addrs() {
        for addr in iter {
            if let std::net::SocketAddr::V4(v4) = addr {
                return Some(*v4.ip());
            }
        }
    }

    None
}

/// Comandos de controle para a thread do pinger.
#[derive(Debug, Clone)]
pub enum PingerCommand {
    ResetStats,
}

/// Orquestrador de testes de conectividade periódicos em background.
pub struct PingerService {
    config: AppConfig,
    dynamic_gateway: Option<Ipv4Addr>,
    hosts: Vec<(HostConfig, HostStats)>,
    #[allow(dead_code)]
    stop_signal: Arc<AtomicBool>,
}

impl PingerService {
    pub fn new(config: AppConfig, detected_gateway: Option<Ipv4Addr>) -> Self {
        let mut hosts = Vec::new();

        for host_cfg in &config.hosts {
            if host_cfg.enabled {
                let stats = HostStats::new(host_cfg.name.clone(), host_cfg.address.clone());
                hosts.push((host_cfg.clone(), stats));
            }
        }

        Self {
            config,
            dynamic_gateway: detected_gateway,
            hosts,
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Atualiza o IP do gateway padrão dinâmico se tiver ocorrido mudança.
    #[allow(dead_code)]
    pub fn update_gateway(&mut self, new_gw: Option<Ipv4Addr>) {
        self.dynamic_gateway = new_gw;
    }

    /// Executa uma única rodada de pings síncrona sobre todos os hosts ativos.
    pub fn probe_all(&mut self) -> Vec<PingUpdate> {
        let timeout = Duration::from_millis(self.config.monitoring.timeout_ms);
        let mut updates = Vec::new();

        for (idx, (cfg, stats)) in self.hosts.iter_mut().enumerate() {
            let target_ip = if cfg.is_dynamic_gateway {
                self.dynamic_gateway
            } else {
                resolve_target(&cfg.address, self.dynamic_gateway)
            };

            let result = if let Some(ip) = target_ip {
                ping_ipv4(ip, timeout)
            } else {
                PingResult {
                    target_ip: Ipv4Addr::UNSPECIFIED,
                    rtt_ms: 0.0,
                    status: PingStatus::Error,
                    error_message: Some(format!("Falha ao resolver endereço '{}'", cfg.address)),
                }
            };

            stats.record_sample(result.clone());

            updates.push(PingUpdate {
                host_index: idx,
                host_name: cfg.name.clone(),
                result,
                stats: stats.clone(),
            });
        }

        updates
    }

    /// Obtém referência para as estatísticas atuais de todos os hosts monitorados.
    pub fn current_stats(&self) -> Vec<HostStats> {
        self.hosts.iter().map(|(_, s)| s.clone()).collect()
    }

    /// Inicia a thread de monitoramento contínuo em background.
    /// Retorna um canal de dados, um canal de comandos, um sinal de parada e o manipulador da thread.
    #[allow(dead_code)]
    pub fn start_worker(
        mut self,
    ) -> (
        Receiver<PingUpdate>,
        Sender<PingerCommand>,
        Arc<AtomicBool>,
        thread::JoinHandle<()>,
    ) {
        let (tx, rx) = channel::<PingUpdate>();
        let (cmd_tx, cmd_rx) = channel::<PingerCommand>();
        let stop_signal = self.stop_signal.clone();
        let thread_stop = stop_signal.clone();
        let interval = Duration::from_secs(self.config.monitoring.interval_secs.max(1));

        info!(
            "Iniciando monitoramento ICMP contínuo (intervalo: {}s)...",
            self.config.monitoring.interval_secs
        );

        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                // Processa eventuais comandos pendentes antes de pingar
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        PingerCommand::ResetStats => {
                            info!("Pinger: reinicializando estatísticas acumuladas de todos os hosts...");
                            for (_, stats) in self.hosts.iter_mut() {
                                stats.reset();
                            }
                        }
                    }
                }

                let updates = self.probe_all();
                for update in updates {
                    if tx.send(update).is_err() {
                        debug!("Canal de atualizações fechado, encerrando worker ICMP.");
                        return;
                    }
                }

                // Espera pelo próximo intervalo com checagem de parada e comandos a cada 100ms
                let steps = (interval.as_millis() / 100).max(1);
                for _ in 0..steps {
                    if thread_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        match cmd {
                            PingerCommand::ResetStats => {
                                info!("Pinger: reinicializando estatísticas acumuladas de todos os hosts...");
                                for (_, stats) in self.hosts.iter_mut() {
                                    stats.reset();
                                }
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
            info!("Worker de monitoramento ICMP finalizado.");
        });

        (rx, cmd_tx, stop_signal, handle)
    }
}
