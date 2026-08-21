use crate::{
    devices::{inspect_device, list_devices},
    models::{DesktopCapabilities, DesktopExperienceResult},
    runtime::{adb_path, output_text, scrcpy_path},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const FORCE_DESKTOP: &str = "force_desktop_mode_on_external_displays";
const ENABLE_FREEFORM: &str = "enable_freeform_support";
const FORCE_RESIZABLE: &str = "force_resizable_activities";
const FORCE_ALLOW_EXTERNAL: &str = "force_allow_on_external";
const ENABLE_NON_RESIZABLE_MULTI_WINDOW: &str = "enable_non_resizable_multi_window";
const OVERRIDE_DESKTOP_EXPERIENCE: &str = "override_desktop_experience_features";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DesktopSettingsBackup {
    #[serde(default)]
    settings: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DesktopBackupStore {
    #[serde(default)]
    devices: HashMap<String, DesktopSettingsBackup>,
}

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
        // One UI 8 desktop/DeX-style virtual displays are commonly demonstrated
        // at 1920x1080/284. Keep that as the Samsung starting point.
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
    // --no-control: scrcpy rejects that combination because app launch is a
    // control action. Desktop UI readiness is diagnosed separately below.
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

fn run_adb_shell(serial: &str, args: &[&str]) -> Result<String, String> {
    let adb = adb_path()?;
    let mut command = Command::new(adb);
    command.arg("-s").arg(serial).arg("shell");
    command.args(args);
    output_text(command)
}

fn get_global_setting(serial: &str, key: &str) -> Result<Option<String>, String> {
    let text = run_adb_shell(serial, &["settings", "get", "global", key])?;
    let value = text.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("null") {
        Ok(None)
    } else {
        Ok(Some(value.to_string()))
    }
}

fn put_global_setting(serial: &str, key: &str, value: &str) -> Result<(), String> {
    run_adb_shell(serial, &["settings", "put", "global", key, value]).map(|_| ())
}

fn delete_global_setting(serial: &str, key: &str) -> Result<(), String> {
    run_adb_shell(serial, &["settings", "delete", "global", key]).map(|_| ())
}

fn stable_device_key(serial: &str) -> String {
    for prop in ["ro.serialno", "ro.boot.serialno"] {
        if let Ok(value) = run_adb_shell(serial, &["getprop", prop]) {
            let value = value.trim();
            if !value.is_empty() && !value.eq_ignore_ascii_case("unknown") {
                return value.to_string();
            }
        }
    }
    serial.to_string()
}

fn desktop_backup_path() -> Result<PathBuf, String> {
    let base = dirs::config_local_dir()
        .or_else(dirs::data_local_dir)
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not locate a local settings directory.".to_string())?;
    let folder = base.join("SCRCPY Studio");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder.join("desktop-settings-backup.json"))
}

fn read_backup_store() -> Result<DesktopBackupStore, String> {
    let path = desktop_backup_path()?;
    if !path.exists() {
        return Ok(DesktopBackupStore::default());
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("Could not read desktop settings backup: {e}"))
}

fn write_backup_store(store: &DesktopBackupStore) -> Result<(), String> {
    let path = desktop_backup_path()?;
    let raw = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn desktop_setting_keys(sdk: u32) -> Vec<&'static str> {
    let mut keys = vec![
        FORCE_DESKTOP,
        ENABLE_FREEFORM,
        FORCE_RESIZABLE,
        FORCE_ALLOW_EXTERNAL,
        ENABLE_NON_RESIZABLE_MULTI_WINDOW,
    ];
    if sdk >= 36 {
        keys.push(OVERRIDE_DESKTOP_EXPERIENCE);
    }
    keys
}

fn backup_available(serial: &str) -> bool {
    let key = stable_device_key(serial);
    read_backup_store()
        .map(|store| store.devices.contains_key(&key))
        .unwrap_or(false)
}

fn save_backup_if_missing(serial: &str, sdk: u32) -> Result<(), String> {
    let device_key = stable_device_key(serial);
    let mut store = read_backup_store().unwrap_or_default();
    if store.devices.contains_key(&device_key) {
        return Ok(());
    }

    let mut settings = HashMap::new();
    for key in desktop_setting_keys(sdk) {
        settings.insert(key.to_string(), get_global_setting(serial, key)?);
    }
    store
        .devices
        .insert(device_key, DesktopSettingsBackup { settings });
    write_backup_store(&store)
}

fn setting_enabled(values: &HashMap<String, Option<String>>, key: &str) -> bool {
    values
        .get(key)
        .and_then(|value| value.as_deref())
        .map(|value| value == "1")
        .unwrap_or(false)
}

fn desktop_experience_state(serial: &str, sdk: u32) -> (bool, String) {
    let mut values = HashMap::new();
    for key in desktop_setting_keys(sdk) {
        let value = get_global_setting(serial, key).unwrap_or(None);
        values.insert(key.to_string(), value);
    }

    let force_desktop = setting_enabled(&values, FORCE_DESKTOP);
    let freeform = setting_enabled(&values, ENABLE_FREEFORM);
    let resizable = setting_enabled(&values, FORCE_RESIZABLE);
    let android16_override = sdk < 36 || setting_enabled(&values, OVERRIDE_DESKTOP_EXPERIENCE);
    let prepared = force_desktop && freeform && resizable && android16_override;

    let summary = if prepared {
        "Android desktop-on-secondary-display, freeform windows, and resizable-app support are enabled.".to_string()
    } else if sdk >= 36 {
        "Virtual display works, but Android 16 desktop-experience/freeform developer settings are not fully enabled.".to_string()
    } else {
        "Virtual display works, but desktop-on-secondary-display/freeform developer settings are not fully enabled.".to_string()
    };

    (prepared, summary)
}

fn ensure_ready_device(serial: &str) -> Result<(), String> {
    let devices = list_devices()?;
    let device = devices
        .iter()
        .find(|item| item.serial == serial)
        .ok_or_else(|| "Selected device is no longer connected.".to_string())?;
    if device.state != "device" {
        return Err("Authorize the device before changing Desktop UI settings.".into());
    }
    Ok(())
}

fn reboot_device(serial: &str) -> Result<(), String> {
    let adb = adb_path()?;
    let status = Command::new(adb)
        .args(["-s", serial, "reboot"])
        .status()
        .map_err(|e| format!("Could not restart the phone: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ADB reboot exited with {status}"))
    }
}

#[tauri::command]
pub(crate) fn enable_desktop_experience(serial: String) -> Result<DesktopExperienceResult, String> {
    ensure_ready_device(&serial)?;
    let profile = inspect_device(serial.clone())?;
    if profile.sdk < 30 {
        return Err("This Android version is below SCRCPY Studio's Desktop UI preparation baseline.".into());
    }

    // Preserve the user's current developer-setting values once. Repeated
    // presses never overwrite the original backup with our own enabled values.
    save_backup_if_missing(&serial, profile.sdk)?;

    for key in desktop_setting_keys(profile.sdk) {
        put_global_setting(&serial, key, "1")?;
    }
    let _ = run_adb_shell(&serial, &["sync"]);

    let (prepared, _) = desktop_experience_state(&serial, profile.sdk);
    if !prepared {
        return Err("Android accepted the settings commands but did not report the desktop environment as prepared.".into());
    }

    // Android's own developer-options flow asks for a reboot when desktop mode
    // on secondary displays is enabled. Do the same, but only after the user
    // explicitly clicked an Enable & Restart button in the UI.
    reboot_device(&serial)?;

    Ok(DesktopExperienceResult {
        prepared: true,
        backup_available: true,
        reboot_started: true,
        message: "Desktop UI settings enabled. The phone is restarting once so Android can apply desktop windowing. Reconnect after it finishes, then open Desktop Mode again.".into(),
    })
}

#[tauri::command]
pub(crate) fn restore_desktop_experience(serial: String) -> Result<DesktopExperienceResult, String> {
    ensure_ready_device(&serial)?;
    let device_key = stable_device_key(&serial);
    let mut store = read_backup_store()?;
    let backup = store
        .devices
        .get(&device_key)
        .cloned()
        .ok_or_else(|| "No original Desktop UI settings backup exists for this phone.".to_string())?;

    for (key, value) in &backup.settings {
        match value {
            Some(value) => put_global_setting(&serial, key, value)?,
            None => delete_global_setting(&serial, key)?,
        }
    }
    store.devices.remove(&device_key);
    write_backup_store(&store)?;
    let _ = run_adb_shell(&serial, &["sync"]);
    reboot_device(&serial)?;

    Ok(DesktopExperienceResult {
        prepared: false,
        backup_available: false,
        reboot_started: true,
        message: "Original Android desktop developer settings restored. The phone is restarting to finish restoring its previous behavior.".into(),
    })
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
    let desktop_experience_can_prepare = profile.sdk >= 30;
    let (desktop_experience_prepared, desktop_experience_summary) =
        desktop_experience_state(&serial, profile.sdk);
    let desktop_experience_backup_available = backup_available(&serial);

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
            desktop_experience_prepared,
            desktop_experience_can_prepare,
            desktop_experience_backup_available,
            desktop_experience_summary,
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
            desktop_experience_prepared,
            desktop_experience_can_prepare,
            desktop_experience_backup_available,
            desktop_experience_summary,
            message: "The installed scrcpy runtime does not expose --new-display. Update the runtime to use Desktop Mode.".into(),
        });
    }

    match run_virtual_display_probe(&serial) {
        Ok(()) => {
            let message = if desktop_experience_prepared {
                "Virtual display support passed and Android desktop-windowing settings are prepared. SCRCPY Studio can now launch the desktop display without forcing the normal phone launcher.".into()
            } else {
                "Virtual display support passed, but a secondary display is not the same as desktop UI. Prepare Android's desktop-windowing settings before launching Desktop Mode.".into()
            };
            Ok(DesktopCapabilities {
                supported: true,
                recommended_width,
                recommended_height,
                recommended_density,
                flex_supported: help.contains("--flex-display"),
                system_decorations_supported: help.contains("--no-vd-system-decorations"),
                keep_content_supported: help.contains("--no-vd-destroy-content"),
                launcher_package,
                startup_package: String::new(),
                desktop_experience_prepared,
                desktop_experience_can_prepare,
                desktop_experience_backup_available,
                desktop_experience_summary,
                message,
            })
        }
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
            desktop_experience_prepared,
            desktop_experience_can_prepare,
            desktop_experience_backup_available,
            desktop_experience_summary,
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
    fn virtual_display_probe_never_combines_start_app_with_no_control() {
        let args = virtual_display_probe_args("ABC");
        assert!(args.contains(&"--no-control".to_string()));
        assert!(!args.iter().any(|arg| arg.starts_with("--start-app")));
    }

    #[test]
    fn android_16_desktop_preparation_includes_new_and_legacy_switches() {
        let keys = desktop_setting_keys(36);
        assert!(keys.contains(&FORCE_DESKTOP));
        assert!(keys.contains(&ENABLE_FREEFORM));
        assert!(keys.contains(&FORCE_RESIZABLE));
        assert!(keys.contains(&OVERRIDE_DESKTOP_EXPERIENCE));
    }

    #[test]
    fn pre_android_16_does_not_write_unknown_override_switch() {
        assert!(!desktop_setting_keys(35).contains(&OVERRIDE_DESKTOP_EXPERIENCE));
    }
}
