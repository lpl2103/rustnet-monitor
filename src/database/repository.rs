use rusqlite::{Connection, OptionalExtension, params};
use std::time::SystemTime;

use crate::database::models::{HostRecord, LatencySampleRecord, NetworkEventRecord};

/// Obtém timestamp UTC atual formatado em padrão ISO 8601 (ex: "2026-09-23T21:15:00Z").
pub fn current_timestamp_iso() -> String {
    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Formatação UTC simplificada e sem dependências extras
    let days = secs / 86400;
    let rem_secs = secs % 86400;
    let hours = rem_secs / 3600;
    let mins = (rem_secs % 3600) / 60;
    let seconds = rem_secs % 60;

    // Algoritmo civil para ano/mês/dia a partir de dias desde 1970-01-01
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, mins, seconds
    )
}

/// Insere ou atualiza um host pelo endereço IP / Hostname único.
pub fn upsert_host(
    conn: &Connection,
    name: &str,
    address: &str,
    host_type: &str,
    enabled: bool,
) -> Result<i64, rusqlite::Error> {
    let now = current_timestamp_iso();

    conn.execute(
        "
        INSERT INTO hosts (name, address, host_type, enabled, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
        ON CONFLICT(address) DO UPDATE SET
            name = excluded.name,
            host_type = excluded.host_type,
            enabled = excluded.enabled,
            updated_at = excluded.updated_at
        ",
        params![name, address, host_type, if enabled { 1 } else { 0 }, now],
    )?;

    // Recupera o ID do host
    let id: i64 = conn.query_row(
        "SELECT id FROM hosts WHERE address = ?1",
        params![address],
        |row| row.get(0),
    )?;

    Ok(id)
}

/// Recupera um host pelo seu endereço.
pub fn get_host_by_address(
    conn: &Connection,
    address: &str,
) -> Result<Option<HostRecord>, rusqlite::Error> {
    conn.query_row(
        "SELECT id, name, address, host_type, enabled, created_at, updated_at FROM hosts WHERE address = ?1",
        params![address],
        |row| {
            Ok(HostRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                address: row.get(2)?,
                host_type: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
    .optional()
}

/// Busca o ID de um host ou o cria caso ainda não exista.
pub fn get_or_create_host(
    conn: &Connection,
    name: &str,
    address: &str,
    host_type: &str,
) -> Result<i64, rusqlite::Error> {
    if let Some(host) = get_host_by_address(conn, address)? {
        Ok(host.id)
    } else {
        upsert_host(conn, name, address, host_type, true)
    }
}

/// Recupera todos os hosts cadastrados.
#[allow(dead_code)]
pub fn get_all_hosts(conn: &Connection) -> Result<Vec<HostRecord>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, address, host_type, enabled, created_at, updated_at FROM hosts ORDER BY id ASC",
    )?;

    let iter = stmt.query_map([], |row| {
        Ok(HostRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            address: row.get(2)?,
            host_type: row.get(3)?,
            enabled: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;

    let mut hosts = Vec::new();
    for host in iter {
        hosts.push(host?);
    }
    Ok(hosts)
}

/// Insere uma amostra de latência individual.
#[allow(dead_code)]
pub fn insert_latency_sample(
    conn: &Connection,
    host_id: i64,
    timestamp: &str,
    latency_ms: f64,
    success: bool,
    error_type: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "
        INSERT INTO latency_samples (host_id, timestamp, latency_ms, success, error_type)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        params![
            host_id,
            timestamp,
            latency_ms,
            if success { 1 } else { 0 },
            error_type
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Insere múltiplas amostras de latência em lote sob uma única transação atômica.
pub fn insert_latency_samples_batch(
    conn: &mut Connection,
    samples: &[LatencySampleRecord],
) -> Result<usize, rusqlite::Error> {
    if samples.is_empty() {
        return Ok(0);
    }

    let tx = conn.transaction()?;
    let mut inserted = 0;

    {
        let mut stmt = tx.prepare_cached(
            "
            INSERT INTO latency_samples (host_id, timestamp, latency_ms, success, error_type)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
        )?;

        for sample in samples {
            stmt.execute(params![
                sample.host_id,
                sample.timestamp,
                sample.latency_ms,
                if sample.success { 1 } else { 0 },
                sample.error_type
            ])?;
            inserted += 1;
        }
    }

    tx.commit()?;
    Ok(inserted)
}

/// Registra um evento de rede (ex: LINK_UP, LINK_DOWN, GATEWAY_CHANGED, IP_CHANGED).
pub fn insert_network_event(
    conn: &Connection,
    event: &NetworkEventRecord,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "
        INSERT INTO network_events (timestamp, interface_name, interface_type, event_type, details)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        params![
            event.timestamp,
            event.interface_name,
            event.interface_type,
            event.event_type,
            event.details
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Executa a política de retenção excluindo registros mais antigos que `retention_days`.
/// Se `retention_days == 0`, a retenção é considerada ilimitada.
/// Retorna `(amostras_excluídas, eventos_excluídos)`.
pub fn cleanup_old_records(
    conn: &Connection,
    retention_days: u32,
) -> Result<(usize, usize), rusqlite::Error> {
    if retention_days == 0 {
        return Ok((0, 0));
    }

    let modifier = format!("-{} days", retention_days);

    let deleted_samples = conn.execute(
        "DELETE FROM latency_samples WHERE timestamp < datetime('now', ?1)",
        params![modifier],
    )?;

    let deleted_events = conn.execute(
        "DELETE FROM network_events WHERE timestamp < datetime('now', ?1)",
        params![modifier],
    )?;

    Ok((deleted_samples, deleted_events))
}

/// Exclui todas as amostras de latência registradas (limpeza / reset de métricas).
pub fn clear_all_samples(conn: &Connection) -> Result<usize, rusqlite::Error> {
    conn.execute("DELETE FROM latency_samples", [])
}

/// Recupera as amostras mais recentes para determinado host.
#[allow(dead_code)]
pub fn get_recent_samples_for_host(
    conn: &Connection,
    host_id: i64,
    limit: u32,
) -> Result<Vec<LatencySampleRecord>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "
        SELECT id, host_id, timestamp, latency_ms, success, error_type
        FROM latency_samples
        WHERE host_id = ?1
        ORDER BY id DESC
        LIMIT ?2
        ",
    )?;

    let iter = stmt.query_map(params![host_id, limit], |row| {
        Ok(LatencySampleRecord {
            id: Some(row.get(0)?),
            host_id: row.get(1)?,
            timestamp: row.get(2)?,
            latency_ms: row.get(3)?,
            success: row.get::<_, i32>(4)? != 0,
            error_type: row.get(5)?,
        })
    })?;

    let mut samples = Vec::new();
    for sample in iter {
        samples.push(sample?);
    }
    // Retorna em ordem cronológica crescente
    samples.reverse();
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::open_memory_connection;
    use crate::database::migrations::run_migrations;

    #[test]
    fn test_upsert_and_retrieve_host() {
        let mut conn = open_memory_connection().unwrap();
        run_migrations(&mut conn).unwrap();

        let id1 = upsert_host(&conn, "Google DNS", "8.8.8.8", "dns", true).unwrap();
        assert!(id1 > 0);

        let id2 = get_or_create_host(&conn, "Google DNS", "8.8.8.8", "dns").unwrap();
        assert_eq!(id1, id2);

        let host = get_host_by_address(&conn, "8.8.8.8").unwrap().unwrap();
        assert_eq!(host.name, "Google DNS");
        assert_eq!(host.address, "8.8.8.8");
        assert!(host.enabled);
    }

    #[test]
    fn test_batch_samples_insertion() {
        let mut conn = open_memory_connection().unwrap();
        run_migrations(&mut conn).unwrap();

        let host_id = upsert_host(&conn, "Cloudflare", "1.1.1.1", "dns", true).unwrap();
        let now = current_timestamp_iso();

        let samples = vec![
            LatencySampleRecord {
                id: None,
                host_id,
                timestamp: now.clone(),
                latency_ms: 12.5,
                success: true,
                error_type: None,
            },
            LatencySampleRecord {
                id: None,
                host_id,
                timestamp: now.clone(),
                latency_ms: 14.1,
                success: true,
                error_type: None,
            },
            LatencySampleRecord {
                id: None,
                host_id,
                timestamp: now,
                latency_ms: 1000.0,
                success: false,
                error_type: Some("TIMEOUT".to_string()),
            },
        ];

        let count = insert_latency_samples_batch(&mut conn, &samples).unwrap();
        assert_eq!(count, 3);

        let recent = get_recent_samples_for_host(&conn, host_id, 10).unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].latency_ms, 12.5);
        assert!(!recent[2].success);
    }

    #[test]
    fn test_network_event_insertion() {
        let mut conn = open_memory_connection().unwrap();
        run_migrations(&mut conn).unwrap();

        let event = NetworkEventRecord {
            id: None,
            timestamp: current_timestamp_iso(),
            interface_name: "Ethernet 2".to_string(),
            interface_type: "Ethernet".to_string(),
            event_type: "CONNECTED".to_string(),
            details: Some("Link Up @ 1 Gbps".to_string()),
        };

        let id = insert_network_event(&conn, &event).unwrap();
        assert!(id > 0);
    }
}
