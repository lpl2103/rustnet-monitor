//! # Teste de Bufferbloat e Latência sob Carga (`bufferbloat.rs`)
//!
//! Avalia o impacto na latência da rede quando o enlace é submetido
//! a tráfego concorrente (Loaded Latency vs Unloaded Latency).

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::network::icmp::ping_ipv4;

#[derive(Debug, Clone, PartialEq)]
pub struct BufferbloatResult {
    pub unloaded_avg_ms: f64,
    pub loaded_avg_ms: f64,
    pub delta_ms: f64,
    pub grade: &'static str,
    pub description: &'static str,
}

impl BufferbloatResult {
    pub fn calculate(unloaded_avg: f64, loaded_avg: f64) -> Self {
        let delta = (loaded_avg - unloaded_avg).max(0.0);
        let (grade, description) = if delta <= 5.0 {
            ("A+", "Excelente! Gerenciamento inteligente de fila ativo (ex: SQM, CAKE, fq_codel).")
        } else if delta <= 30.0 {
            ("A", "Bom. Retenção mínima de buffer sob tráfego intenso.")
        } else if delta <= 65.0 {
            ("B", "Moderado. Aceitável para navegação, com pequena oscilação em jogos/chamadas.")
        } else if delta <= 200.0 {
            ("C", "Ruim. O tráfego concorrente gera atraso perceptível nas chamadas de voz/vídeo.")
        } else {
            ("F", "Crítico! Bufferbloat severo. Downloads causam picos extremos de ping.")
        };

        Self {
            unloaded_avg_ms: unloaded_avg,
            loaded_avg_ms: loaded_avg,
            delta_ms: delta,
            grade,
            description,
        }
    }
}

/// Executa um teste de Bufferbloat de aproximadamente 8 segundos.
/// `progress_cb`: callback para reportar progresso de 0.0 a 1.0.
pub fn run_bufferbloat_test<F>(target: Ipv4Addr, progress_cb: F) -> BufferbloatResult
where
    F: Fn(f32, &str),
{
    // Fase 1: Latência Ociosa (Unloaded) - 8 pings espaçados (~2 segundos)
    progress_cb(0.1, "Medindo latência ociosa de referência...");
    let mut unloaded_samples = Vec::new();

    for i in 0..8 {
        let res = ping_ipv4(target, Duration::from_millis(800));
        if res.rtt_ms > 0.0 {
            unloaded_samples.push(res.rtt_ms);
        }
        progress_cb(0.1 + (i as f32 * 0.025), "Medindo latência ociosa...");
        std::thread::sleep(Duration::from_millis(150));
    }

    let unloaded_avg = if !unloaded_samples.is_empty() {
        unloaded_samples.iter().sum::<f64>() / unloaded_samples.len() as f64
    } else {
        20.0
    };

    // Fase 2: Geração de Carga + Medição Concorrente (~5 segundos)
    progress_cb(0.35, "Iniciando tráfego de carga concorrente via CDN...");
    let stop_load = Arc::new(AtomicBool::new(false));
    let stop_clone = stop_load.clone();

    // Dispara thread de download em streaming para saturar brevemente a fila
    let load_handle = std::thread::spawn(move || {
        let agent = ureq::builder()
            .timeout(Duration::from_secs(6))
            .build();

        let urls = [
            "https://speed.cloudflare.com/__down?bytes=25000000",
            "https://proof.ovh.net/files/10Mb.dat",
            "https://1.1.1.1/cdn-cgi/trace",
        ];

        for url in urls {
            if stop_clone.load(Ordering::Relaxed) {
                break;
            }
            if let Ok(resp) = agent.get(url).call() {
                let mut reader = resp.into_reader();
                let mut buf = [0u8; 16384];
                while !stop_clone.load(Ordering::Relaxed) {
                    if let Ok(n) = std::io::Read::read(&mut reader, &mut buf) {
                        if n == 0 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    });

    let mut loaded_samples = Vec::new();
    let load_start = Instant::now();

    while load_start.elapsed() < Duration::from_secs(5) {
        let progress = 0.4 + (load_start.elapsed().as_secs_f32() / 5.0) * 0.55;
        progress_cb(progress.min(0.95), "Medindo latência sob tráfego de carga...");

        let res = ping_ipv4(target, Duration::from_millis(800));
        if res.rtt_ms > 0.0 {
            loaded_samples.push(res.rtt_ms);
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    stop_load.store(true, Ordering::Relaxed);
    let _ = load_handle.join();

    let loaded_avg = if !loaded_samples.is_empty() {
        loaded_samples.iter().sum::<f64>() / loaded_samples.len() as f64
    } else {
        unloaded_avg
    };

    progress_cb(1.0, "Teste concluído!");
    BufferbloatResult::calculate(unloaded_avg, loaded_avg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bufferbloat_grading() {
        let res_a_plus = BufferbloatResult::calculate(10.0, 14.0);
        assert_eq!(res_a_plus.grade, "A+");

        let res_a = BufferbloatResult::calculate(10.0, 30.0);
        assert_eq!(res_a.grade, "A");

        let res_f = BufferbloatResult::calculate(10.0, 250.0);
        assert_eq!(res_f.grade, "F");
    }
}
