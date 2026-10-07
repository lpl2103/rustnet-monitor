use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Erro de I/O ao acessar o arquivo de configuração: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro de sintaxe no arquivo TOML de configuração: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("Erro de serialização TOML: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub monitoring: MonitoringConfig,
    #[serde(default)]
    pub thresholds: LatencyThresholds,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub hosts: Vec<HostConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneralConfig {
    #[serde(default = "default_false")]
    pub start_with_windows: bool,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_true")]
    pub sound_alerts: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonitoringConfig {
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LatencyThresholds {
    #[serde(default = "default_green_ms")]
    pub green_max_ms: u32,
    #[serde(default = "default_yellow_ms")]
    pub yellow_max_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostConfig {
    pub name: String,
    pub address: String,
    #[serde(default = "default_custom_type")]
    pub host_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_false")]
    pub is_dynamic_gateway: bool,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "pt-BR".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_interval_secs() -> u64 {
    5
}

fn default_timeout_ms() -> u64 {
    1000
}

fn default_retry_count() -> u32 {
    1
}

fn default_green_ms() -> u32 {
    50
}

fn default_yellow_ms() -> u32 {
    100
}

fn default_db_path() -> String {
    "rustnet.db".to_string()
}

fn default_retention_days() -> u32 {
    30
}

fn default_custom_type() -> String {
    "custom".to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            start_with_windows: default_false(),
            minimize_to_tray: default_true(),
            sound_alerts: default_true(),
            theme: default_theme(),
            language: default_language(),
            log_level: default_log_level(),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            interval_secs: default_interval_secs(),
            timeout_ms: default_timeout_ms(),
            retry_count: default_retry_count(),
        }
    }
}

impl Default for LatencyThresholds {
    fn default() -> Self {
        Self {
            green_max_ms: default_green_ms(),
            yellow_max_ms: default_yellow_ms(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
            retention_days: default_retention_days(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            monitoring: MonitoringConfig::default(),
            thresholds: LatencyThresholds::default(),
            database: DatabaseConfig::default(),
            hosts: vec![
                HostConfig {
                    name: "Gateway Padrão".to_string(),
                    address: "auto".to_string(),
                    host_type: "gateway".to_string(),
                    enabled: true,
                    is_dynamic_gateway: true,
                },
                HostConfig {
                    name: "Google DNS".to_string(),
                    address: "8.8.8.8".to_string(),
                    host_type: "dns".to_string(),
                    enabled: true,
                    is_dynamic_gateway: false,
                },
                HostConfig {
                    name: "Cloudflare DNS".to_string(),
                    address: "1.1.1.1".to_string(),
                    host_type: "dns".to_string(),
                    enabled: true,
                    is_dynamic_gateway: false,
                },
            ],
        }
    }
}

impl AppConfig {
    /// Carrega as configurações a partir do arquivo fornecido.
    /// Se o arquivo não existir, cria o arquivo com as configurações padrão.
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            let default_config = Self::default();
            default_config.save(path)?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Salva a configuração atual no arquivo especificado em formato TOML.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_serialization() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).expect("deve serializar toml");
        let deserialized: AppConfig = toml::from_str(&serialized).expect("deve deserializar toml");
        assert_eq!(config, deserialized);
    }
}
