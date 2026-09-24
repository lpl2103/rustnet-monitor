use crate::network::types::{AdapterType, NetworkAdapter};

/// Avalia se um adaptador é virtual ou físico utilizando uma estratégia multinível:
/// 1. Verificação de hardware NDIS (`HardwareInterface` bit).
/// 2. Verificação de conector físico (`ConnectorPresent` bit).
/// 3. Tipo de interface NDIS (Loopback, Tunnel, Virtual).
/// 4. Padrões conhecidos de nomes/descrições de fornecedores de software de virtualização e VPNs.
///
/// Retorna `(is_virtual: bool, reason: Option<String>)`.
pub fn classify_adapter(
    if_type: AdapterType,
    is_hardware: bool,
    connector_present: bool,
    friendly_name: &str,
    description: &str,
) -> (bool, Option<String>) {
    let name_lower = friendly_name.to_lowercase();
    let desc_lower = description.to_lowercase();

    // 1. Verificação por tipo de interface NDIS
    match if_type {
        AdapterType::Loopback => {
            return (true, Some("Interface de loopback local".to_string()));
        }
        AdapterType::Tunnel => {
            return (
                true,
                Some("Interface de túnel / encapsulamento".to_string()),
            );
        }
        AdapterType::Virtual => {
            return (
                true,
                Some("Interface virtual proprietária NDIS".to_string()),
            );
        }
        _ => {}
    }

    // 2. Assinaturas conhecidas de softwares de virtualização e VPNs
    let vendor_checks: &[(&[&str], &str)] = &[
        (&["vmware", "vmnet"], "Adaptador virtual VMware"),
        (&["virtualbox", "vbox"], "Adaptador virtual VirtualBox"),
        (
            &["hyper-v", "vethernet"],
            "Adaptador virtual Microsoft Hyper-V",
        ),
        (&["tailscale"], "Túnel VPN Tailscale"),
        (&["radmin"], "Adaptador VPN Radmin"),
        (&["wireguard"], "Túnel VPN WireGuard"),
        (&["zerotier"], "Adaptador de rede ZeroTier"),
        (&["fortinet", "fortissl"], "Adaptador VPN Fortinet"),
        (
            &["openvpn", "tap-windows", "wintun"],
            "Adaptador virtual OpenVPN/TAP/Wintun",
        ),
        (&["hamachi"], "Adaptador VPN LogMeIn Hamachi"),
        (&["docker"], "Adaptador de rede de contêiner Docker"),
        (&["wsl"], "Adaptador virtual WSL (Linux)"),
        (&["npcap", "winpcap"], "Adaptador de captura Npcap/WinPcap"),
        (&["teredo"], "Túnel IPv6 Microsoft Teredo"),
        (&["isatap"], "Túnel Microsoft ISATAP"),
        (&["bluetooth"], "Conexão de rede Bluetooth"),
    ];

    for (patterns, reason) in vendor_checks {
        for pattern in *patterns {
            if name_lower.contains(pattern) || desc_lower.contains(pattern) {
                return (true, Some(reason.to_string()));
            }
        }
    }

    // 3. Verificação de hardware NDIS nativo da Windows API
    if !is_hardware && !connector_present {
        return (
            true,
            Some(
                "Adaptador de software emulado (HardwareInterface=0, ConnectorPresent=0)"
                    .to_string(),
            ),
        );
    }

    if !is_hardware {
        return (
            true,
            Some("Adaptador sem hardware físico NDIS (HardwareInterface=0)".to_string()),
        );
    }

    // Se chegou até aqui, é considerado uma interface física legítima (Ethernet ou Wi-Fi)
    (false, None)
}

/// Aplica a classificação em uma lista de adaptadores e atualiza os campos `is_virtual` e `virtual_reason`.
#[allow(dead_code)]
pub fn filter_and_classify_adapters(adapters: &mut [NetworkAdapter]) {
    for adapter in adapters.iter_mut() {
        let (is_virt, reason) = classify_adapter(
            adapter.if_type,
            adapter.is_hardware,
            adapter.connector_present,
            &adapter.friendly_name,
            &adapter.description,
        );
        adapter.is_virtual = is_virt;
        adapter.virtual_reason = reason;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::types::AdapterType;

    #[test]
    fn test_physical_ethernet() {
        let (is_virt, reason) = classify_adapter(
            AdapterType::Ethernet,
            true,
            true,
            "Ethernet 2",
            "Realtek PCIe GbE Family Controller",
        );
        assert!(!is_virt);
        assert!(reason.is_none());
    }

    #[test]
    fn test_physical_wifi() {
        let (is_virt, reason) = classify_adapter(
            AdapterType::Wireless,
            true,
            true,
            "Wi-Fi",
            "Intel(R) Wi-Fi 6 AX200 160MHz",
        );
        assert!(!is_virt);
        assert!(reason.is_none());
    }

    #[test]
    fn test_radmin_vpn() {
        let (is_virt, reason) = classify_adapter(
            AdapterType::Ethernet,
            false,
            false,
            "Radmin VPN",
            "Famatech Radmin VPN Ethernet Adapter",
        );
        assert!(is_virt);
        assert!(reason.unwrap().contains("Radmin"));
    }

    #[test]
    fn test_vmware_adapter() {
        let (is_virt, reason) = classify_adapter(
            AdapterType::Ethernet,
            false,
            false,
            "VMware Network Adapter VMnet1",
            "VMware Virtual Ethernet Adapter for VMnet1",
        );
        assert!(is_virt);
        assert!(reason.unwrap().contains("VMware"));
    }

    #[test]
    fn test_tailscale_adapter() {
        let (is_virt, reason) = classify_adapter(
            AdapterType::Tunnel,
            false,
            false,
            "Tailscale",
            "Tailscale Tunnel",
        );
        assert!(is_virt);
        assert!(reason.is_some());
    }

    #[test]
    fn test_loopback() {
        let (is_virt, reason) = classify_adapter(
            AdapterType::Loopback,
            false,
            false,
            "Loopback Pseudo-Interface 1",
            "Software Loopback Interface 1",
        );
        assert!(is_virt);
        assert!(reason.unwrap().contains("loopback"));
    }
}
