use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::database::connection::open_optimized_connection;
use crate::database::migrations::run_migrations;
use crate::database::models::{LatencySampleRecord, NetworkEventRecord};
use crate::database::repository::{
    cleanup_old_records, clear_all_samples, current_timestamp_iso, get_or_create_host,
    insert_latency_samples_batch, insert_network_event,
};

pub enum DbCommand {
    RecordSample {
        host_name: String,
        address: String,
        host_type: String,
        latency_ms: f64,
        success: bool,
        error_type: Option<String>,
    },
    RecordEvent(NetworkEventRecord),
    ClearSamples,
    #[allow(dead_code)]
    Cleanup,
    Flush,
    Stop,
}

/// Manipulador thread-safe para envio não-bloqueante de comandos para o Database Worker.
#[derive(Clone)]
pub struct DatabaseHandle {
    sender: Sender<DbCommand>,
}

impl DatabaseHandle {
    pub fn record_sample(
        &self,
        host_name: String,
        address: String,
        host_type: String,
        latency_ms: f64,
        success: bool,
        error_type: Option<String>,
    ) {
        let _ = self.sender.send(DbCommand::RecordSample {
            host_name,
            address,
            host_type,
            latency_ms,
            success,
            error_type,
        });
    }

    pub fn record_event(
        &self,
        interface_name: String,
        interface_type: String,
        event_type: String,
        details: Option<String>,
    ) {
        let event = NetworkEventRecord {
            id: None,
            timestamp: current_timestamp_iso(),
            interface_name,
            interface_type,
            event_type,
            details,
        };
        let _ = self.sender.send(DbCommand::RecordEvent(event));
    }

    #[allow(dead_code)]
    pub fn flush(&self) {
        let _ = self.sender.send(DbCommand::Flush);
    }

    pub fn clear_samples(&self) {
        let _ = self.sender.send(DbCommand::ClearSamples);
    }

    pub fn stop(&self) {
        let _ = self.sender.send(DbCommand::Stop);
    }
}

/// Inicia o Database Worker em uma thread dedicada.
/// Executa as migrações, a limpeza de retenção inicial e processa gravações em lote sem travar a aplicação.
pub fn start_database_worker(
    db_path: PathBuf,
    retention_days: u32,
) -> Result<(DatabaseHandle, thread::JoinHandle<()>), rusqlite::Error> {
    // 1. Abre conexão inicial para validar migrações antes de spawnar a thread
    {
        let mut test_conn = open_optimized_connection(&db_path)?;
        run_migrations(&mut test_conn)?;
    }

    let (tx, rx) = channel::<DbCommand>();
    let handle = DatabaseHandle { sender: tx };

    let worker_thread = thread::spawn(move || {
        run_worker_loop(db_path, rx, retention_days);
    });

    Ok((handle, worker_thread))
}

fn run_worker_loop(db_path: impl AsRef<Path>, rx: Receiver<DbCommand>, retention_days: u32) {
    let mut conn = match open_optimized_connection(&db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Erro fatal ao abrir conexão com SQLite no worker: {}", e);
            return;
        }
    };

    info!("Database Worker ativo e pronto para gravações assíncronas.");

    // Executa limpeza inicial de retenção
    if retention_days > 0 {
        match cleanup_old_records(&conn, retention_days) {
            Ok((s, e)) => {
                if s > 0 || e > 0 {
                    info!(
                        "Limpeza de retenção: {} amostras e {} eventos antigos removidos.",
                        s, e
                    );
                }
            }
            Err(e) => warn!("Falha ao executar limpeza de retenção inicial: {}", e),
        }
    }

    let mut host_cache: HashMap<String, i64> = HashMap::new();
    let mut pending_samples: Vec<LatencySampleRecord> = Vec::with_capacity(32);
    let mut last_flush = Instant::now();
    let mut last_cleanup = Instant::now();
    let flush_interval = Duration::from_millis(1000);
    let cleanup_interval = Duration::from_secs(3600); // 1 hora

    loop {
        // Aguarda comando com timeout para realizar flush periódico
        let cmd = match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                debug!("Canal do Database Worker desconectado, finalizando.");
                break;
            }
        };

        if let Some(command) = cmd {
            match command {
                DbCommand::RecordSample {
                    host_name,
                    address,
                    host_type,
                    latency_ms,
                    success,
                    error_type,
                } => {
                    // Resolve ou cria host no banco (com cache em memória para evitar queries repetidas)
                    let host_id = match host_cache.get(&address) {
                        Some(&id) => id,
                        None => match get_or_create_host(&conn, &host_name, &address, &host_type) {
                            Ok(id) => {
                                host_cache.insert(address.clone(), id);
                                id
                            }
                            Err(e) => {
                                error!("Falha ao obter host_id para '{}': {}", address, e);
                                continue;
                            }
                        },
                    };

                    pending_samples.push(LatencySampleRecord {
                        id: None,
                        host_id,
                        timestamp: current_timestamp_iso(),
                        latency_ms,
                        success,
                        error_type,
                    });

                    // Flush se atingir capacidade de lote
                    if pending_samples.len() >= 20 {
                        flush_batch(&mut conn, &mut pending_samples);
                        last_flush = Instant::now();
                    }
                }
                DbCommand::RecordEvent(event) => {
                    if let Err(e) = insert_network_event(&conn, &event) {
                        error!("Falha ao persistir evento de rede: {}", e);
                    }
                }
                DbCommand::ClearSamples => {
                    pending_samples.clear();
                    match clear_all_samples(&conn) {
                        Ok(count) => {
                            info!(
                                "Database: {} amostras de latência excluídas com sucesso.",
                                count
                            );
                        }
                        Err(e) => {
                            error!("Database: Falha ao excluir amostras: {}", e);
                        }
                    }
                }
                DbCommand::Cleanup => {
                    let _ = cleanup_old_records(&conn, retention_days);
                }
                DbCommand::Flush => {
                    flush_batch(&mut conn, &mut pending_samples);
                    last_flush = Instant::now();
                }
                DbCommand::Stop => {
                    info!("Comando de encerramento recebido no Database Worker.");
                    break;
                }
            }
        }

        // Flush por tempo decorrido
        if !pending_samples.is_empty() && last_flush.elapsed() >= flush_interval {
            flush_batch(&mut conn, &mut pending_samples);
            last_flush = Instant::now();
        }

        // Limpeza periódica de retenção (a cada 1 hora)
        if retention_days > 0 && last_cleanup.elapsed() >= cleanup_interval {
            let _ = cleanup_old_records(&conn, retention_days);
            last_cleanup = Instant::now();
        }
    }

    // Flush final antes de encerrar a thread
    flush_batch(&mut conn, &mut pending_samples);
    info!("Database Worker finalizado com sucesso.");
}

fn flush_batch(conn: &mut rusqlite::Connection, samples: &mut Vec<LatencySampleRecord>) {
    if samples.is_empty() {
        return;
    }

    match insert_latency_samples_batch(conn, samples) {
        Ok(count) => {
            debug!("Batch de {} amostras persistido com sucesso.", count);
            samples.clear();
        }
        Err(e) => {
            error!("Erro ao gravar lote de amostras no SQLite: {}", e);
        }
    }
}
