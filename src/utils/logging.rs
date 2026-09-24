use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Inicializa o sistema de telemetria e logging da aplicação.
///
/// Registra logs tanto no console quanto em arquivo rotativo na pasta especificada (`logs/rustnet.log`).
/// Retorna o `WorkerGuard` do appender assíncrono para garantir que os buffers de log sejam descarregados
/// ao encerrar a aplicação.
pub fn init_logging(
    log_dir: &str,
    default_level: &str,
) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    let log_path = Path::new(log_dir);
    if !log_path.exists() {
        std::fs::create_dir_all(log_path)?;
    }

    let file_appender = tracing_appender::rolling::never(log_dir, "rustnet.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_writer(non_blocking)
        .with_filter(env_filter.clone());

    let console_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_target(false)
        .compact()
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();

    Ok(guard)
}
