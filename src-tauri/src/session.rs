use crate::{
    creator::recordings_root,
    devices::list_devices,
    models::{LaunchConfig, LaunchResult},
    preferences::remember_successful_profile,
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
    let folder = recordings_root()?.join(Local::now().format("%Y-%m-%d").to_string());
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder.join(format!(
        "SCRCPY-Studio-{}.mp4",
        Local::now().format("%H-%M-%S")
    )))
}

fn valid_camera_facing(value: &str) -> bool {
    matches!(value, "front" | "back" | "external")
}

fn build_args(config: &LaunchConfig, recording: Option<&Path>) -> Vec<String> {
    let mut args = vec!["-s".into(), config.serial.clone()];

    match config.mode.as_str() {
        "camera" => {
            args.push("--video-source=camera".into());
            if let Some(id) = config.camera_id.as_deref().filter(|id| !id.trim().is_empty()) {
                args.push(format!("--camera-id={}", id.trim()));
            } else if let Some(facing) = config
                .camera_facing
                .as_deref()
                .filter(|facing| valid_camera_facing(facing))
            {
                args.push(format!("--camera-facing={facing}"));
            }
            if config.max_size > 0 {
                args.push(format!("--max-size={}", config.max_size));
            }
            if config.max_fps > 0 {
                args.push(format!("--camera-fps={}", config.max_fps));
            }
            if let Some(zoom) = config.camera_zoom.filter(|zoom| zoom.is_finite() && *zoom > 0.0) {
                args.push(format!("--camera-zoom={zoom:.2}"));
            }
            if config.camera_torch {
                args.push("--camera-torch".into());
            }
        }
        "desktop" => {
            args.push("--new-display".into());
            if config.max_fps > 0 {
                args.push(format!("--max-fps={}", config.max_fps));
            }
        }
        _ => {
            if config.max_size > 0 {
                args.push(format!("--max-size={}", config.max_size));
            }
            if config.max_fps > 0 {
                args.push(format!("--max-fps={}", config.max_fps));
            }
        }
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

fn push_variant(variants: &mut Vec<LaunchConfig>, mutator: impl FnOnce(&mut LaunchConfig)) {
    let mut next = variants.last().cloned().expect("at least one launch variant");
    mutator(&mut next);
    variants.push(next);
}

pub(crate) fn fallback_configs(original: &LaunchConfig) -> Vec<LaunchConfig> {
    let mut variants = vec![original.clone()];

    if original.mode == "camera" {
        if original.camera_torch {
            push_variant(&mut variants, |next| next.camera_torch = false);
        }
        if original.camera_zoom.unwrap_or(1.0) > 1.01 {
            push_variant(&mut variants, |next| next.camera_zoom = None);
        }
        if original.codec == "h265" {
            push_variant(&mut variants, |next| next.codec = "h264".into());
        }
        if original.max_fps > 30 {
            push_variant(&mut variants, |next| next.max_fps = 30);
        }
        if original.max_size > 1280 {
            push_variant(&mut variants, |next| next.max_size = 1280);
        }
        if original.camera_id.is_some() && original.camera_facing.is_some() {
            push_variant(&mut variants, |next| next.camera_id = None);
        }
        return variants;
    }

    if original.codec == "h265" {
        push_variant(&mut variants, |next| next.codec = "h264".into());
    }
    if original.max_size > 1280 {
        push_variant(&mut variants, |next| next.max_size = 1280);
    }
    if original.max_fps > 30 {
        push_variant(&mut variants, |next| next.max_fps = 30);
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
            let remembered = remember_successful_profile(variant).is_ok();
            return Ok(LaunchResult {
                started: true,
                fallback_used,
                attempts: index + 1,
                command_preview: shell_preview(&scrcpy, &args),
                recording_path: recording.as_ref().map(|p| p.display().to_string()),
                message: if fallback_used {
                    if config.mode == "camera" {
                        format!(
                            "Camera opened after SCRCPY Studio automatically found a safer working combination on attempt {} of {}.",
                            index + 1,
                            total
                        )
                    } else if remembered {
                        format!(
                            "Session recovered on attempt {} of {}. This working profile is now remembered for this device.",
                            index + 1,
                            total
                        )
                    } else {
                        format!(
                            "Session started after SCRCPY Studio automatically recovered on attempt {} of {}.",
                            index + 1,
                            total
                        )
                    }
                } else if config.mode == "camera" {
                    "Camera opened with the selected smart camera profile.".into()
                } else {
                    "Session started with the selected smart profile.".into()
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

    fn sample_config(mode: &str) -> LaunchConfig {
        LaunchConfig {
            serial: "ABC".into(),
            mode: mode.into(),
            max_size: 1920,
            max_fps: 60,
            codec: "h265".into(),
            audio: true,
            stay_awake: true,
            turn_screen_off: false,
            show_touches: false,
            record: false,
            fullscreen: false,
            camera_id: None,
            camera_facing: None,
            camera_zoom: None,
            camera_torch: false,
        }
    }

    #[test]
    fn generates_progressively_safer_fallbacks() {
        let config = sample_config("mirror");
        let variants = fallback_configs(&config);
        assert_eq!(variants.len(), 4);
        assert_eq!(variants[1].codec, "h264");
        assert_eq!(variants[2].max_size, 1280);
        assert_eq!(variants[3].max_fps, 30);
    }

    #[test]
    fn camera_args_use_camera_specific_fps_and_controls() {
        let mut config = sample_config("camera");
        config.camera_id = Some("2".into());
        config.camera_facing = Some("back".into());
        config.camera_zoom = Some(2.0);
        config.camera_torch = true;
        let args = build_args(&config, None);
        assert!(args.contains(&"--video-source=camera".to_string()));
        assert!(args.contains(&"--camera-id=2".to_string()));
        assert!(args.contains(&"--camera-fps=60".to_string()));
        assert!(args.contains(&"--camera-zoom=2.00".to_string()));
        assert!(args.contains(&"--camera-torch".to_string()));
        assert!(!args.iter().any(|arg| arg.starts_with("--max-fps=")));
    }

    #[test]
    fn camera_fallbacks_remove_risky_options() {
        let mut config = sample_config("camera");
        config.camera_id = Some("0".into());
        config.camera_facing = Some("back".into());
        config.camera_zoom = Some(2.0);
        config.camera_torch = true;
        let variants = fallback_configs(&config);
        assert!(variants.iter().any(|item| !item.camera_torch));
        assert!(variants.iter().any(|item| item.camera_zoom.is_none()));
        assert!(variants.iter().any(|item| item.codec == "h264"));
        assert!(variants.iter().any(|item| item.max_fps == 30));
        assert!(variants.iter().any(|item| item.max_size == 1280));
        assert!(variants.iter().any(|item| item.camera_id.is_none()));
    }
}
