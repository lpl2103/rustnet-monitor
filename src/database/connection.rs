use rusqlite::Connection;
use std::path::Path;
use tracing::debug;

/// Abre uma conexão com o SQLite aplicando as otimizações necessárias para alta performance:
/// - WAL mode (Write-Ahead Logging) para permitir leituras concorrentes sem bloqueio durante escritas.
/// - SYNCHRONOUS = NORMAL para minimizar gravações síncronas pesadas em disco.
/// - BUSY_TIMEOUT = 5000 para evitar erros temporários de bloqueio.
/// - FOREIGN_KEYS = ON para garantir a integridade referencial.
pub fn open_optimized_connection<P: AsRef<Path>>(path: P) -> Result<Connection, rusqlite::Error> {
    let path = path.as_ref();
    let conn = Connection::open(path)?;

    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA busy_timeout = 5000;
        PRAGMA foreign_keys = ON;
        ",
    )?;

    debug!("Conexão SQLite aberta com sucesso em: {:?}", path);
    Ok(conn)
}

/// Cria uma conexão em memória otimizada para a execução de testes unitários.
#[allow(dead_code)]
pub fn open_memory_connection() -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        ",
    )?;
    Ok(conn)
}
