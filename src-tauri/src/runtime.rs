use crate::models::RuntimeStatus;
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

fn executable_names(base: &str) -> Vec<OsString> {
    #[cfg(target_os = "windows")]
    {
        vec![OsString::from(format!("{base}.exe")), OsString::from(base)]
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![OsString::from(base)]
    }
}

pub(crate) fn resolve_binary(base: &str) -> Option<PathBuf> {
    let names = executable_names(base);

    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            for folder in [
                parent.to_path_buf(),
                parent.join("runtime"),
                parent.join("scrcpy"),
            ] {
                for name in &names {
                    let candidate = folder.join(name);
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    if let Some(paths) = env::var_os("PATH") {
        for folder in env::split_paths(&paths) {
            for name in &names {
                let candidate = folder.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

pub(crate) fn output_text(mut command: Command) -> Result<String, String> {
    let output = command.output().map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    if output.status.success() {
        Ok(text)
    } else if text.is_empty() {
        Err(format!("Command failed with {}", output.status))
    } else {
        Err(text)
    }
}

fn tool_version(path: &Path, arg: &str) -> Option<String> {
    let mut command = Command::new(path);
    command.arg(arg);
    output_text(command)
        .ok()
        .and_then(|text| text.lines().next().map(str::to_owned))
}

#[tauri::command]
pub(crate) fn runtime_status() -> RuntimeStatus {
    let adb = resolve_binary("adb");
    let scrcpy = resolve_binary("scrcpy");
    RuntimeStatus {
        adb_found: adb.is_some(),
        scrcpy_found: scrcpy.is_some(),
        adb_path: adb.as_ref().map(|p| p.display().to_string()),
        scrcpy_path: scrcpy.as_ref().map(|p| p.display().to_string()),
        adb_version: adb.as_ref().and_then(|p| tool_version(p, "version")),
        scrcpy_version: scrcpy.as_ref().and_then(|p| tool_version(p, "--version")),
    }
}

pub(crate) fn adb_path() -> Result<PathBuf, String> {
    resolve_binary("adb").ok_or_else(|| {
        "ADB was not found. Install Android Platform Tools or place adb in SCRCPY Studio's runtime folder."
            .into()
    })
}

pub(crate) fn scrcpy_path() -> Result<PathBuf, String> {
    resolve_binary("scrcpy").ok_or_else(|| {
        "scrcpy was not found. Install official scrcpy or place it in SCRCPY Studio's runtime folder."
            .into()
    })
}
