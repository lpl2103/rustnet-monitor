use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostRecord {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub host_type: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatencySampleRecord {
    pub id: Option<i64>,
    pub host_id: i64,
    pub timestamp: String,
    pub latency_ms: f64,
    pub success: bool,
    pub error_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkEventRecord {
    pub id: Option<i64>,
    pub timestamp: String,
    pub interface_name: String,
    pub interface_type: String,
    pub event_type: String,
    pub details: Option<String>,
}
