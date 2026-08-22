use crate::{
    commands::hidden_command,
    creator::recordings_root,
    desktop::launch_desktop_and_watch,
    devices::list_devices,
    models::{DesktopDiagnostics, LaunchConfig, LaunchResult},
    preferences::remember_successful_profile,
    runtime::scrcpy_path,
};
use chrono::Local;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
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

fn validate_launch_mode(config: &LaunchConfig, requested_mode: &str) -> Result<(), String> {
    if !matches!(requested_mode, "mirror" | "creator" | "camera" | "desktop") {
        return Err("Unknown session mode.".into());
    }
    if config.mode != requested_mode {
        return Err(
            "The selected mode changed while its settings were loading. Wait a moment and try again."
                .into(),
        );
    }
    Ok(())
}

fn safe_desktop_dimension(value: Option<u32>, fallback: u32) -> u32 {
    value
        .filter(|value| (480..=7680).contains(value))
        .unwrap_or(fallback)
}

fn safe_desktop_density(value: Option<u32>) -> u32 {
    value
        .filter(|value| (120..=640).contains(value))
        .unwrap_or(240)
}

fn build_args(config: &LaunchConfig, recording: Option<&Path>) -> Vec<String> {
    let mut args = vec!["-s".into(), config.serial.clone()];

    match config.mode.as_str() {
        "camera" => {
            args.push("--video-source=camera".into());
            if let Some(id) = config
                .camera_id
                .as_deref()
                .filter(|id| !id.trim().is_empty())
            {
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
            if let Some(zoom) = config
                .camera_zoom
                .filter(|zoom| zoom.is_finite() && *zoom > 0.0)
            {
                args.push(format!("--camera-zoom={zoom:.2}"));
            }
            if config.camera_torch {
                args.push("--camera-torch".into());
            }
        }
        "desktop" => {
            if config.desktop_environment.as_deref() == Some("samsung_dex") {
                if let Some(display_id) = config.desktop_display_id {
                    args.push(format!("--display-id={display_id}"));
                }
            } else {
                let width = safe_desktop_dimension(
                    config.desktop_width,
                    if config.max_size >= 1920 { 1920 } else { 1280 },
                );
                let height = safe_desktop_dimension(
                    config.desktop_height,
                    if width >= 1920 { 1080 } else { 720 },
                );
                let density = safe_desktop_density(config.desktop_density);
                args.push(format!("--new-display={width}x{height}/{density}"));
                if config.desktop_flex {
                    args.push("--flex-display".into());
                }
                if config.desktop_no_decorations {
                    args.push("--no-vd-system-decorations".into());
                }
                if config.desktop_keep_content {
                    args.push("--no-vd-destroy-content".into());
                }
                if let Some(package) = config
                    .desktop_start_app
                    .as_deref()
                    .map(str::trim)
                    .filter(|package| !package.is_empty())
                {
                    args.push(format!("--start-app={package}"));
                }
                args.push("--display-ime-policy=local".into());
            }
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
        args.push(if config.mode == "desktop" {
            "--keep-active".into()
        } else {
            "--stay-awake".into()
        });
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
    let mut child = hidden_command(path)
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
    let mut next = variants
        .last()
        .cloned()
        .expect("at least one launch variant");
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

    if original.mode == "desktop" {
        if original.desktop_environment.as_deref() != Some("samsung_dex") {
            if original.desktop_no_decorations {
                push_variant(&mut variants, |next| next.desktop_no_decorations = false);
            }
            if original.desktop_flex {
                push_variant(&mut variants, |next| next.desktop_flex = false);
            }
            if original.desktop_start_app.as_deref() != Some("com.android.settings") {
                push_variant(&mut variants, |next| {
                    next.desktop_start_app = Some("com.android.settings".into())
                });
            }
            if original.desktop_width.unwrap_or(1920) > 1280
                || original.desktop_height.unwrap_or(1080) > 720
            {
                push_variant(&mut variants, |next| {
                    next.desktop_width = Some(1280);
                    next.desktop_height = Some(720);
                    next.desktop_density = Some(200);
                });
            }
        }
        if original.codec == "h265" {
            push_variant(&mut variants, |next| next.codec = "h264".into());
        }
        if original.max_fps > 30 {
            push_variant(&mut variants, |next| next.max_fps = 30);
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
pub(crate) fn launch_session(
    config: LaunchConfig,
    requested_mode: String,
) -> Result<LaunchResult, String> {
    validate_launch_mode(&config, &requested_mode)?;
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
    let mut last_desktop_diagnostics: Option<DesktopDiagnostics> = None;

    for (index, variant) in variants.iter().enumerate() {
        let args = build_args(variant, recording.as_deref());
        let started = if config.mode == "desktop" {
            let outcome = launch_desktop_and_watch(&scrcpy, &args, &config.serial)?;
            last_desktop_diagnostics = Some(outcome.diagnostics);
            outcome.started
        } else {
            launch_and_watch(&scrcpy, &args)?
        };
        if started {
            let fallback_used = index > 0;
            let remembered = remember_successful_profile(variant).is_ok();
            return Ok(LaunchResult {
                started: true,
                fallback_used,
                attempts: index + 1,
                command_preview: shell_preview(&scrcpy, &args),
                recording_path: recording.as_ref().map(|p| p.display().to_string()),
                desktop_diagnostics: last_desktop_diagnostics,
                message: if fallback_used {
                    if config.mode == "camera" {
                        format!(
                            "Camera opened after SCRCPY Studio automatically found a safer working combination on attempt {} of {}.",
                            index + 1,
                            total
                        )
                    } else if config.mode == "desktop" {
                        format!(
                            "{} recovered automatically on attempt {} of {} using a safer capture configuration.",
                            desktop_launch_name(&config), index + 1, total
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
                } else if config.mode == "desktop" {
                    format!(
                        "{} opened. The Desktop Diagnostics log records the exact command and observed Android display state.",
                        desktop_launch_name(&config)
                    )
                } else {
                    "Session started with the selected smart profile.".into()
                },
            });
        }
    }

    if config.mode == "desktop" {
        let diagnostics = last_desktop_diagnostics.unwrap_or_default();
        let detail = diagnostics
            .scrcpy_output
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or(&diagnostics.exit_result)
            .trim();
        return Ok(LaunchResult {
            started: false,
            fallback_used: total > 1,
            attempts: total,
            command_preview: diagnostics.command.clone(),
            recording_path: recording.as_ref().map(|p| p.display().to_string()),
            message: format!(
                "{} did not stay running after {} attempts: {}. Open Desktop Diagnostics for the complete command, output, and device evidence.",
                desktop_launch_name(&config), total, detail
            ),
            desktop_diagnostics: Some(diagnostics),
        });
    }

    Err(format!(
        "scrcpy exited immediately after {} smart attempts. Open Connection Doctor and verify the device/runtime.",
        total
    ))
}

fn desktop_launch_name(config: &LaunchConfig) -> &'static str {
    match config.desktop_environment.as_deref() {
        Some("samsung_dex") => "Samsung DeX display",
        Some("android_desktop_windowing") => "Android Desktop Windowing",
        _ => "Virtual Display",
    }
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
            desktop_width: None,
            desktop_height: None,
            desktop_density: None,
            desktop_flex: false,
            desktop_no_decorations: false,
            desktop_keep_content: false,
            desktop_start_app: None,
            desktop_environment: None,
            desktop_display_id: None,
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
    fn rejects_a_stale_config_from_another_mode() {
        let config = sample_config("mirror");
        let error = validate_launch_mode(&config, "camera").unwrap_err();
        assert!(error.contains("changed while its settings were loading"));
    }

    #[test]
    fn accepts_a_config_for_the_requested_mode() {
        let config = sample_config("camera");
        assert!(validate_launch_mode(&config, "camera").is_ok());
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

    #[test]
    fn desktop_args_use_verified_virtual_display_controls() {
        let mut config = sample_config("desktop");
        config.desktop_width = Some(1920);
        config.desktop_height = Some(1080);
        config.desktop_density = Some(240);
        config.desktop_flex = true;
        config.desktop_keep_content = true;
        config.desktop_start_app = Some("com.android.settings".into());
        let args = build_args(&config, None);
        assert!(args.contains(&"--new-display=1920x1080/240".to_string()));
        assert!(args.contains(&"--flex-display".to_string()));
        assert!(args.contains(&"--no-vd-destroy-content".to_string()));
        assert!(args.contains(&"--start-app=com.android.settings".to_string()));
        assert!(args.contains(&"--display-ime-policy=local".to_string()));
        assert!(args.contains(&"--keep-active".to_string()));
    }

    #[test]
    fn desktop_fallbacks_reduce_risky_options() {
        let mut config = sample_config("desktop");
        config.desktop_width = Some(1920);
        config.desktop_height = Some(1080);
        config.desktop_density = Some(240);
        config.desktop_flex = true;
        config.desktop_start_app = Some("com.example.launcher".into());
        let variants = fallback_configs(&config);
        assert!(variants.iter().any(|item| !item.desktop_flex));
        assert!(variants
            .iter()
            .any(|item| item.desktop_start_app.as_deref() == Some("com.android.settings")));
        assert!(variants.iter().any(|item| item.desktop_width == Some(1280)));
        assert!(variants.iter().any(|item| item.max_fps == 30));
    }

    #[test]
    fn samsung_dex_captures_an_existing_display() {
        let mut config = sample_config("desktop");
        config.desktop_environment = Some("samsung_dex".into());
        config.desktop_display_id = Some(2);
        let args = build_args(&config, None);
        assert!(args.contains(&"--display-id=2".to_string()));
        assert!(!args.iter().any(|arg| arg.starts_with("--new-display=")));
    }
}
