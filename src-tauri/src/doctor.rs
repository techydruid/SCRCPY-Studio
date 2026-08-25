use crate::{devices::list_devices, models::DoctorFinding, runtime::resolve_binary};
use std::collections::HashSet;

fn finding(
    level: &str,
    title: &str,
    detail: impl Into<String>,
    action: Option<&str>,
) -> DoctorFinding {
    DoctorFinding {
        level: level.into(),
        title: title.into(),
        detail: detail.into(),
        action: action.map(str::to_owned),
    }
}

fn physical_key(device: &crate::models::DeviceInfo) -> String {
    match (&device.product, &device.model, &device.device) {
        (Some(product), Some(model), Some(codename)) => {
            format!("{product}|{model}|{codename}")
        }
        _ => device.serial.clone(),
    }
}

fn adb_install_action() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Click Install official runtime to add the verified Windows scrcpy package with ADB."
    }
    #[cfg(target_os = "linux")]
    {
        "Install the ADB package from your Linux distribution (for example, adb on Debian/Ubuntu or android-tools on Fedora)."
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
    {
        "Install Android Platform Tools and reopen SCRCPY Studio."
    }
}

fn scrcpy_install_action() -> &'static str {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        "Click Install official runtime. SCRCPY Studio downloads the latest official Genymobile release and verifies its SHA-256 checksum."
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
    {
        "Install official scrcpy and reopen SCRCPY Studio."
    }
}

#[tauri::command(async)]
pub(crate) fn run_doctor() -> Vec<DoctorFinding> {
    let mut items = Vec::new();
    let adb = resolve_binary("adb");
    let scrcpy = resolve_binary("scrcpy");

    match adb {
        Some(ref path) => items.push(finding(
            "ok",
            "ADB ready",
            format!("Using {}", path.display()),
            None,
        )),
        None => items.push(finding(
            "error",
            "ADB missing",
            "SCRCPY Studio cannot detect Android devices without ADB.",
            Some(adb_install_action()),
        )),
    }
    match scrcpy {
        Some(ref path) => items.push(finding(
            "ok",
            "scrcpy ready",
            format!("Using {}", path.display()),
            None,
        )),
        None => items.push(finding(
            "error",
            "scrcpy missing",
            "The mirroring engine is not available yet.",
            Some(scrcpy_install_action()),
        )),
    }

    if adb.is_some() {
        match list_devices() {
            Ok(devices) if devices.is_empty() => items.push(finding(
                "warning",
                "No phone detected",
                "ADB is working, but no Android device is visible.",
                Some("Use a data-capable USB cable, enable USB debugging, then reconnect."),
            )),
            Ok(devices) => {
                let mut seen = HashSet::new();
                for device in devices {
                    let key = physical_key(&device);
                    if !seen.insert(key) {
                        continue;
                    }

                    match device.state.as_str() {
                        "device" => items.push(finding(
                            "ok",
                            "Device authorized",
                            format!(
                                "{} is ready for control.",
                                device.model.unwrap_or(device.serial)
                            ),
                            None,
                        )),
                        "unauthorized" => items.push(finding(
                            "warning",
                            "USB debugging approval needed",
                            format!("{} is connected but not authorized.", device.serial),
                            Some("Unlock the phone and tap Allow on the USB debugging prompt."),
                        )),
                        "offline" => items.push(finding(
                            "warning",
                            "Device offline",
                            format!("{} is visible to ADB but not responding.", device.serial),
                            Some("Reconnect USB or restart Wireless debugging, then refresh."),
                        )),
                        other => items.push(finding(
                            "info",
                            "Device needs attention",
                            format!("{} reports state '{}'.", device.serial, other),
                            Some("Reconnect the device and refresh."),
                        )),
                    }
                }
            }
            Err(error) => items.push(finding(
                "error",
                "ADB check failed",
                error,
                Some("Close other ADB tools, reconnect the phone, and refresh."),
            )),
        }
    }

    items
}
