use std::fmt;
use std::net::Ipv4Addr;
use std::time::Instant;

use crate::config::settings::LatencyThresholds;
use crate::network::icmp::{PingResult, PingStatus};

/// Classificação visual da qualidade da latência/conectividade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyQuality {
    Good,    // Verde (< green_max_ms)
    Fair,    // Amarelo (green_max_ms..yellow_max_ms)
    Poor,    // Vermelho (> yellow_max_ms)
    Offline, // Vermelho (Timeout ou Erro)
}

impl fmt::Display for LatencyQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatencyQuality::Good => write!(f, "BOM"),
            LatencyQuality::Fair => write!(f, "MÉDIO"),
            LatencyQuality::Poor => write!(f, "ALTO"),
            LatencyQuality::Offline => write!(f, "OFFLINE"),
        }
    }
}

/// Acumulador e calculador de estatísticas de conectividade para um host específico.
#[derive(Debug, Clone)]
pub struct HostStats {
    pub name: String,
    pub target_str: String,
    pub resolved_ip: Option<Ipv4Addr>,
    pub last_rtt_ms: Option<f64>,
    pub min_rtt_ms: Option<f64>,
    pub max_rtt_ms: Option<f64>,
    pub avg_rtt_ms: Option<f64>,
    pub jitter_ms: f64,
    pub sent_packets: u64,
    pub received_packets: u64,
    pub lost_packets: u64,
    pub packet_loss_pct: f64,
    pub last_status: PingStatus,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
    pub last_success_time: Option<Instant>,
    pub last_sample_time: Option<Instant>,
    total_rtt_sum: f64,
    previous_rtt: Option<f64>,
}

impl HostStats {
    pub fn new(name: String, target_str: String) -> Self {
        Self {
            name,
            target_str,
            resolved_ip: None,
            last_rtt_ms: None,
            min_rtt_ms: None,
            max_rtt_ms: None,
            avg_rtt_ms: None,
            jitter_ms: 0.0,
            sent_packets: 0,
            received_packets: 0,
            lost_packets: 0,
            packet_loss_pct: 0.0,
            last_status: PingStatus::Error,
            last_error: None,
            consecutive_failures: 0,
            last_success_time: None,
            last_sample_time: None,
            total_rtt_sum: 0.0,
            previous_rtt: None,
        }
    }

    /// Reinicia todas as métricas acumuladas deste host para o estado inicial.
    pub fn reset(&mut self) {
        self.last_rtt_ms = None;
        self.min_rtt_ms = None;
        self.max_rtt_ms = None;
        self.avg_rtt_ms = None;
        self.jitter_ms = 0.0;
        self.sent_packets = 0;
        self.received_packets = 0;
        self.lost_packets = 0;
        self.packet_loss_pct = 0.0;
        self.last_status = PingStatus::Error;
        self.last_error = None;
        self.consecutive_failures = 0;
        self.last_success_time = None;
        self.last_sample_time = None;
        self.total_rtt_sum = 0.0;
        self.previous_rtt = None;
    }

    /// Classifica a qualidade com base nos limites definidos na configuração.
    pub fn quality(&self, thresholds: &LatencyThresholds) -> LatencyQuality {
        match self.last_status {
            PingStatus::Success => {
                if let Some(rtt) = self.last_rtt_ms {
                    if rtt < thresholds.green_max_ms as f64 {
                        LatencyQuality::Good
                    } else if rtt <= thresholds.yellow_max_ms as f64 {
                        LatencyQuality::Fair
                    } else {
                        LatencyQuality::Poor
                    }
                } else {
                    LatencyQuality::Offline
                }
            }
            PingStatus::Timeout | PingStatus::Error => LatencyQuality::Offline,
        }
    }

    /// Registra uma nova amostra e recalcula métricas agregadas (mín, máx, média, perda e jitter RFC 3550).
    pub fn record_sample(&mut self, result: PingResult) {
        let now = Instant::now();
        self.last_sample_time = Some(now);
        self.sent_packets += 1;
        self.last_status = result.status;
        self.last_error = result.error_message;
        self.resolved_ip = Some(result.target_ip);

        match result.status {
            PingStatus::Success => {
                self.received_packets += 1;
                self.consecutive_failures = 0;
                self.last_success_time = Some(now);
                let rtt = result.rtt_ms;
                self.last_rtt_ms = Some(rtt);

                // Mínimo e Máximo
                self.min_rtt_ms = Some(match self.min_rtt_ms {
                    Some(cur_min) => cur_min.min(rtt),
                    None => rtt,
                });
                self.max_rtt_ms = Some(match self.max_rtt_ms {
                    Some(cur_max) => cur_max.max(rtt),
                    None => rtt,
                });

                // Média acumulada
                self.total_rtt_sum += rtt;
                self.avg_rtt_ms = Some(self.total_rtt_sum / self.received_packets as f64);

                // Cálculo do Jitter conforme RFC 3550:
                // D(i-1, i) = |RTT_i - RTT_{i-1}|
                // J_i = J_{i-1} + (|D(i-1, i)| - J_{i-1}) / 16.0
                if let Some(prev) = self.previous_rtt {
                    let diff = (rtt - prev).abs();
                    self.jitter_ms += (diff - self.jitter_ms) / 16.0;
                }
                self.previous_rtt = Some(rtt);
            }
            PingStatus::Timeout | PingStatus::Error => {
                self.lost_packets += 1;
                self.consecutive_failures += 1;
                // Em caso de perda, mantemos a última latência conhecida mas resetamos previous_rtt
                // para não distorcer o cálculo de jitter do próximo pacote recebido
                self.previous_rtt = None;
            }
        }

        // Percentual de perda de pacotes
        if self.sent_packets > 0 {
            self.packet_loss_pct = (self.lost_packets as f64 / self.sent_packets as f64) * 100.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jitter_rfc3550_calculation() {
        let mut stats = HostStats::new("Test".to_string(), "127.0.0.1".to_string());
        let ip = Ipv4Addr::new(127, 0, 0, 1);

        // Amostra 1: 10 ms -> Jitter = 0.0
        stats.record_sample(PingResult {
            target_ip: ip,
            rtt_ms: 10.0,
            status: PingStatus::Success,
            error_message: None,
        });
        assert_eq!(stats.jitter_ms, 0.0);

        // Amostra 2: 26 ms -> Diff = 16 ms. Jitter = 0 + (16 - 0) / 16 = 1.0
        stats.record_sample(PingResult {
            target_ip: ip,
            rtt_ms: 26.0,
            status: PingStatus::Success,
            error_message: None,
        });
        assert_eq!(stats.jitter_ms, 1.0);

        // Amostra 3: 10 ms -> Diff = 16 ms. Jitter = 1.0 + (16 - 1.0) / 16 = 1.9375
        stats.record_sample(PingResult {
            target_ip: ip,
            rtt_ms: 10.0,
            status: PingStatus::Success,
            error_message: None,
        });
        let expected = 1.0 + (16.0 - 1.0) / 16.0;
        assert!((stats.jitter_ms - expected).abs() < 1e-6);
    }

    #[test]
    fn test_packet_loss_and_aggregates() {
        let mut stats = HostStats::new("Test".to_string(), "127.0.0.1".to_string());
        let ip = Ipv4Addr::new(127, 0, 0, 1);

        // 3 sucessos
        for rtt in [10.0, 20.0, 30.0] {
            stats.record_sample(PingResult {
                target_ip: ip,
                rtt_ms: rtt,
                status: PingStatus::Success,
                error_message: None,
            });
        }

        // 1 timeout
        stats.record_sample(PingResult {
            target_ip: ip,
            rtt_ms: 1000.0,
            status: PingStatus::Timeout,
            error_message: None,
        });

        assert_eq!(stats.sent_packets, 4);
        assert_eq!(stats.received_packets, 3);
        assert_eq!(stats.lost_packets, 1);
        assert_eq!(stats.packet_loss_pct, 25.0);
        assert_eq!(stats.min_rtt_ms, Some(10.0));
        assert_eq!(stats.max_rtt_ms, Some(30.0));
        assert_eq!(stats.avg_rtt_ms, Some(20.0));
    }

    #[test]
    fn test_latency_quality_classification() {
        let thresholds = LatencyThresholds {
            green_max_ms: 50,
            yellow_max_ms: 100,
        };

        let mut stats = HostStats::new("Test".to_string(), "127.0.0.1".to_string());
        let ip = Ipv4Addr::new(127, 0, 0, 1);

        stats.record_sample(PingResult {
            target_ip: ip,
            rtt_ms: 20.0,
            status: PingStatus::Success,
            error_message: None,
        });
        assert_eq!(stats.quality(&thresholds), LatencyQuality::Good);

        stats.record_sample(PingResult {
            target_ip: ip,
            rtt_ms: 75.0,
            status: PingStatus::Success,
            error_message: None,
        });
        assert_eq!(stats.quality(&thresholds), LatencyQuality::Fair);

        stats.record_sample(PingResult {
            target_ip: ip,
            rtt_ms: 150.0,
            status: PingStatus::Success,
            error_message: None,
        });
        assert_eq!(stats.quality(&thresholds), LatencyQuality::Poor);

        stats.record_sample(PingResult {
            target_ip: ip,
            rtt_ms: 1000.0,
            status: PingStatus::Timeout,
            error_message: None,
        });
        assert_eq!(stats.quality(&thresholds), LatencyQuality::Offline);
    }
}
