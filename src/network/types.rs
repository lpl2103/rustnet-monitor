use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterType {
    Ethernet,
    Wireless,
    Loopback,
    Tunnel,
    Virtual,
    Other(u32),
}

impl fmt::Display for AdapterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterType::Ethernet => write!(f, "Ethernet"),
            AdapterType::Wireless => write!(f, "Wi-Fi"),
            AdapterType::Loopback => write!(f, "Loopback"),
            AdapterType::Tunnel => write!(f, "Túnel / VPN"),
            AdapterType::Virtual => write!(f, "Virtual"),
            AdapterType::Other(code) => write!(f, "Outro ({})", code),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperStatus {
    Up,
    Down,
    Testing,
    Unknown,
    Dormant,
    NotPresent,
    LowerLayerDown,
}

impl fmt::Display for OperStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperStatus::Up => write!(f, "Conectado (Up)"),
            OperStatus::Down => write!(f, "Desconectado (Down)"),
            OperStatus::Testing => write!(f, "Em teste"),
            OperStatus::Unknown => write!(f, "Desconhecido"),
            OperStatus::Dormant => write!(f, "Inativo"),
            OperStatus::NotPresent => write!(f, "Não presente"),
            OperStatus::LowerLayerDown => write!(f, "Camada inferior inativa"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkAdapter {
    pub adapter_name: String,  // GUID
    pub friendly_name: String, // Ex: "Ethernet 2"
    pub description: String,   // Ex: "Realtek PCIe GbE Family Controller"
    pub if_type: AdapterType,
    pub oper_status: OperStatus,
    pub if_index: u32,
    pub ipv6_if_index: u32,
    pub mac_address: Option<String>,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub gateway_addresses: Vec<String>,
    pub dns_addresses: Vec<String>,
    pub transmit_link_speed: u64, // bits por segundo
    pub receive_link_speed: u64,
    pub is_hardware: bool,       // HardwareInterface bit do NDIS
    pub connector_present: bool, // ConnectorPresent bit do NDIS
    pub is_virtual: bool,
    pub virtual_reason: Option<String>,
}

impl NetworkAdapter {
    /// Formata a velocidade do link de forma legível (ex: 1 Gbps, 100 Mbps).
    pub fn formatted_speed(&self) -> String {
        let speed = self.transmit_link_speed.max(self.receive_link_speed);
        if speed == 0 {
            "Desconhecido".to_string()
        } else if speed >= 1_000_000_000 {
            format!("{:.1} Gbps", speed as f64 / 1_000_000_000.0).replace(".0 Gbps", " Gbps")
        } else if speed >= 1_000_000 {
            format!("{:.0} Mbps", speed as f64 / 1_000_000.0)
        } else if speed >= 1_000 {
            format!("{:.0} Kbps", speed as f64 / 1_000.0)
        } else {
            format!("{} bps", speed)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntry {
    pub destination: String,
    pub next_hop: String,
    pub interface_index: u32,
    pub interface_alias: Option<String>,
    pub route_metric: u32,
    pub interface_metric: u32,
    pub total_metric: u32,
    pub is_ipv4: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct NetworkDiagnostic {
    pub active_physical_interface: Option<NetworkAdapter>,
    pub default_route_ipv4: Option<RouteEntry>,
    pub default_route_ipv6: Option<RouteEntry>,
    pub all_adapters: Vec<NetworkAdapter>,
    pub ignored_virtual_adapters: Vec<NetworkAdapter>,
}
