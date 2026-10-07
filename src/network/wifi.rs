//! # Telemetria de Conexão Wi-Fi (`wifi.rs`)
//!
//! Realiza a coleta de métricas da camada física sem fio (802.11)
//! utilizando a biblioteca nativa do Windows `wlanapi.dll`.

use std::ffi::c_void;
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq)]
pub struct WifiInfo {
    pub ssid: String,
    pub bssid: String,
    pub signal_quality_pct: u32,
    pub rssi_dbm: i32,
    pub phy_type: String,
    pub tx_rate_mbps: f64,
    pub rx_rate_mbps: f64,
}

impl WifiInfo {
    pub fn signal_badge(&self) -> &'static str {
        if self.signal_quality_pct >= 75 {
            "Excelente"
        } else if self.signal_quality_pct >= 50 {
            "Bom"
        } else if self.signal_quality_pct >= 25 {
            "Médio"
        } else {
            "Fraco"
        }
    }
}

// Definições de estruturas da Windows WLAN API
#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct WlanInterfaceInfo {
    interface_guid: Guid,
    str_interface_description: [u16; 256],
    is_state: u32, // 1 = connected
}

#[repr(C)]
struct WlanInterfaceInfoList {
    number_of_items: u32,
    index: u32,
    interface_info: [WlanInterfaceInfo; 1],
}

#[repr(C)]
struct Dot11Ssid {
    length: u32,
    ssid: [u8; 32],
}

#[repr(C)]
struct WlanAssociationAttributes {
    dot11_ssid: Dot11Ssid,
    dot11_bsstype: u32,
    dot11_bssid: [u8; 6],
    dot11_phy_type: u32,
    dot11_phy_index: u32,
    wlan_signal_quality: u32,
    rx_rate: u32,
    tx_rate: u32,
}

#[repr(C)]
struct WlanSecurityAttributes {
    security_enabled: i32,
    one_x_enabled: i32,
    dot11_auth_algo: u32,
    dot11_cipher_algo: u32,
}

#[repr(C)]
struct WlanConnectionAttributes {
    is_state: u32,
    wlan_connection_mode: u32,
    str_profile_name: [u16; 256],
    wlan_association_attributes: WlanAssociationAttributes,
    wlan_security_attributes: WlanSecurityAttributes,
}

type WlanOpenHandleFunc = unsafe extern "system" fn(u32, *const c_void, *mut u32, *mut *mut c_void) -> u32;
type WlanCloseHandleFunc = unsafe extern "system" fn(*mut c_void, *const c_void) -> u32;
type WlanEnumInterfacesFunc = unsafe extern "system" fn(*mut c_void, *const c_void, *mut *mut WlanInterfaceInfoList) -> u32;
type WlanQueryInterfaceFunc = unsafe extern "system" fn(
    *mut c_void,
    *const Guid,
    u32,
    *const c_void,
    *mut u32,
    *mut *mut c_void,
    *mut u32,
) -> u32;
type WlanFreeMemoryFunc = unsafe extern "system" fn(*mut c_void);

/// Consulta a telemetria do adaptador Wi-Fi atualmente conectado (se existente).
#[allow(clippy::manual_c_str_literals)]
pub fn query_wifi_telemetry() -> Option<WifiInfo> {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

    unsafe {
        let dll_name = b"wlanapi.dll\0";
        let module = LoadLibraryA(dll_name.as_ptr());
        if module.is_null() {
            debug!("wlanapi.dll não disponível no sistema.");
            return None;
        }

        let open_handle_ptr = GetProcAddress(module, b"WlanOpenHandle\0".as_ptr());
        let close_handle_ptr = GetProcAddress(module, b"WlanCloseHandle\0".as_ptr());
        let enum_interfaces_ptr = GetProcAddress(module, b"WlanEnumInterfaces\0".as_ptr());
        let query_interface_ptr = GetProcAddress(module, b"WlanQueryInterface\0".as_ptr());
        let free_memory_ptr = GetProcAddress(module, b"WlanFreeMemory\0".as_ptr());

        if open_handle_ptr.is_none()
            || close_handle_ptr.is_none()
            || enum_interfaces_ptr.is_none()
            || query_interface_ptr.is_none()
            || free_memory_ptr.is_none()
        {
            return None;
        }

        let wlan_open_handle: WlanOpenHandleFunc = std::mem::transmute(open_handle_ptr);
        let wlan_close_handle: WlanCloseHandleFunc = std::mem::transmute(close_handle_ptr);
        let wlan_enum_interfaces: WlanEnumInterfacesFunc = std::mem::transmute(enum_interfaces_ptr);
        let wlan_query_interface: WlanQueryInterfaceFunc = std::mem::transmute(query_interface_ptr);
        let wlan_free_memory: WlanFreeMemoryFunc = std::mem::transmute(free_memory_ptr);

        let mut client_version = 0u32;
        let mut handle: *mut c_void = std::ptr::null_mut();

        // Client version 2 para Windows Vista até 11
        let res = wlan_open_handle(2, std::ptr::null(), &mut client_version, &mut handle);
        if res != 0 || handle.is_null() {
            return None;
        }

        let mut iface_list_ptr: *mut WlanInterfaceInfoList = std::ptr::null_mut();
        let res_enum = wlan_enum_interfaces(handle, std::ptr::null(), &mut iface_list_ptr);
        if res_enum != 0 || iface_list_ptr.is_null() {
            wlan_close_handle(handle, std::ptr::null());
            return None;
        }

        let num_items = (*iface_list_ptr).number_of_items;
        let mut result_info: Option<WifiInfo> = None;

        let base_ptr = &(*iface_list_ptr).interface_info[0] as *const WlanInterfaceInfo;

        for i in 0..num_items {
            let iface = &*base_ptr.add(i as usize);
            // 1 = wlan_interface_state_connected
            if iface.is_state == 1 {
                let mut data_size = 0u32;
                let mut data_ptr: *mut c_void = std::ptr::null_mut();
                let mut opcode_val = 0u32;

                // Opcode 7 = wlan_intf_opcode_current_connection
                let q_res = wlan_query_interface(
                    handle,
                    &iface.interface_guid,
                    7,
                    std::ptr::null(),
                    &mut data_size,
                    &mut data_ptr,
                    &mut opcode_val,
                );

                if q_res == 0 && !data_ptr.is_null() {
                    let conn_attr = &*(data_ptr as *const WlanConnectionAttributes);
                    let assoc = &conn_attr.wlan_association_attributes;

                    let ssid_len = assoc.dot11_ssid.length.min(32) as usize;
                    let ssid = String::from_utf8_lossy(&assoc.dot11_ssid.ssid[..ssid_len]).to_string();

                    let bssid_bytes = assoc.dot11_bssid;
                    let bssid = format!(
                        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                        bssid_bytes[0], bssid_bytes[1], bssid_bytes[2],
                        bssid_bytes[3], bssid_bytes[4], bssid_bytes[5]
                    );

                    let quality = assoc.wlan_signal_quality.min(100);
                    // Estimativa empírica padrão: 100% = -50 dBm, 0% = -100 dBm
                    let rssi_dbm = (quality as i32 / 2) - 100;

                    let phy_type = match assoc.dot11_phy_type {
                        1 => "802.11 FHSS",
                        2 => "802.11 DSSS",
                        3 => "802.11 IR",
                        4 => "802.11b (Wi-Fi 1)",
                        5 => "802.11b (Wi-Fi 1)",
                        6 => "802.11g (Wi-Fi 3)",
                        7 => "802.11n (Wi-Fi 4)",
                        8 => "802.11ac (Wi-Fi 5)",
                        9 => "802.11ad",
                        10 => "802.11ax (Wi-Fi 6/6E)",
                        11 => "802.11be (Wi-Fi 7)",
                        _ => "802.11 Wireless",
                    }
                    .to_string();

                    let rx_rate_mbps = assoc.rx_rate as f64 / 1000.0;
                    let tx_rate_mbps = assoc.tx_rate as f64 / 1000.0;

                    info!(
                        "Wi-Fi ativo detectado: SSID='{}', BSSID='{}', Sinal={}%, RSSI={} dBm, PHY={}",
                        ssid, bssid, quality, rssi_dbm, phy_type
                    );

                    result_info = Some(WifiInfo {
                        ssid,
                        bssid,
                        signal_quality_pct: quality,
                        rssi_dbm,
                        phy_type,
                        tx_rate_mbps,
                        rx_rate_mbps,
                    });

                    wlan_free_memory(data_ptr);
                    break;
                }
            }
        }

        wlan_free_memory(iface_list_ptr as *mut c_void);
        wlan_close_handle(handle, std::ptr::null());

        result_info
    }
}
