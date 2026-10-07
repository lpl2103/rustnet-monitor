//! # Alertas Sonoros e Notificações do Sistema (`alerts.rs`)
//!
//! Emite sinais sonoros discretos e gerencia alertas de desconexão,
//! perda excessiva de pacotes e reconexão.

use std::time::{Duration, Instant};

#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBeep(utype: u32) -> i32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    ConnectionLost,
    HighPacketLoss,
    ConnectionRestored,
}

pub struct AlertManager {
    pub enabled: bool,
    last_alert_time: Option<Instant>,
    was_offline: bool,
}

impl Default for AlertManager {
    fn default() -> Self {
        Self {
            enabled: true,
            last_alert_time: None,
            was_offline: false,
        }
    }
}

impl AlertManager {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last_alert_time: None,
            was_offline: false,
        }
    }

    /// Dispara um sinal sonoro do Windows conforme a criticidade.
    pub fn play_sound(alert: AlertType) {
        unsafe {
            match alert {
                AlertType::ConnectionLost => {
                    // MB_ICONHAND (Som crítico de erro)
                    MessageBeep(0x00000010);
                }
                AlertType::HighPacketLoss => {
                    // MB_ICONEXCLAMATION (Som de advertência)
                    MessageBeep(0x00000030);
                }
                AlertType::ConnectionRestored => {
                    // MB_ICONASTERISK (Som de informação/sucesso)
                    MessageBeep(0x00000040);
                }
            }
        }
    }

    /// Avalia o status atual dos hosts e emite alertas com taxa de amostragem limitada (cooldown de 20s).
    pub fn check_and_alert(&mut self, is_online: bool, max_packet_loss_pct: f32) -> Option<AlertType> {
        if !self.enabled {
            return None;
        }

        let now = Instant::now();
        let in_cooldown = self
            .last_alert_time
            .map(|t| now.duration_since(t) < Duration::from_secs(20))
            .unwrap_or(false);

        // 1. Queda total de conexão
        if !is_online {
            if !self.was_offline {
                self.was_offline = true;
                self.last_alert_time = Some(now);
                Self::play_sound(AlertType::ConnectionLost);
                return Some(AlertType::ConnectionLost);
            }
            return None;
        }

        // 2. Reconexão restabelecida
        if is_online && self.was_offline {
            self.was_offline = false;
            self.last_alert_time = Some(now);
            Self::play_sound(AlertType::ConnectionRestored);
            return Some(AlertType::ConnectionRestored);
        }

        // 3. Alerta de perda de pacotes elevada (> 10%)
        if is_online && max_packet_loss_pct >= 10.0 && !in_cooldown {
            self.last_alert_time = Some(now);
            Self::play_sound(AlertType::HighPacketLoss);
            return Some(AlertType::HighPacketLoss);
        }

        None
    }
}
