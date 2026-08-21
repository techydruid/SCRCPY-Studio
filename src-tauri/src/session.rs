use crate::{
    devices::list_devices,
    models::{LaunchConfig, LaunchResult},
    runtime::scrcpy_path,
};
use chrono::Local;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

fn recording_path() -> Result<PathBuf, String> {
    let base = dirs::video_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "Could not find a Videos or home folder for recordings.".to_string())?;
    let folder = base
        .join("SCRCPY Studio")
        .join(Local::now().format("%Y-%m-%d").to_string());
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder.join(format!(
        "SCRCPY-Studio-{}.mp4",
        Local::now().format("%H-%M-%S")
    )))
}

fn build_args(config: &LaunchConfig, recording: Option<&Path>) -> Vec<String> {
    let mut args = vec!["-s".into(), config.serial.clone()];

    match config.mode.as_str() {
        "camera" => {
            args.push("--video-source=camera".into());
            let camera_size = if config.max_size >= 1920 {
                "1920x1080"
            } else {
                "1280x720"
            };
            args.push(format!("--camera-size={camera_size}"));
        }
        "desktop" => args.push("--new-display".into()),
        _ => {
            if config.max_size > 0 {
                args.push(format!("--max-size={}", config.max_size));
            }
        }
    }

    if config.max_fps > 0 {
        args.push(format!("--max-fps={}", config.max_fps));
    }
    args.push(format!("--video-codec={}", config.codec));
    if !config.audio {
        args.push("--no-audio".into());
    }
    if config.stay_awake && config.mode != "camera" {
        args.push("--stay-awake".into());
    }
    if config.turn_screen_off && config.mode != "camera" {
        args.push("--turn-screen-off".into());
    }
    if config.show_touches && config.mode != "camera" {
        args.push("--show-touches".into());
    }
    if config.fullscreen {
        args.push("--fullscreen".into());
    }
    if let Some(path) = recording {
        args.push(format!("--record={}", path.display()));
    }
    args.push(format!("--window-title=SCRCPY Studio · {}", config.serial));
    args
}

fn shell_preview(path: &Path, args: &[String]) -> String {
    let quoted = args
        .iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{}\"", arg.replace('"', "\\\""))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{} {}", path.display(), quoted)
}

fn launch_and_watch(path: &Path, args: &[String]) -> Result<bool, String> {
    let mut child = Command::new(path)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;

    for _ in 0..6 {
        thread::sleep(Duration::from_millis(150));
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(_) => return Ok(false),
            None => continue,
        }
    }
    Ok(true)
}

pub(crate) fn fallback_configs(original: &LaunchConfig) -> Vec<LaunchConfig> {
    let mut variants = vec![original.clone()];

    if original.codec == "h265" {
        let mut next = original.clone();
        next.codec = "h264".into();
        variants.push(next);
    }
    if original.max_size > 1280 && original.mode != "camera" {
        let mut next = variants.last().cloned().unwrap_or_else(|| original.clone());
        next.max_size = 1280;
        variants.push(next);
    }
    if original.max_fps > 30 {
        let mut next = variants.last().cloned().unwrap_or_else(|| original.clone());
        next.max_fps = 30;
        variants.push(next);
    }
    variants
}

#[tauri::command]
pub(crate) fn launch_session(config: LaunchConfig) -> Result<LaunchResult, String> {
    let scrcpy = scrcpy_path()?;
    let devices = list_devices()?;
    let device = devices
        .iter()
        .find(|d| d.serial == config.serial)
        .ok_or_else(|| "Selected device is no longer connected.".to_string())?;
    if device.state != "device" {
        return Err(format!(
            "Device is '{}', not ready. Run Connection Doctor for the next step.",
            device.state
        ));
    }

    let recording = if config.record {
        Some(recording_path()?)
    } else {
        None
    };
    let variants = fallback_configs(&config);
    let total = variants.len();

    for (index, variant) in variants.iter().enumerate() {
        let args = build_args(variant, recording.as_deref());
        if launch_and_watch(&scrcpy, &args)? {
            let fallback_used = index > 0;
            return Ok(LaunchResult {
                started: true,
                fallback_used,
                attempts: index + 1,
                command_preview: shell_preview(&scrcpy, &args),
                recording_path: recording.as_ref().map(|p| p.display().to_string()),
                message: if fallback_used {
                    format!(
                        "Session started after SCRCPY Studio automatically recovered on attempt {} of {}.",
                        index + 1,
                        total
                    )
                } else {
                    "Session started with the recommended profile.".into()
                },
            });
        }
    }

    Err(format!(
        "scrcpy exited immediately after {} smart attempts. Open Connection Doctor and verify the device/runtime.",
        total
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_progressively_safer_fallbacks() {
        let config = LaunchConfig {
            serial: "ABC".into(),
            mode: "mirror".into(),
            max_size: 1920,
            max_fps: 60,
            codec: "h265".into(),
            audio: true,
            stay_awake: true,
            turn_screen_off: false,
            show_touches: false,
            record: false,
            fullscreen: false,
        };
        let variants = fallback_configs(&config);
        assert_eq!(variants.len(), 4);
        assert_eq!(variants[1].codec, "h264");
        assert_eq!(variants[2].max_size, 1280);
        assert_eq!(variants[3].max_fps, 30);
    }
}
