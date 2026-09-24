use rusqlite::Connection;
use tracing::info;

/// Executa as migrações de esquema criando tabelas e índices de forma idempotente.
pub fn run_migrations(conn: &mut Connection) -> Result<(), rusqlite::Error> {
    info!("Executando migrações no banco SQLite...");

    conn.execute_batch(
        "
        -- Tabela de Hosts monitorados
        CREATE TABLE IF NOT EXISTS hosts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            address TEXT NOT NULL UNIQUE,
            host_type TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- Tabela de Amostras de Latência (ICMP)
        CREATE TABLE IF NOT EXISTS latency_samples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            host_id INTEGER NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
            timestamp TEXT NOT NULL,
            latency_ms REAL NOT NULL,
            success INTEGER NOT NULL,
            error_type TEXT
        );

        -- Índices otimizados para consultas temporais e agrupamento por host
        CREATE INDEX IF NOT EXISTS idx_samples_timestamp ON latency_samples(timestamp);
        CREATE INDEX IF NOT EXISTS idx_samples_host_timestamp ON latency_samples(host_id, timestamp);

        -- Tabela de Eventos de Rede (Link up/down, Gateway changed, IP changed)
        CREATE TABLE IF NOT EXISTS network_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            interface_name TEXT NOT NULL,
            interface_type TEXT NOT NULL,
            event_type TEXT NOT NULL,
            details TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON network_events(timestamp);
        CREATE INDEX IF NOT EXISTS idx_events_type ON network_events(event_type);
        ",
    )?;

    info!("Migrações SQLite concluídas com sucesso.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::open_memory_connection;

    #[test]
    fn test_migrations_create_tables() {
        let mut conn = open_memory_connection().expect("deve abrir banco em memória");
        run_migrations(&mut conn).expect("deve executar migrações com sucesso");

        // Verifica existência das tabelas
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert!(tables.contains(&"hosts".to_string()));
        assert!(tables.contains(&"latency_samples".to_string()));
        assert!(tables.contains(&"network_events".to_string()));
    }
}
