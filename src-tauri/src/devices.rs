use crate::{
    models::{DeviceInfo, DeviceProfile, Recommendation},
    preferences::load_learned_profile,
    runtime::{adb_path, output_text, scrcpy_path},
};
use std::{collections::HashMap, process::Command};

pub(crate) fn is_wireless_serial(serial: &str) -> bool {
    serial.contains(':') || serial.contains("_adb-tls-")
}

pub(crate) fn parse_devices(raw: &str) -> Vec<DeviceInfo> {
    raw.lines()
        .skip_while(|line| !line.starts_with("List of devices attached"))
        .skip(1)
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('*') {
                return None;
            }
            let mut parts = line.split_whitespace();
            let serial = parts.next()?.to_string();
            let state = parts.next().unwrap_or("unknown").to_string();
            let mut fields: HashMap<&str, String> = HashMap::new();
            for part in parts {
                if let Some((key, value)) = part.split_once(':') {
                    fields.insert(key, value.to_string());
                }
            }
            Some(DeviceInfo {
                connection_kind: if is_wireless_serial(&serial) {
                    "wireless".into()
                } else {
                    "usb".into()
                },
                serial,
                state,
                model: fields.get("model").cloned(),
                product: fields.get("product").cloned(),
                device: fields.get("device").cloned(),
            })
        })
        .collect()
}

#[tauri::command]
pub(crate) fn list_devices() -> Result<Vec<DeviceInfo>, String> {
    let adb = adb_path()?;
    let mut command = Command::new(adb);
    command.args(["devices", "-l"]);
    let raw = output_text(command)?;
    Ok(parse_devices(&raw))
}

fn adb_shell(serial: &str, args: &[&str]) -> Result<String, String> {
    let adb = adb_path()?;
    let mut command = Command::new(adb);
    command.arg("-s").arg(serial).arg("shell").args(args);
    output_text(command)
}

fn getprop(serial: &str, prop: &str) -> String {
    adb_shell(serial, &["getprop", prop])
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn parse_u32(value: &str) -> u32 {
    value.trim().parse::<u32>().unwrap_or(0)
}

fn parse_physical_size(raw: &str) -> (Option<u32>, Option<u32>) {
    let candidate = raw
        .lines()
        .find(|line| line.to_ascii_lowercase().contains("physical size"))
        .or_else(|| raw.lines().find(|line| line.contains('x')));
    if let Some(line) = candidate {
        if let Some(value) = line.split(':').next_back() {
            if let Some((w, h)) = value.trim().split_once('x') {
                return (w.trim().parse().ok(), h.trim().parse().ok());
            }
        }
    }
    (None, None)
}

fn parse_density(raw: &str) -> Option<u32> {
    raw.lines()
        .find(|line| line.to_ascii_lowercase().contains("physical density"))
        .or_else(|| raw.lines().next())
        .and_then(|line| line.split(':').next_back())
        .and_then(|value| value.trim().parse().ok())
}

fn has_h265_encoder(serial: &str) -> bool {
    let Ok(scrcpy) = scrcpy_path() else {
        return false;
    };
    let mut command = Command::new(scrcpy);
    command.args(["-s", serial, "--list-encoders"]);
    match output_text(command) {
        Ok(text) => {
            let lower = text.to_ascii_lowercase();
            lower.contains("h265") || lower.contains("hevc")
        }
        Err(_) => false,
    }
}

#[tauri::command]
pub(crate) fn inspect_device(serial: String) -> Result<DeviceProfile, String> {
    let devices = list_devices()?;
    let device = devices
        .iter()
        .find(|d| d.serial == serial)
        .ok_or_else(|| "Selected device is no longer connected.".to_string())?;
    if device.state != "device" {
        return Err(format!(
            "Device state is '{}'. Authorize or reconnect the phone first.",
            device.state
        ));
    }

    let model = {
        let prop = getprop(&serial, "ro.product.model");
        if prop.is_empty() {
            device.model.clone().unwrap_or_else(|| serial.clone())
        } else {
            prop
        }
    };
    let brand = getprop(&serial, "ro.product.brand");
    let android_version = getprop(&serial, "ro.build.version.release");
    let sdk = parse_u32(&getprop(&serial, "ro.build.version.sdk"));
    let (width, height) =
        parse_physical_size(&adb_shell(&serial, &["wm", "size"]).unwrap_or_default());
    let density = parse_density(&adb_shell(&serial, &["wm", "density"]).unwrap_or_default());

    Ok(DeviceProfile {
        serial: serial.clone(),
        model,
        brand,
        android_version,
        sdk,
        width,
        height,
        density,
        connection_kind: if is_wireless_serial(&serial) {
            "wireless".into()
        } else {
            "usb".into()
        },
        supports_audio: sdk >= 30,
        supports_camera: sdk >= 31,
        can_attempt_virtual_display: sdk >= 30,
        h265_available: has_h265_encoder(&serial),
    })
}

fn recommendation_for(profile: &DeviceProfile, mode: &str) -> Recommendation {
    let wireless = profile.connection_kind == "wireless";
    let mut max_size = if wireless { 1280 } else { 1920 };
    let mut max_fps = if wireless { 30 } else { 60 };
    let mut codec = "h264".to_string();
    let mut audio = profile.supports_audio;
    let mut stay_awake = true;
    let turn_screen_off = false;
    let mut show_touches = false;
    let mut rationale = vec![if wireless {
        "Wireless profile reduces bandwidth for a steadier connection.".to_string()
    } else {
        "USB connection allows a higher-quality low-latency profile.".to_string()
    }];

    match mode {
        "creator" => {
            show_touches = true;
            codec = if profile.h265_available {
                "h265".into()
            } else {
                "h264".into()
            };
            max_size = if wireless { 1280 } else { 1920 };
            max_fps = if wireless { 30 } else { 60 };
            rationale.push(
                "Creator Mode enables visible touches and prioritizes clean tutorial capture."
                    .into(),
            );
            if profile.h265_available {
                rationale.push(
                    "H.265 is available, so Creator Mode starts with the more bandwidth-efficient encoder."
                        .into(),
                );
            }
        }
        "camera" => {
            max_size = 1920;
            max_fps = 30;
            codec = "h264".into();
            stay_awake = false;
            rationale.push(
                "Camera Mode uses conservative 1080p/30 settings for broad encoder compatibility."
                    .into(),
            );
        }
        "desktop" => {
            max_size = 1920;
            max_fps = if wireless { 30 } else { 60 };
            codec = "h264".into();
            rationale.push(
                "Desktop Mode uses scrcpy's virtual-display path and a compatibility-first codec."
                    .into(),
            );
        }
        _ => {
            rationale.push(
                "Everyday Mirror Mode favors H.264 compatibility and responsive control.".into(),
            );
        }
    }

    if !profile.supports_audio {
        audio = false;
        rationale.push(
            "Audio forwarding is disabled because this Android version is below the supported level."
                .into(),
        );
    }

    Recommendation {
        mode: mode.to_string(),
        max_size,
        max_fps,
        codec,
        audio,
        stay_awake,
        turn_screen_off,
        show_touches,
        quality_label: match mode {
            "creator" => "Creator-ready profile".into(),
            "camera" => "Stable camera profile".into(),
            "desktop" => "Balanced desktop profile".into(),
            _ if wireless => "Balanced wireless profile".into(),
            _ => "Fast USB profile".into(),
        },
        rationale,
    }
}

#[tauri::command]
pub(crate) fn recommend_settings(
    serial: String,
    mode: String,
) -> Result<Recommendation, String> {
    if !matches!(mode.as_str(), "mirror" | "creator" | "camera" | "desktop") {
        return Err("Unknown session mode.".into());
    }
    let profile = inspect_device(serial.clone())?;
    if mode == "camera" && !profile.supports_camera {
        return Err("Camera mirroring requires Android 12 or newer.".into());
    }

    let mut recommendation = recommendation_for(&profile, &mode);
    if let Some(learned) = load_learned_profile(&serial, &mode) {
        let learned_codec_supported = learned.codec != "h265" || profile.h265_available;
        if learned_codec_supported {
            recommendation.max_size = learned.max_size;
            recommendation.max_fps = learned.max_fps;
            recommendation.codec = learned.codec;
            recommendation.audio = learned.audio && profile.supports_audio;
            recommendation.quality_label = "Tested device profile".into();
            recommendation.rationale.insert(
                0,
                "Using settings that previously launched successfully on this device in this mode."
                    .into(),
            );
        }
    }

    Ok(recommendation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_usb_and_wireless_devices() {
        let raw = "List of devices attached\nABC123 device product:foo model:Pixel_9 device:tokay transport_id:1\n192.168.1.7:5555 unauthorized product:bar model:POCO_F1 device:beryllium\n";
        let devices = parse_devices(raw);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].connection_kind, "usb");
        assert_eq!(devices[1].connection_kind, "wireless");
        assert_eq!(devices[1].state, "unauthorized");
    }
}
