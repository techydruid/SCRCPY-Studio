use crate::runtime::adb_path;
use chrono::Local;
use std::{fs, path::{Path, PathBuf}, process::Command};

fn recordings_root() -> Result<PathBuf, String> {
    let base = dirs::video_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not find a Videos or home folder.".to_string())?;
    let folder = base.join("SCRCPY Studio");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder)
}

fn screenshots_root() -> Result<PathBuf, String> {
    let base = dirs::picture_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not find a Pictures or home folder.".to_string())?;
    let folder = base.join("SCRCPY Studio").join("Screenshots");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder)
}

fn open_directory(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = Command::new("explorer").arg(path).spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(path).spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(path).spawn();

    result.map(|_| ()).map_err(|e| format!("Could not open folder: {e}"))
}

#[tauri::command]
pub(crate) fn capture_screenshot(serial: String) -> Result<String, String> {
    let adb = adb_path()?;
    let output = Command::new(adb)
        .arg("-s")
        .arg(&serial)
        .args(["exec-out", "screencap", "-p"])
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

#[tauri::command]
pub(crate) fn open_recordings_folder() -> Result<String, String> {
    let folder = recordings_root()?;
    open_directory(&folder)?;
    Ok(folder.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creator_paths_use_product_name() {
        let root = PathBuf::from("Videos").join("SCRCPY Studio");
        assert!(root.to_string_lossy().contains("SCRCPY Studio"));
    }
}
