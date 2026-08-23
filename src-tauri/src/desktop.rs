use crate::{
    commands::hidden_command,
    devices::{inspect_device, list_devices},
    models::{
        DesktopCapabilities, DesktopDiagnostics, DesktopExperienceResult, DesktopSettingDiagnostic,
    },
    runtime::{adb_path, output_text, scrcpy_path},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FORCE_DESKTOP: &str = "force_desktop_mode_on_external_displays";
const ENABLE_FREEFORM: &str = "enable_freeform_support";
const FORCE_RESIZABLE: &str = "force_resizable_activities";
const ENABLE_NON_RESIZABLE_MULTI_WINDOW: &str = "enable_non_resizable_multi_window";
const OVERRIDE_DESKTOP_EXPERIENCE: &str = "override_desktop_experience_features";
const OVERRIDE_DESKTOP_MODE: &str = "override_desktop_mode_features";

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

#[derive(Debug, Clone, Default)]
struct PlatformSnapshot {
    settings: Vec<DesktopSettingDiagnostic>,
    evidence: Vec<String>,
    android_desktop_available: bool,
    samsung_dex_available: bool,
    samsung_dex_active: bool,
    samsung_dex_display_id: Option<u32>,
}

#[derive(Debug)]
struct VirtualDisplayProbe {
    success: bool,
    diagnostics: DesktopDiagnostics,
}

pub(crate) struct DesktopLaunchOutcome {
    pub(crate) started: bool,
    pub(crate) diagnostics: DesktopDiagnostics,
}

fn default_launcher_package(serial: &str) -> Option<String> {
    let text = run_adb_shell(
        serial,
        &[
            "cmd",
            "package",
            "resolve-activity",
            "--brief",
            "-a",
            "android.intent.action.MAIN",
            "-c",
            "android.intent.category.HOME",
        ],
    )
    .ok()?;
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
    let mut command = hidden_command(scrcpy);
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
    if line.chars().count() > 220 {
        format!("{}…", line.chars().take(220).collect::<String>())
    } else {
        line.to_string()
    }
}

fn recommended_desktop_geometry(brand: &str, wireless: bool) -> (u32, u32, u32) {
    if wireless {
        return (1280, 720, 180);
    }

    if brand.to_ascii_lowercase().contains("samsung") {
        // This is a generic virtual-display geometry, not a DeX trigger.
        (1920, 1080, 284)
    } else {
        (1920, 1080, 240)
    }
}

fn run_adb_shell(serial: &str, args: &[&str]) -> Result<String, String> {
    let adb = adb_path()?;
    let mut command = hidden_command(adb);
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

fn app_data_dir() -> Result<PathBuf, String> {
    let base = dirs::config_local_dir()
        .or_else(dirs::data_local_dir)
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not locate a local settings directory.".to_string())?;
    let folder = base.join("SCRCPY Studio");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder)
}

fn desktop_backup_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("desktop-settings-backup.json"))
}

fn diagnostics_dir() -> Result<PathBuf, String> {
    let folder = app_data_dir()?.join("Desktop Diagnostics");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder)
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
        ENABLE_NON_RESIZABLE_MULTI_WINDOW,
    ];
    if sdk >= 36 {
        keys.push(OVERRIDE_DESKTOP_EXPERIENCE);
    }
    keys
}

fn diagnostic_setting_keys(sdk: u32) -> Vec<&'static str> {
    let mut keys = desktop_setting_keys(sdk);
    if sdk >= 36 {
        keys.push(OVERRIDE_DESKTOP_MODE);
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

fn read_setting_diagnostics(serial: &str, sdk: u32) -> Vec<DesktopSettingDiagnostic> {
    diagnostic_setting_keys(sdk)
        .into_iter()
        .map(|key| DesktopSettingDiagnostic {
            key: key.to_string(),
            value: get_global_setting(serial, key).unwrap_or(None),
        })
        .collect()
}

fn diagnostic_setting_enabled(settings: &[DesktopSettingDiagnostic], key: &str) -> bool {
    settings
        .iter()
        .find(|item| item.key == key)
        .and_then(|item| item.value.as_deref())
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn developer_settings_applied(settings: &[DesktopSettingDiagnostic], sdk: u32) -> bool {
    desktop_setting_keys(sdk)
        .into_iter()
        .all(|key| diagnostic_setting_enabled(settings, key))
}

fn can_prepare_desktop_experience(
    supported: bool,
    dex_capturable: bool,
    sdk: u32,
    android_desktop_available: bool,
    settings_applied: bool,
) -> bool {
    supported
        && !dex_capturable
        && !settings_applied
        && sdk >= 29
        && (android_desktop_available || sdk >= 36)
}

fn overlay_bool(serial: &str, resource: &str) -> Option<bool> {
    let key = format!("android:bool/{resource}");
    let value = run_adb_shell(serial, &["cmd", "overlay", "lookup", "android", &key]).ok()?;
    let lower = value.trim().to_ascii_lowercase();
    if lower.ends_with("true") {
        Some(true)
    } else if lower.ends_with("false") {
        Some(false)
    } else {
        None
    }
}

fn parse_number_after(text: &str, marker: &str) -> Option<u32> {
    let start = text.find(marker)? + marker.len();
    let digits = text[start..]
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    digits.parse().ok()
}

fn is_samsung_dex_active(text: &str) -> bool {
    let compact = text.to_ascii_lowercase().replace(' ', "");
    [
        "enabled=4",
        "mdesktopmodestate=4",
        "desktopmode=true",
        "isdesktopmode=true",
        "state=enabled",
    ]
    .iter()
    .any(|needle| compact.contains(needle))
}

fn parse_samsung_dex_display_id(text: &str) -> Option<u32> {
    text.lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("dex") || lower.contains("desktop") || lower.contains("display")
        })
        .find_map(|line| {
            ["displayId=", "mDisplayId=", "display_id=", "displayId:"]
                .iter()
                .find_map(|marker| parse_number_after(line, marker))
        })
}

fn component_display_id(text: &str, components: &[&str]) -> Option<u32> {
    let mut current_display = None;
    for line in text.lines() {
        if let Some(id) = ["Display #", "displayId=", "mDisplayId=", "displayId: "]
            .iter()
            .find_map(|marker| parse_number_after(line, marker))
        {
            current_display = Some(id);
        }
        if components.iter().any(|component| line.contains(component)) {
            let inline = ["displayId=", "mDisplayId=", "displayId: "]
                .iter()
                .find_map(|marker| parse_number_after(line, marker));
            return inline.or(current_display).filter(|id| *id > 0);
        }
    }
    None
}

fn named_desktop_display_id(text: &str) -> Option<u32> {
    let mut current_display = None;
    for line in text.lines() {
        if let Some(id) = ["Display #", "mDisplayId=", "displayId=", "displayId: "]
            .iter()
            .find_map(|marker| parse_number_after(line, marker))
        {
            current_display = Some(id);
        }
        let lower = line.to_ascii_lowercase();
        if (lower.contains("dex")
            || lower.contains("com.samsung.android.hardware.display.category.desktop"))
            && current_display.is_some_and(|id| id > 0)
        {
            return current_display;
        }
    }
    None
}

fn collect_platform_snapshot(serial: &str, sdk: u32, brand: &str) -> PlatformSnapshot {
    let settings = read_setting_diagnostics(serial, sdk);
    let mut evidence = Vec::new();

    let desktop_config = overlay_bool(serial, "config_isDesktopModeSupported");
    let desktop_dev_option = overlay_bool(serial, "config_isDesktopModeDevOptionSupported");
    let multi_window = overlay_bool(serial, "config_supportsMultiWindow");
    let freeform_config = overlay_bool(serial, "config_freeformWindowManagement");
    evidence.push(format!(
        "OEM overlays: desktopModeSupported={}, desktopDevOptionSupported={}, supportsMultiWindow={}, freeformWindowManagement={}",
        optional_bool(desktop_config),
        optional_bool(desktop_dev_option),
        optional_bool(multi_window),
        optional_bool(freeform_config)
    ));

    let freeform_feature = run_adb_shell(
        serial,
        &[
            "pm",
            "has-feature",
            "android.software.freeform_window_management",
        ],
    )
    .ok();
    if let Some(feature) = &freeform_feature {
        evidence.push(format!("Freeform feature: {}", feature.trim()));
    }

    let freeform_feature_available = freeform_feature
        .as_deref()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"));
    let android_desktop_available = desktop_config == Some(true)
        || desktop_dev_option == Some(true)
        || freeform_config == Some(true)
        || freeform_feature_available;

    let samsung = brand.to_ascii_lowercase().contains("samsung");
    let launcher_installed = if samsung {
        [
            "com.sec.android.app.desktoplauncher",
            "com.samsung.android.app.desktoplauncher",
        ]
        .iter()
        .any(|package| run_adb_shell(serial, &["pm", "path", package]).is_ok())
    } else {
        false
    };
    let desktop_service = if samsung {
        run_adb_shell(serial, &["dumpsys", "desktopmode"]).unwrap_or_default()
    } else {
        String::new()
    };
    let service_present = !desktop_service.trim().is_empty()
        && !desktop_service.to_ascii_lowercase().contains("not found");
    let samsung_dex_available = samsung && (launcher_installed || service_present);
    let activity_dump = if samsung {
        run_adb_shell(serial, &["dumpsys", "activity", "activities"]).unwrap_or_default()
    } else {
        String::new()
    };
    let display_dump = if samsung {
        run_adb_shell(serial, &["dumpsys", "display"]).unwrap_or_default()
    } else {
        String::new()
    };
    let launcher_display_id = component_display_id(
        &activity_dump,
        &[
            "com.sec.android.app.desktoplauncher",
            "com.samsung.android.app.desktoplauncher",
        ],
    );
    let named_display_id = named_desktop_display_id(&display_dump);
    let samsung_dex_active = samsung_dex_available
        && (is_samsung_dex_active(&desktop_service)
            || launcher_display_id.is_some()
            || named_display_id.is_some());
    let samsung_dex_display_id = if samsung_dex_active {
        launcher_display_id
            .or_else(|| parse_samsung_dex_display_id(&desktop_service))
            .or(named_display_id)
    } else {
        None
    };

    if samsung {
        evidence.push(format!(
            "Samsung DeX service: installed={}, active={}, displayId={}",
            samsung_dex_available,
            samsung_dex_active,
            samsung_dex_display_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "none".into())
        ));
    }
    evidence.push(
        "Developer settings are reported as inputs only; they are not treated as proof that a desktop shell is running."
            .into(),
    );

    PlatformSnapshot {
        settings,
        evidence,
        android_desktop_available,
        samsung_dex_available,
        samsung_dex_active,
        samsung_dex_display_id,
    }
}

fn optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn ensure_ready_device(serial: &str) -> Result<(), String> {
    let devices = list_devices()?;
    let device = devices
        .iter()
        .find(|item| item.serial == serial)
        .ok_or_else(|| "Selected device is no longer connected.".to_string())?;
    if device.state != "device" {
        return Err("Authorize the device before changing Desktop settings.".into());
    }
    Ok(())
}

fn reboot_device(serial: &str) -> Result<(), String> {
    let adb = adb_path()?;
    let status = hidden_command(adb)
        .args(["-s", serial, "reboot"])
        .status()
        .map_err(|e| format!("Could not restart the phone: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ADB reboot exited with {status}"))
    }
}

fn virtual_display_probe_args(serial: &str) -> Vec<String> {
    vec![
        "-s".into(),
        serial.into(),
        "--new-display=1024x640/160".into(),
        "--start-app=com.android.settings".into(),
        "--no-audio".into(),
        "--no-playback".into(),
        "--no-window".into(),
        "--time-limit=5".into(),
    ]
}

fn quote_preview(value: &str) -> String {
    if value.chars().any(char::is_whitespace) {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

pub(crate) fn command_preview(path: &Path, args: &[String]) -> String {
    let mut values = vec![quote_preview(&path.display().to_string())];
    values.extend(args.iter().map(|arg| quote_preview(arg)));
    values.join(" ")
}

fn parse_new_display(text: &str) -> Option<(u32, u32, u32, u32)> {
    let marker = "New display:";
    let after = text.split_once(marker)?.1.trim();
    let geometry = after.split_whitespace().next()?;
    let (size, dpi) = geometry.split_once('/')?;
    let (width, height) = size.split_once('x')?;
    let id = parse_number_after(after, "id=")?;
    Some((
        width.parse().ok()?,
        height.parse().ok()?,
        dpi.parse().ok()?,
        id,
    ))
}

fn parse_resolution(text: &str) -> Option<String> {
    for token in text.split(|ch: char| ch.is_whitespace() || matches!(ch, ':' | ',' | '(' | ')')) {
        let cleaned = token.trim_matches(|ch: char| !ch.is_ascii_digit() && ch != 'x');
        let Some((width, height)) = cleaned.split_once('x') else {
            continue;
        };
        if width.parse::<u32>().is_ok() && height.parse::<u32>().is_ok() {
            return Some(format!("{width}x{height}"));
        }
    }
    None
}

fn parse_density(text: &str) -> Option<u32> {
    text.split(|ch: char| !ch.is_ascii_digit())
        .filter(|value| !value.is_empty())
        .find_map(|value| value.parse::<u32>().ok())
}

fn display_section(text: &str, display_id: u32) -> String {
    let markers = [
        format!("Display #{display_id}"),
        format!("mDisplayId={display_id}"),
        format!("displayId={display_id}"),
        format!("displayId: {display_id}"),
    ];
    let lines = text.lines().collect::<Vec<_>>();
    let Some(index) = lines
        .iter()
        .position(|line| markers.iter().any(|marker| line.contains(marker)))
    else {
        return String::new();
    };
    let start = index.saturating_sub(3);
    let end = (index + 100).min(lines.len());
    lines[start..end].join("\n")
}

fn component_from_line(line: &str) -> Option<String> {
    line.split_whitespace().find_map(|token| {
        let cleaned = token
            .trim_matches(|ch: char| matches!(ch, '{' | '}' | '[' | ']' | '(' | ')' | ',' | ';'));
        if cleaned.contains('/') && !cleaned.starts_with("http") {
            Some(cleaned.trim_start_matches("cmp=").to_string())
        } else {
            None
        }
    })
}

fn running_activity(text: &str, display_id: u32) -> Option<String> {
    let section = display_section(text, display_id);
    [
        "topResumedActivity",
        "mResumedActivity",
        "mCurrentFocus",
        "realActivity=",
        "ActivityRecord",
    ]
    .iter()
    .find_map(|marker| {
        section
            .lines()
            .find(|line| line.contains(marker))
            .and_then(component_from_line)
    })
}

fn observed_windowing_mode(text: &str, display_id: u32) -> String {
    let lower = display_section(text, display_id).to_ascii_lowercase();
    if lower.contains("windowingmode=freeform")
        || lower.contains("windowing_mode_freeform")
        || lower.contains("windowingmode=5")
        || lower.contains("windowingmode = 5")
    {
        "freeform".into()
    } else if lower.contains("desktopmode=true") || lower.contains("windowingmode=desktop") {
        "desktop".into()
    } else if lower.contains("windowingmode=fullscreen")
        || lower.contains("windowing_mode_fullscreen")
        || lower.contains("windowingmode=1")
    {
        "fullscreen".into()
    } else {
        "unknown".into()
    }
}

fn display_name(text: &str, display_id: u32) -> Option<String> {
    let section = display_section(text, display_id);
    section.lines().find_map(|line| {
        let marker = "DisplayDeviceInfo{\"";
        let start = line.find(marker)? + marker.len();
        let end = line[start..].find('"')? + start;
        Some(line[start..end].to_string())
    })
}

fn cap_output(text: String) -> String {
    const MAX_CHARS: usize = 16_000;
    if text.chars().count() <= MAX_CHARS {
        text
    } else {
        text.chars().take(MAX_CHARS).collect::<String>() + "\n… output truncated"
    }
}

fn collect_display_diagnostics(
    serial: &str,
    display_id: u32,
    diagnostics: &mut DesktopDiagnostics,
) {
    diagnostics.display_id = Some(display_id);

    if let Ok(size) = run_adb_shell(serial, &["wm", "size", "-d", &display_id.to_string()]) {
        diagnostics.resolution = parse_resolution(&size).or(diagnostics.resolution.take());
        diagnostics
            .platform_evidence
            .push(format!("wm size -d {display_id}: {}", size.trim()));
    }
    if let Ok(density) = run_adb_shell(serial, &["wm", "density", "-d", &display_id.to_string()]) {
        diagnostics.density = parse_density(&density).or(diagnostics.density);
        diagnostics
            .platform_evidence
            .push(format!("wm density -d {display_id}: {}", density.trim()));
    }

    let activity =
        run_adb_shell(serial, &["dumpsys", "activity", "activities"]).unwrap_or_default();
    let window = run_adb_shell(serial, &["dumpsys", "window", "displays"]).unwrap_or_default();
    let display = run_adb_shell(serial, &["dumpsys", "display"]).unwrap_or_default();
    diagnostics.launcher_activity =
        running_activity(&activity, display_id).or_else(|| running_activity(&window, display_id));
    diagnostics.windowing_mode = observed_windowing_mode(&activity, display_id);
    if diagnostics.windowing_mode == "unknown" {
        diagnostics.windowing_mode = observed_windowing_mode(&window, display_id);
    }
    diagnostics.display_name =
        display_name(&display, display_id).or_else(|| display_name(&window, display_id));
}

fn reader_thread<R: Read + Send + 'static>(
    reader: R,
    prefix: &'static str,
    sender: mpsc::Sender<String>,
    lines: Arc<Mutex<Vec<String>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            if let Ok(mut output) = lines.lock() {
                output.push(format!("[{prefix}] {line}"));
            }
            let _ = sender.send(line);
        }
    })
}

fn joined_output(lines: &Arc<Mutex<Vec<String>>>) -> String {
    lines
        .lock()
        .map(|items| cap_output(items.join("\n")))
        .unwrap_or_else(|_| "Could not read captured scrcpy output.".into())
}

fn format_exit(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit code {code}"),
        None => status.to_string(),
    }
}

fn diagnostic_log_path(kind: &str, serial: &str) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let safe_serial = serial
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    Ok(diagnostics_dir()?.join(format!("{kind}-{safe_serial}-{stamp}.json")))
}

fn write_new_diagnostic_log(
    kind: &str,
    serial: &str,
    diagnostics: &mut DesktopDiagnostics,
) -> Result<(), String> {
    let path = diagnostic_log_path(kind, serial)?;
    diagnostics.log_path = Some(path.display().to_string());
    rewrite_diagnostic_log(diagnostics)
}

fn rewrite_diagnostic_log(diagnostics: &DesktopDiagnostics) -> Result<(), String> {
    let path = diagnostics
        .log_path
        .as_ref()
        .ok_or_else(|| "Desktop diagnostic log path is missing.".to_string())?;
    let json = serde_json::to_string_pretty(diagnostics).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn run_virtual_display_probe(
    serial: &str,
    platform: &PlatformSnapshot,
) -> Result<VirtualDisplayProbe, String> {
    let scrcpy = scrcpy_path()?;
    let mut args = virtual_display_probe_args(serial);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let recording = std::env::temp_dir().join(format!(
        "scrcpy-studio-vd-probe-{}-{stamp}.mp4",
        std::process::id()
    ));
    args.push(format!("--record={}", recording.display()));

    let mut diagnostics = DesktopDiagnostics {
        command: command_preview(&scrcpy, &args),
        relevant_settings: platform.settings.clone(),
        platform_evidence: platform.evidence.clone(),
        ..DesktopDiagnostics::default()
    };

    let mut child = hidden_command(&scrcpy)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start the virtual-display probe: {e}"))?;

    let (sender, receiver) = mpsc::channel();
    let lines = Arc::new(Mutex::new(Vec::new()));
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        readers.push(reader_thread(
            stdout,
            "stdout",
            sender.clone(),
            lines.clone(),
        ));
    }
    if let Some(stderr) = child.stderr.take() {
        readers.push(reader_thread(stderr, "stderr", sender, lines.clone()));
    }

    let deadline = Instant::now() + Duration::from_secs(7);
    let mut created = None;
    let mut early_status = None;
    while Instant::now() < deadline && created.is_none() {
        if let Ok(line) = receiver.recv_timeout(Duration::from_millis(100)) {
            created = parse_new_display(&line);
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            early_status = Some(status);
            break;
        }
    }

    if let Some((width, height, density, display_id)) = created {
        diagnostics.resolution = Some(format!("{width}x{height}"));
        diagnostics.density = Some(density);
        thread::sleep(Duration::from_millis(700));
        collect_display_diagnostics(serial, display_id, &mut diagnostics);
    }

    let status = match early_status {
        Some(status) => status,
        None => child.wait().map_err(|e| e.to_string())?,
    };
    for reader in readers {
        let _ = reader.join();
    }
    let _ = fs::remove_file(&recording);
    diagnostics.exit_result = format_exit(status);
    diagnostics.scrcpy_output = joined_output(&lines);
    let success = status.success() && diagnostics.display_id.is_some();
    if !success && diagnostics.scrcpy_output.trim().is_empty() {
        diagnostics.scrcpy_output = "scrcpy produced no captured output.".into();
    }
    let _ = write_new_diagnostic_log("probe", serial, &mut diagnostics);

    Ok(VirtualDisplayProbe {
        success,
        diagnostics,
    })
}

fn display_id_from_args(args: &[String]) -> Option<u32> {
    args.iter().find_map(|arg| {
        arg.strip_prefix("--display-id=")
            .and_then(|value| value.parse().ok())
    })
}

pub(crate) fn launch_desktop_and_watch(
    path: &Path,
    args: &[String],
    serial: &str,
) -> Result<DesktopLaunchOutcome, String> {
    let profile = inspect_device(serial.to_string())?;
    let platform = collect_platform_snapshot(serial, profile.sdk, &profile.brand);
    let mut diagnostics = DesktopDiagnostics {
        command: command_preview(path, args),
        display_id: display_id_from_args(args),
        relevant_settings: platform.settings,
        platform_evidence: platform.evidence,
        ..DesktopDiagnostics::default()
    };

    let mut child = hidden_command(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let (sender, receiver) = mpsc::channel();
    let lines = Arc::new(Mutex::new(Vec::new()));
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        readers.push(reader_thread(
            stdout,
            "stdout",
            sender.clone(),
            lines.clone(),
        ));
    }
    if let Some(stderr) = child.stderr.take() {
        readers.push(reader_thread(stderr, "stderr", sender, lines.clone()));
    }

    let deadline = Instant::now() + Duration::from_millis(2500);
    let mut early_status = None;
    while Instant::now() < deadline && diagnostics.display_id.is_none() {
        if let Ok(line) = receiver.recv_timeout(Duration::from_millis(100)) {
            if let Some((width, height, density, display_id)) = parse_new_display(&line) {
                diagnostics.display_id = Some(display_id);
                diagnostics.resolution = Some(format!("{width}x{height}"));
                diagnostics.density = Some(density);
            }
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            early_status = Some(status);
            break;
        }
    }

    if let Some(display_id) = diagnostics.display_id {
        thread::sleep(Duration::from_millis(700));
        collect_display_diagnostics(serial, display_id, &mut diagnostics);
    }

    if let Some(status) = early_status.or(child.try_wait().map_err(|e| e.to_string())?) {
        for reader in readers {
            let _ = reader.join();
        }
        diagnostics.exit_result = format_exit(status);
        diagnostics.scrcpy_output = joined_output(&lines);
        let _ = write_new_diagnostic_log("launch-failed", serial, &mut diagnostics);
        return Ok(DesktopLaunchOutcome {
            started: false,
            diagnostics,
        });
    }

    diagnostics.exit_result =
        "running (final exit result will be written when scrcpy closes)".into();
    diagnostics.scrcpy_output = joined_output(&lines);
    write_new_diagnostic_log("launch", serial, &mut diagnostics)?;

    let background_lines = lines.clone();
    let mut final_diagnostics = diagnostics.clone();
    thread::spawn(move || {
        let status = child.wait();
        for reader in readers {
            let _ = reader.join();
        }
        final_diagnostics.exit_result = status
            .map(format_exit)
            .unwrap_or_else(|error| format!("wait failed: {error}"));
        final_diagnostics.scrcpy_output = joined_output(&background_lines);
        let _ = rewrite_diagnostic_log(&final_diagnostics);
    });

    Ok(DesktopLaunchOutcome {
        started: true,
        diagnostics,
    })
}

#[tauri::command]
pub(crate) fn open_desktop_diagnostics() -> Result<String, String> {
    let folder = diagnostics_dir()?;

    #[cfg(target_os = "windows")]
    let result = hidden_command("explorer").arg(&folder).spawn();

    #[cfg(target_os = "macos")]
    let result = hidden_command("open").arg(&folder).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let result = hidden_command("xdg-open").arg(&folder).spawn();

    result.map_err(|e| format!("Could not open Desktop Diagnostics: {e}"))?;
    Ok(folder.display().to_string())
}

#[tauri::command]
pub(crate) fn enable_desktop_experience(serial: String) -> Result<DesktopExperienceResult, String> {
    ensure_ready_device(&serial)?;
    let profile = inspect_device(serial.clone())?;
    if profile.sdk < 29 {
        return Err("Android Desktop Windowing preparation requires Android 10 or newer. Generic scrcpy Virtual Display may still be used without changing these settings.".into());
    }

    save_backup_if_missing(&serial, profile.sdk)?;
    for key in desktop_setting_keys(profile.sdk) {
        put_global_setting(&serial, key, "1")?;
    }
    let _ = run_adb_shell(&serial, &["sync"]);

    let settings = read_setting_diagnostics(&serial, profile.sdk);
    if !developer_settings_applied(&settings, profile.sdk) {
        return Err("Android did not retain all requested desktop developer settings.".into());
    }
    reboot_device(&serial)?;

    Ok(DesktopExperienceResult {
        prepared: false,
        backup_available: true,
        reboot_started: true,
        message: "Android freeform, resizable-window, non-resizable multi-window, and secondary-display desktop settings were applied. The phone is restarting; SCRCPY Studio will reconnect and verify the created display's real windowing mode. This does not enable Samsung DeX.".into(),
    })
}

#[tauri::command]
pub(crate) fn restore_desktop_experience(
    serial: String,
) -> Result<DesktopExperienceResult, String> {
    ensure_ready_device(&serial)?;
    let device_key = stable_device_key(&serial);
    let mut store = read_backup_store()?;
    let backup = store.devices.get(&device_key).cloned().ok_or_else(|| {
        "No original Desktop developer settings backup exists for this phone.".to_string()
    })?;

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
    ensure_ready_device(&serial)?;
    let profile = inspect_device(serial.clone())?;
    let wireless = profile.connection_kind == "wireless";
    let (recommended_width, recommended_height, recommended_density) =
        recommended_desktop_geometry(&profile.brand, wireless);
    let launcher_package = default_launcher_package(&serial);
    let backup = backup_available(&serial);
    let platform = collect_platform_snapshot(&serial, profile.sdk, &profile.brand);
    let help = scrcpy_help()?;

    let mut diagnostics = DesktopDiagnostics {
        relevant_settings: platform.settings.clone(),
        platform_evidence: platform.evidence.clone(),
        ..DesktopDiagnostics::default()
    };

    if profile.sdk < 21 || !help.contains("--new-display") {
        let message = if profile.sdk < 21 {
            format!(
                "Android {} (API {}) is below scrcpy 4.1's Android 5.0 requirement.",
                profile.android_version, profile.sdk
            )
        } else {
            "The installed scrcpy runtime does not expose --new-display. Update the runtime to use Virtual Display.".into()
        };
        diagnostics.exit_result = "probe not run".into();
        return Ok(DesktopCapabilities {
            supported: false,
            environment_kind: "unavailable".into(),
            environment_label: "Unavailable".into(),
            launch_label: "Virtual Display unavailable".into(),
            virtual_display_supported: false,
            android_desktop_windowing_available: platform.android_desktop_available,
            android_desktop_windowing_active: false,
            samsung_dex_available: platform.samsung_dex_available,
            samsung_dex_active: platform.samsung_dex_active,
            existing_display_id: None,
            recommended_width,
            recommended_height,
            recommended_density,
            flex_supported: false,
            system_decorations_supported: false,
            keep_content_supported: false,
            launcher_package,
            startup_package: String::new(),
            desktop_experience_prepared: false,
            desktop_experience_can_prepare: false,
            desktop_experience_backup_available: backup,
            desktop_experience_summary: message.clone(),
            message,
            diagnostics,
        });
    }

    let probe = run_virtual_display_probe(&serial, &platform)?;
    diagnostics = probe.diagnostics;
    let android_desktop_active =
        matches!(diagnostics.windowing_mode.as_str(), "freeform" | "desktop");
    let dex_capturable = platform.samsung_dex_active && platform.samsung_dex_display_id.is_some();

    let (environment_kind, environment_label, launch_label, existing_display_id, summary, message): (
        String,
        String,
        String,
        Option<u32>,
        String,
        String,
    ) = if dex_capturable {
        (
            "samsung_dex".into(),
            "Samsung DeX".into(),
            "Open Samsung DeX Display".into(),
            platform.samsung_dex_display_id,
            "Samsung DeX is active on an external display and exposes a display ID that scrcpy can capture.".into(),
            "Samsung DeX was detected as an active external display. SCRCPY Studio will mirror that display instead of creating another virtual display.".into(),
        )
    } else if android_desktop_active {
        (
            "android_desktop_windowing".into(),
            "Android Desktop Windowing".into(),
            "Launch Android Desktop".into(),
            None,
            "The temporary scrcpy display actually entered freeform/desktop windowing.".into(),
            "Android Desktop Windowing was verified from the created display's real WindowManager state.".into(),
        )
    } else if probe.success {
        let (summary, message) = if platform.samsung_dex_available {
            (
                "Virtual Display is available. Samsung DeX exists on this phone but is not active on, or exposed to, the scrcpy-created display.",
                "Virtual Display is ready. Samsung firmware kept the scrcpy display in phone-style windowing; current One UI requires a real HDMI or Miracast display to start DeX.",
            )
        } else if platform.android_desktop_available {
            (
                "Virtual Display is available. Android desktop-related configuration exists, but the created display did not enter freeform/desktop windowing.",
                "Virtual Display is ready, but the actual display stayed in fullscreen/phone-style windowing. SCRCPY Studio will not label it Android Desktop.",
            )
        } else {
            (
                "Virtual Display is available; no real desktop shell was observed on the created display.",
                "Virtual Display is ready. This device does not expose verified Android desktop windowing or Samsung DeX to the scrcpy-created display.",
            )
        };
        (
            "virtual_display".into(),
            "Virtual Display".into(),
            "Launch Virtual Display".into(),
            None,
            summary.into(),
            message.into(),
        )
    } else {
        let detail = if diagnostics.scrcpy_output.trim().is_empty() {
            diagnostics.exit_result.clone()
        } else {
            compact_error(&diagnostics.scrcpy_output)
        };
        (
            "unavailable".into(),
            "Unavailable".into(),
            "Virtual Display unavailable".into(),
            None,
            "scrcpy could not create a diagnostic virtual display.".into(),
            format!("Virtual Display probe failed: {detail}"),
        )
    };

    let supported = probe.success || dex_capturable;
    let settings_applied = developer_settings_applied(&platform.settings, profile.sdk);
    let can_prepare = can_prepare_desktop_experience(
        supported,
        dex_capturable,
        profile.sdk,
        platform.android_desktop_available,
        settings_applied,
    );

    Ok(DesktopCapabilities {
        supported,
        environment_kind,
        environment_label,
        launch_label,
        virtual_display_supported: probe.success,
        android_desktop_windowing_available: platform.android_desktop_available,
        android_desktop_windowing_active: android_desktop_active,
        samsung_dex_available: platform.samsung_dex_available,
        samsung_dex_active: dex_capturable,
        existing_display_id,
        recommended_width,
        recommended_height,
        recommended_density,
        flex_supported: help.contains("--flex-display") && existing_display_id.is_none(),
        system_decorations_supported: help.contains("--no-vd-system-decorations")
            && existing_display_id.is_none(),
        keep_content_supported: help.contains("--no-vd-destroy-content")
            && existing_display_id.is_none(),
        launcher_package,
        startup_package: String::new(),
        desktop_experience_prepared: android_desktop_active || dex_capturable,
        desktop_experience_can_prepare: can_prepare,
        desktop_experience_backup_available: backup,
        desktop_experience_summary: summary,
        message,
        diagnostics,
    })
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
    fn parses_scrcpy_41_new_display_log() {
        let parsed = parse_new_display("[server] INFO: New display: 1024x640/160 (id=7)");
        assert_eq!(parsed, Some((1024, 640, 160, 7)));
    }

    #[test]
    fn detects_actual_freeform_windowing_for_the_created_display() {
        let dump = "Display #0\n  windowingMode=fullscreen\nDisplay #7\n  TaskDisplayArea windowingMode=freeform\n  topResumedActivity=ActivityRecord{abc u0 com.android.settings/.Settings t12}";
        assert_eq!(observed_windowing_mode(dump, 7), "freeform");
        assert_eq!(
            running_activity(dump, 7).as_deref(),
            Some("com.android.settings/.Settings")
        );
    }

    #[test]
    fn global_flags_are_not_the_desktop_windowing_verdict() {
        let settings = vec![
            DesktopSettingDiagnostic {
                key: FORCE_DESKTOP.into(),
                value: Some("1".into()),
            },
            DesktopSettingDiagnostic {
                key: ENABLE_FREEFORM.into(),
                value: Some("1".into()),
            },
            DesktopSettingDiagnostic {
                key: FORCE_RESIZABLE.into(),
                value: Some("1".into()),
            },
            DesktopSettingDiagnostic {
                key: ENABLE_NON_RESIZABLE_MULTI_WINDOW.into(),
                value: Some("1".into()),
            },
            DesktopSettingDiagnostic {
                key: OVERRIDE_DESKTOP_EXPERIENCE.into(),
                value: Some("1".into()),
            },
        ];
        assert!(developer_settings_applied(&settings, 36));
        assert_eq!(
            observed_windowing_mode("Display #3\nwindowingMode=fullscreen", 3),
            "fullscreen"
        );
    }

    #[test]
    fn android_16_samsung_can_prepare_generic_freeform_windowing() {
        assert!(can_prepare_desktop_experience(true, false, 36, true, false));
    }

    #[test]
    fn preparation_is_hidden_only_after_all_settings_are_applied() {
        assert!(!can_prepare_desktop_experience(true, false, 36, true, true));
    }

    #[test]
    fn preparation_requires_every_setting_it_writes() {
        let settings = vec![
            DesktopSettingDiagnostic {
                key: FORCE_DESKTOP.into(),
                value: Some("1".into()),
            },
            DesktopSettingDiagnostic {
                key: ENABLE_FREEFORM.into(),
                value: Some("1".into()),
            },
            DesktopSettingDiagnostic {
                key: FORCE_RESIZABLE.into(),
                value: Some("1".into()),
            },
            DesktopSettingDiagnostic {
                key: ENABLE_NON_RESIZABLE_MULTI_WINDOW.into(),
                value: Some("0".into()),
            },
            DesktopSettingDiagnostic {
                key: OVERRIDE_DESKTOP_EXPERIENCE.into(),
                value: Some("1".into()),
            },
        ];
        assert!(!developer_settings_applied(&settings, 36));
    }

    #[test]
    fn recognizes_active_samsung_dex_service_state() {
        let dump = "SemDesktopModeState{ enabled=4, state=0 } mDisplayId=2";
        assert!(is_samsung_dex_active(dump));
        assert_eq!(parse_samsung_dex_display_id(dump), Some(2));
    }

    #[test]
    fn finds_samsung_dex_launcher_on_a_secondary_display() {
        let dump = "Display #0\n  com.sec.android.app.launcher/.Launcher\nDisplay #2\n  topResumedActivity=ActivityRecord{abc u0 com.sec.android.app.desktoplauncher/.DesktopLauncher t4}";
        assert_eq!(
            component_display_id(dump, &["com.sec.android.app.desktoplauncher"]),
            Some(2)
        );
    }

    #[test]
    fn virtual_display_probe_uses_an_app_without_disabling_control() {
        let args = virtual_display_probe_args("ABC");
        assert!(args.iter().any(|arg| arg.starts_with("--start-app=")));
        assert!(!args.contains(&"--no-control".to_string()));
    }

    #[test]
    fn android_16_diagnostics_include_current_and_legacy_switches() {
        let keys = diagnostic_setting_keys(36);
        assert!(keys.contains(&FORCE_DESKTOP));
        assert!(keys.contains(&ENABLE_FREEFORM));
        assert!(keys.contains(&OVERRIDE_DESKTOP_EXPERIENCE));
        assert!(keys.contains(&OVERRIDE_DESKTOP_MODE));
    }
}
