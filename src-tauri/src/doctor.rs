use crate::{devices::list_devices, models::DoctorFinding, runtime::resolve_binary};

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

#[tauri::command]
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
            Some("Install Android Platform Tools or place adb in the runtime folder."),
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
            Some("Install official scrcpy or place it in the runtime folder."),
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
                for device in devices {
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
