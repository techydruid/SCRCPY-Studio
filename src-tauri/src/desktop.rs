use crate::{
    devices::{inspect_device, list_devices},
    models::DesktopCapabilities,
    runtime::{adb_path, output_text, scrcpy_path},
};
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn default_launcher_package(serial: &str) -> Option<String> {
    let adb = adb_path().ok()?;
    let mut command = Command::new(adb);
    command.args([
        "-s",
        serial,
        "shell",
        "cmd",
        "package",
        "resolve-activity",
        "--brief",
        "-a",
        "android.intent.action.MAIN",
        "-c",
        "android.intent.category.HOME",
    ]);
    let text = output_text(command).ok()?;
    text.lines().rev().find_map(|line| {
        let line = line.trim();
        let (package, _) = line.split_once('/')?;
        if package.is_empty() || package.contains(' ') {
            None
        } else {
            Some(package.to_string())
        }
    })
}

fn scrcpy_help() -> Result<String, String> {
    let scrcpy = scrcpy_path()?;
    let mut command = Command::new(scrcpy);
    command.arg("--help");
    output_text(command)
}

fn compact_error(text: &str) -> String {
    let line = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(text)
        .trim();
    if line.len() > 220 {
        format!("{}…", &line[..220])
    } else {
        line.to_string()
    }
}

fn run_virtual_display_probe(serial: &str) -> Result<(), String> {
    let scrcpy = scrcpy_path()?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let recording = std::env::temp_dir().join(format!(
        "scrcpy-studio-vd-probe-{}-{stamp}.mp4",
        std::process::id()
    ));

    let mut command = Command::new(scrcpy);
    command.args([
        "-s",
        serial,
        "--new-display=1024x576/160",
        "--start-app=com.android.settings",
        "--no-audio",
        "--no-playback",
        "--no-control",
        "--no-window",
        "--time-limit=1",
    ]);
    command.arg(format!("--record={}", recording.display()));

    let output = command.output().map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&recording);
    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout.trim(), stderr.trim());
    if combined.trim().is_empty() {
        Err(format!("Virtual display probe exited with {}", output.status))
    } else {
        Err(compact_error(&combined))
    }
}

#[tauri::command]
pub(crate) fn probe_desktop_capabilities(serial: String) -> Result<DesktopCapabilities, String> {
    let devices = list_devices()?;
    let device = devices
        .iter()
        .find(|item| item.serial == serial)
        .ok_or_else(|| "Selected device is no longer connected.".to_string())?;
    if device.state != "device" {
        return Err("Authorize the device before checking Desktop Mode.".into());
    }

    let profile = inspect_device(serial.clone())?;
    let wireless = profile.connection_kind == "wireless";
    let (recommended_width, recommended_height, recommended_density) = if wireless {
        (1280, 720, 200)
    } else {
        (1920, 1080, 240)
    };

    if profile.sdk < 30 {
        return Ok(DesktopCapabilities {
            supported: false,
            recommended_width,
            recommended_height,
            recommended_density,
            flex_supported: false,
            system_decorations_supported: false,
            keep_content_supported: false,
            launcher_package: default_launcher_package(&serial),
            startup_package: "com.android.settings".into(),
            message: format!(
                "Android {} (API {}) is below SCRCPY Studio's safe virtual-display baseline.",
                profile.android_version, profile.sdk
            ),
        });
    }

    let help = scrcpy_help()?;
    if !help.contains("--new-display") {
        return Ok(DesktopCapabilities {
            supported: false,
            recommended_width,
            recommended_height,
            recommended_density,
            flex_supported: false,
            system_decorations_supported: false,
            keep_content_supported: false,
            launcher_package: default_launcher_package(&serial),
            startup_package: "com.android.settings".into(),
            message: "The installed scrcpy runtime does not expose --new-display. Update the runtime to use Desktop Mode.".into(),
        });
    }

    let launcher_package = default_launcher_package(&serial);
    let startup_package = launcher_package
        .clone()
        .unwrap_or_else(|| "com.android.settings".into());

    match run_virtual_display_probe(&serial) {
        Ok(()) => Ok(DesktopCapabilities {
            supported: true,
            recommended_width,
            recommended_height,
            recommended_density,
            flex_supported: help.contains("--flex-display"),
            system_decorations_supported: help.contains("--no-vd-system-decorations"),
            keep_content_supported: help.contains("--no-vd-destroy-content"),
            launcher_package,
            startup_package,
            message: "Virtual display probe passed. Desktop Mode can create and mirror a secondary Android display on this phone.".into(),
        }),
        Err(reason) => Ok(DesktopCapabilities {
            supported: false,
            recommended_width,
            recommended_height,
            recommended_density,
            flex_supported: help.contains("--flex-display"),
            system_decorations_supported: help.contains("--no-vd-system-decorations"),
            keep_content_supported: help.contains("--no-vd-destroy-content"),
            launcher_package,
            startup_package,
            message: format!("Virtual display probe failed: {reason}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_error_prefers_last_nonempty_line() {
        assert_eq!(compact_error("first\n\nlast"), "last");
    }
}
