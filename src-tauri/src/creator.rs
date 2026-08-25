use crate::{commands::hidden_command, runtime::adb_path};
use chrono::Local;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn media_root() -> Result<PathBuf, String> {
    let base = dirs::video_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not find a Videos or home folder.".to_string())?;
    let folder = base.join("SCRCPY Studio");
    fs::create_dir_all(folder.join("Recordings")).map_err(|e| e.to_string())?;
    fs::create_dir_all(folder.join("Screenshots")).map_err(|e| e.to_string())?;
    Ok(folder)
}

pub(crate) fn recordings_root() -> Result<PathBuf, String> {
    Ok(media_root()?.join("Recordings"))
}

fn screenshots_root() -> Result<PathBuf, String> {
    Ok(media_root()?.join("Screenshots"))
}

fn open_directory(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = hidden_command("explorer").arg(path).spawn();

    #[cfg(target_os = "macos")]
    let result = hidden_command("open").arg(path).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let result = hidden_command("xdg-open").arg(path).spawn();

    result
        .map(|_| ())
        .map_err(|e| format!("Could not open folder: {e}"))
}

fn adb_shell_text(serial: &str, args: &[&str]) -> Result<String, String> {
    let adb = adb_path()?;
    let output = hidden_command(adb)
        .arg("-s")
        .arg(serial)
        .arg("shell")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            format!("ADB command failed: {}", args.join(" "))
        } else {
            detail
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn logical_display_device(text: &str, display_id: u32) -> Option<(String, String)> {
    let marker = format!("mDisplayId={display_id}");
    let lines = text.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        if line.trim() != marker {
            continue;
        }
        for candidate in lines.iter().skip(index + 1).take(8) {
            let trimmed = candidate.trim();
            let Some(value) = trimmed.strip_prefix("mPrimaryDisplayDevice=") else {
                continue;
            };
            let open = value.rfind('(')?;
            let unique_id = value[open + 1..].strip_suffix(')')?;
            return Some((value[..open].to_string(), unique_id.to_string()));
        }
    }
    None
}

fn surfaceflinger_display_id(text: &str, display_name: &str) -> Option<String> {
    let name_marker = format!("displayName=\"{display_name}\"");
    text.lines()
        .find(|line| line.contains(&name_marker))
        .and_then(|line| line.split_whitespace().nth(1))
        .filter(|value| value.chars().all(|character| character.is_ascii_digit()))
        .map(str::to_string)
}

fn screenshot_display_id(serial: &str, logical_display_id: u32) -> Result<String, String> {
    let display_dump = adb_shell_text(serial, &["dumpsys", "display"])?;
    let (display_name, unique_id) = logical_display_device(&display_dump, logical_display_id)
        .ok_or_else(|| format!("Android display {logical_display_id} is no longer active."))?;

    if let Some(local_id) = unique_id.strip_prefix("local:") {
        if local_id.chars().all(|character| character.is_ascii_digit()) {
            return Ok(local_id.to_string());
        }
    }

    let surface_dump = adb_shell_text(serial, &["dumpsys", "SurfaceFlinger", "--display-id"])?;
    surfaceflinger_display_id(&surface_dump, &display_name).ok_or_else(|| {
        format!(
            "Android display {logical_display_id} ({display_name}) could not be mapped for screenshot capture. Relaunch Desktop Mode and try again."
        )
    })
}

#[tauri::command(async)]
pub(crate) fn capture_screenshot(
    serial: String,
    display_id: Option<u32>,
) -> Result<String, String> {
    let adb = adb_path()?;
    let capture_id = display_id
        .map(|logical_id| screenshot_display_id(&serial, logical_id))
        .transpose()?;
    let mut command = hidden_command(adb);
    command.arg("-s").arg(&serial).args(["exec-out", "screencap"]);
    if let Some(capture_id) = capture_id.as_deref() {
        command.args(["-d", capture_id]);
    }
    let output = command
        .arg("-p")
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            "Android screenshot capture failed.".into()
        } else {
            detail
        });
    }
    if output.stdout.is_empty() {
        return Err("Android returned an empty screenshot.".into());
    }

    let folder = screenshots_root()?;
    let path = folder.join(format!(
        "SCRCPY-Studio-{}.png",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));
    fs::write(&path, output.stdout).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command(async)]
pub(crate) fn open_media_folder() -> Result<String, String> {
    let folder = media_root()?;
    open_directory(&folder)?;
    Ok(folder.display().to_string())
}

#[tauri::command(async)]
pub(crate) fn open_recordings_folder() -> Result<String, String> {
    let folder = recordings_root()?;
    open_directory(&folder)?;
    Ok(folder.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_layout_has_recordings_and_screenshots() {
        let root = PathBuf::from("Videos").join("SCRCPY Studio");
        assert_eq!(root.join("Recordings").file_name().unwrap(), "Recordings");
        assert_eq!(root.join("Screenshots").file_name().unwrap(), "Screenshots");
    }

    #[test]
    fn maps_a_logical_virtual_display_to_its_surfaceflinger_id() {
        let display = r#"
    mDisplayId=42
    mPrimaryDisplayDevice=scrcpy(virtual:com.android.shell,2000,scrcpy,36)
    mIsEnabled=true
"#;
        let surface = r#"
Display 4630947232161729154 (HWC display 0): port=130 displayName=""
Display 11529215050104701316 (Virtual display): displayName="scrcpy"
"#;
        let (name, unique_id) = logical_display_device(display, 42).unwrap();
        assert_eq!(name, "scrcpy");
        assert_eq!(unique_id, "virtual:com.android.shell,2000,scrcpy,36");
        assert_eq!(
            surfaceflinger_display_id(surface, &name).as_deref(),
            Some("11529215050104701316")
        );
    }

    #[test]
    fn reads_a_physical_surface_id_from_the_logical_display_device() {
        let display = r#"
    mDisplayId=0
    mPrimaryDisplayDevice=Built-in Screen(local:4630947232161729154)
    mIsEnabled=true
"#;
        assert_eq!(
            logical_display_device(display, 0),
            Some((
                "Built-in Screen".into(),
                "local:4630947232161729154".into()
            ))
        );
    }
}

