use crate::{
    commands::hidden_command,
    devices::list_devices,
    models::{DeviceInfo, RememberedWirelessDevice, TransportSwitchResult},
    runtime::{adb_path, output_text},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::Ipv4Addr,
    path::PathBuf,
    str::FromStr,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedWirelessDevice {
    address: String,
    label: String,
    last_used: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WirelessStore {
    #[serde(default)]
    devices: Vec<SavedWirelessDevice>,
}

fn safe_address(address: &str) -> Result<String, String> {
    let value = address.trim();
    if value.is_empty()
        || value.len() > 255
        || value.chars().any(char::is_whitespace)
        || !value.contains(':')
    {
        return Err("Enter a valid host:port address, for example 192.168.1.20:37123.".into());
    }
    Ok(value.to_string())
}

fn store_path() -> Result<PathBuf, String> {
    let base = dirs::config_local_dir()
        .or_else(dirs::data_local_dir)
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not locate a local settings directory.".to_string())?;
    let folder = base.join("SCRCPY Studio");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder.join("wireless-devices.json"))
}

fn read_store() -> Result<WirelessStore, String> {
    let path = store_path()?;
    if !path.exists() {
        return Ok(WirelessStore::default());
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("Could not read remembered wireless devices: {e}"))
}

fn write_store(store: &WirelessStore) -> Result<(), String> {
    let path = store_path()?;
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn connect_failed(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "failed to connect",
        "cannot connect",
        "unable to connect",
        "connection refused",
        "no route to host",
        "timed out",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn adb_getprop(serial: &str, prop: &str) -> Option<String> {
    let adb = adb_path().ok()?;
    let mut command = hidden_command(adb);
    command.args(["-s", serial, "shell", "getprop", prop]);
    output_text(command)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "unknown")
}

fn hardware_serial(serial: &str) -> Option<String> {
    adb_getprop(serial, "ro.boot.serialno").or_else(|| adb_getprop(serial, "ro.serialno"))
}

fn same_physical_phone(a: &DeviceInfo, b: &DeviceInfo) -> bool {
    if let (Some(a_serial), Some(b_serial)) = (hardware_serial(&a.serial), hardware_serial(&b.serial)) {
        if a_serial == b_serial {
            return true;
        }
    }

    a.model.is_some()
        && a.model == b.model
        && a.product.is_some()
        && a.product == b.product
        && a.device.is_some()
        && a.device == b.device
}

fn sibling_transport(serial: &str, target_kind: &str) -> Result<Option<DeviceInfo>, String> {
    let devices = list_devices()?;
    let source = devices
        .iter()
        .find(|item| item.serial == serial)
        .cloned()
        .ok_or_else(|| "The selected phone is no longer visible to ADB.".to_string())?;

    let matches = devices
        .into_iter()
        .filter(|item| {
            item.state == "device"
                && item.connection_kind == target_kind
                && item.serial != source.serial
                && same_physical_phone(&source, item)
        })
        .collect::<Vec<_>>();

    Ok(if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    })
}

fn connected_model(address: &str) -> Option<String> {
    adb_getprop(address, "ro.product.model")
}

fn remember_address(address: &str) -> Result<(), String> {
    let mut store = read_store().unwrap_or_default();
    let label = connected_model(address).unwrap_or_else(|| address.to_string());
    let last_used = now_epoch();

    if let Some(existing) = store.devices.iter_mut().find(|item| item.address == address) {
        existing.label = label;
        existing.last_used = last_used;
    } else {
        store.devices.push(SavedWirelessDevice {
            address: address.to_string(),
            label,
            last_used,
        });
    }
    store.devices.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    write_store(&store)
}

fn disconnect_address(address: &str) -> Result<String, String> {
    let adb = adb_path()?;
    let mut command = hidden_command(adb);
    command.args(["disconnect", address]);
    output_text(command)
}

fn parse_route_ipv4(text: &str) -> Option<String> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    for pair in tokens.windows(2) {
        if pair[0] == "src" && Ipv4Addr::from_str(pair[1]).is_ok() {
            return Some(pair[1].to_string());
        }
    }
    None
}

fn parse_wlan_ipv4(text: &str) -> Option<String> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    for pair in tokens.windows(2) {
        if pair[0] == "inet" {
            let value = pair[1].split('/').next().unwrap_or_default();
            if Ipv4Addr::from_str(value).is_ok() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn device_wifi_ip(serial: &str) -> Result<String, String> {
    let adb = adb_path()?;

    let mut route = hidden_command(&adb);
    route.args(["-s", serial, "shell", "ip", "route"]);
    if let Ok(text) = output_text(route) {
        if let Some(ip) = parse_route_ipv4(&text) {
            return Ok(ip);
        }
    }

    let mut wlan = hidden_command(adb);
    wlan.args(["-s", serial, "shell", "ip", "-f", "inet", "addr", "show", "wlan0"]);
    if let Ok(text) = output_text(wlan) {
        if let Some(ip) = parse_wlan_ipv4(&text) {
            return Ok(ip);
        }
    }

    Err("Could not detect the phone's Wi-Fi IP address. Make sure the phone is connected to Wi-Fi.".into())
}

#[tauri::command(async)]
pub(crate) fn pair_device(address: String, code: String) -> Result<String, String> {
    let address = safe_address(&address)?;
    let code = code.trim();
    if code.len() < 6 || code.len() > 12 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("Enter the numeric pairing code shown on the phone.".into());
    }
    let adb = adb_path()?;
    let mut command = hidden_command(adb);
    command.args(["pair", &address, code]);
    let output = output_text(command)?;
    Ok(if output.is_empty() {
        "Pairing request completed. Use the separate IP:port shown on the main Wireless debugging page to connect.".into()
    } else {
        output
    })
}

#[tauri::command(async)]
pub(crate) fn connect_device(address: String) -> Result<String, String> {
    let address = safe_address(&address)?;
    let adb = adb_path()?;
    let mut command = hidden_command(adb);
    command.args(["connect", &address]);
    let output = output_text(command)?;
    if connect_failed(&output) {
        return Err(output);
    }
    remember_address(&address)?;
    Ok(if output.is_empty() {
        format!("Connected to {address}")
    } else {
        output
    })
}

#[tauri::command(async)]
pub(crate) fn list_remembered_wireless() -> Result<Vec<RememberedWirelessDevice>, String> {
    let store = read_store()?;
    let connected = list_devices().unwrap_or_default();
    let mut items = store
        .devices
        .into_iter()
        .map(|saved| RememberedWirelessDevice {
            connected: connected
                .iter()
                .any(|device| device.serial == saved.address && device.state == "device"),
            address: saved.address,
            label: saved.label,
            last_used: saved.last_used,
        })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    Ok(items)
}

#[tauri::command(async)]
pub(crate) fn reconnect_wireless_device(address: String) -> Result<String, String> {
    connect_device(address)
}

#[tauri::command(async)]
pub(crate) fn forget_wireless_device(address: String) -> Result<TransportSwitchResult, String> {
    let address = safe_address(&address)?;
    let usb_sibling = sibling_transport(&address, "usb").ok().flatten();

    let mut store = read_store().unwrap_or_default();
    store.devices.retain(|item| item.address != address);
    write_store(&store)?;

    let _ = disconnect_address(&address);

    if let Some(usb) = usb_sibling {
        Ok(TransportSwitchResult {
            active_serial: usb.serial,
            active_connection: "usb".into(),
            message: "Wireless connection disconnected and forgotten. USB is now selected.".into(),
            safe_to_unplug_usb: false,
        })
    } else {
        Ok(TransportSwitchResult {
            active_serial: String::new(),
            active_connection: "none".into(),
            message: "Wireless connection disconnected and forgotten.".into(),
            safe_to_unplug_usb: false,
        })
    }
}

#[tauri::command(async)]
pub(crate) fn enable_usb_wireless(serial: String) -> Result<TransportSwitchResult, String> {
    let devices = list_devices()?;
    let device = devices
        .iter()
        .find(|item| item.serial == serial)
        .ok_or_else(|| "The selected USB device is no longer connected.".to_string())?;
    if device.state != "device" {
        return Err("The selected device is not authorized for ADB.".into());
    }
    if device.connection_kind != "usb" {
        return Err("This device is already connected wirelessly.".into());
    }

    let ip = device_wifi_ip(&serial)?;
    let adb = adb_path()?;
    let mut tcpip = hidden_command(adb);
    tcpip.args(["-s", &serial, "tcpip", "5555"]);
    let output = output_text(tcpip)?;
    if connect_failed(&output) {
        return Err(output);
    }

    thread::sleep(Duration::from_millis(900));
    let address = format!("{ip}:5555");
    connect_device(address.clone())?;

    Ok(TransportSwitchResult {
        active_serial: address.clone(),
        active_connection: "wireless".into(),
        message: format!("Wireless connection established at {address}. You can unplug the USB cable now."),
        safe_to_unplug_usb: true,
    })
}

#[tauri::command(async)]
pub(crate) fn switch_to_usb(serial: String) -> Result<TransportSwitchResult, String> {
    let devices = list_devices()?;
    let selected = devices
        .iter()
        .find(|item| item.serial == serial)
        .ok_or_else(|| "The selected phone is no longer visible to ADB.".to_string())?;

    if selected.connection_kind == "usb" {
        return Ok(TransportSwitchResult {
            active_serial: selected.serial.clone(),
            active_connection: "usb".into(),
            message: "USB is already the active connection.".into(),
            safe_to_unplug_usb: false,
        });
    }

    let usb = sibling_transport(&serial, "usb")?.ok_or_else(|| {
        "Connect this phone with a USB data cable and approve USB debugging, then try Use USB Instead again."
            .to_string()
    })?;

    let _ = disconnect_address(&serial);
    thread::sleep(Duration::from_millis(250));

    Ok(TransportSwitchResult {
        active_serial: usb.serial,
        active_connection: "usb".into(),
        message: "USB connection is active. Wireless ADB has been disconnected.".into(),
        safe_to_unplug_usb: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_wireless_addresses() {
        assert!(safe_address("192.168.1.20:5555").is_ok());
        assert!(safe_address("bad address:5555").is_err());
        assert!(safe_address("192.168.1.20").is_err());
    }

    #[test]
    fn parses_route_source_ipv4() {
        let route = "192.168.1.0/24 dev wlan0 proto kernel scope link src 192.168.1.55";
        assert_eq!(parse_route_ipv4(route).as_deref(), Some("192.168.1.55"));
    }

    #[test]
    fn parses_wlan_inet_ipv4() {
        let addr = "inet 10.0.0.18/24 brd 10.0.0.255 scope global wlan0";
        assert_eq!(parse_wlan_ipv4(addr).as_deref(), Some("10.0.0.18"));
    }

    #[test]
    fn identifies_failed_adb_connect_text() {
        assert!(connect_failed("failed to connect to 1.2.3.4:5555"));
        assert!(!connect_failed("connected to 1.2.3.4:5555"));
    }

    #[test]
    fn matches_same_physical_device_from_adb_metadata() {
        let usb = DeviceInfo {
            serial: "ABC123".into(),
            state: "device".into(),
            model: Some("CPH2413".into()),
            product: Some("ossi".into()),
            device: Some("ossi".into()),
            connection_kind: "usb".into(),
        };
        let wifi = DeviceInfo {
            serial: "192.168.1.20:5555".into(),
            state: "device".into(),
            model: Some("CPH2413".into()),
            product: Some("ossi".into()),
            device: Some("ossi".into()),
            connection_kind: "wireless".into(),
        };
        assert!(same_physical_phone(&usb, &wifi));
    }
}
