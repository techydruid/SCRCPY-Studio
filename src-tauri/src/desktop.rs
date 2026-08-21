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

fn recommended_desktop_geometry(brand: &str, wireless: bool) -> (u32, u32, u32) {
    if wireless {
        // Keep the smallest width at >= 600dp so Android does not fall back to
        // a phone-class layout just because the transport is wireless.
        return (1280, 720, 180);
    }

    if brand.to_ascii_lowercase().contains("samsung") {
        // One UI 8/DeX-style virtual displays behave best around this density.
        (1920, 1080, 284)
    } else {
        (1920, 1080, 240)
    }
}

fn virtual_display_probe_args(serial: &str) -> Vec<String> {
    vec![
        "-s".into(),
        serial.into(),
        "--new-display=1024x640/160".into(),
        "--no-audio".into(),
        "--no-playback".into(),
        "--no-control".into(),
        "--no-window".into(),
        "--time-limit=1".into(),
    ]
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

    // This probe answers only one question: can scrcpy create and stream a
    // secondary display on this device? Do not combine --start-app with
    // --no-control: scrcpy correctly rejects that combination because starting
    // an Android app is itself a control action. The real Desktop launch is
    // tested separately and intentionally lets Android/OEMs own the new display.
    let mut command = Command::new(scrcpy);
    command.args(virtual_display_probe_args(serial));
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
    let (recommended_width, recommended_height, recommended_density) =
        recommended_desktop_geometry(&profile.brand, wireless);
    let launcher_package = default_launcher_package(&serial);

    if profile.sdk < 30 {
        return Ok(DesktopCapabilities {
            supported: false,
            recommended_width,
            recommended_height,
            recommended_density,
            flex_supported: false,
            system_decorations_supported: false,
            keep_content_supported: false,
            launcher_package,
            startup_package: String::new(),
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
            launcher_package,
            startup_package: String::new(),
            message: "The installed scrcpy runtime does not expose --new-display. Update the runtime to use Desktop Mode.".into(),
        });
    }

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
            startup_package: String::new(),
            message: "Virtual display probe passed. SCRCPY Studio will now let Android or the phone maker start its own secondary-display desktop environment instead of forcing the normal phone launcher.".into(),
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
            startup_package: String::new(),
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

    #[test]
    fn wireless_geometry_stays_desktop_class() {
        let (width, height, density) = recommended_desktop_geometry("Samsung", true);
        assert_eq!((width, height), (1280, 720));
        assert!(height * 160 / density >= 600);
    }

    #[test]
    fn samsung_usb_uses_desktop_friendly_density() {
        assert_eq!(recommended_desktop_geometry("samsung", false), (1920, 1080, 284));
    }

    #[test]
    fn desktop_probe_never_starts_an_app_while_control_is_disabled() {
        let args = virtual_display_probe_args("ABC123");
        assert!(args.contains(&"--no-control".to_string()));
        assert!(args.iter().any(|arg| arg.starts_with("--new-display=")));
        assert!(!args.iter().any(|arg| arg.starts_with("--start-app=")));
    }
}
