use crate::{
    devices::list_devices,
    models::{CameraCapabilities, CameraInfo},
    runtime::scrcpy_path,
};
use std::process::Command;

fn command_output(mut command: Command) -> Result<String, String> {
    let output = command.output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = format!("{}\n{}", stdout.trim(), stderr.trim()).trim().to_string();
    if output.status.success() {
        Ok(text)
    } else if text.is_empty() {
        Err(format!("Camera probe failed with {}", output.status))
    } else {
        Err(text)
    }
}

fn dimensions(value: &str) -> Option<(u32, u32)> {
    let cleaned = value.trim_matches(|c: char| !c.is_ascii_digit() && c != 'x');
    let (w, h) = cleaned.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

fn first_dimensions(text: &str) -> Option<(u32, u32)> {
    text.split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')' | '[' | ']'))
        .find_map(dimensions)
}

fn list_values(text: &str, marker: &str) -> Vec<String> {
    let Some(start) = text.find(marker) else {
        return Vec::new();
    };
    let after = &text[start + marker.len()..];
    let mut end = after.len();
    for (index, ch) in after.char_indices() {
        if matches!(ch, ']' | '}') {
            end = index;
            break;
        }
    }
    after[..end]
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn parse_fps(text: &str) -> Vec<u32> {
    let mut values = list_values(text, "fps=[");
    if values.is_empty() {
        values = list_values(text, "fps={");
    }
    let mut fps = values
        .into_iter()
        .filter_map(|value| value.parse::<u32>().ok())
        .collect::<Vec<_>>();
    fps.sort_unstable();
    fps.dedup();
    fps
}

fn parse_zoom(text: &str) -> (Option<f64>, Option<f64>) {
    let values = list_values(text, "zoom-range=[");
    if values.len() < 2 {
        return (None, None);
    }
    (values[0].parse().ok(), values[1].parse().ok())
}

fn parse_camera_header(line: &str) -> Option<CameraInfo> {
    let marker = "--camera-id=";
    let start = line.find(marker)? + marker.len();
    let id = line[start..]
        .split_whitespace()
        .next()?
        .trim_matches(|c: char| c == ',' || c == ')')
        .to_string();
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    let details = &line[open + 1..close];
    let facing = details
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_ascii_lowercase();
    let (max_width, max_height) = first_dimensions(details)
        .map(|(w, h)| (Some(w), Some(h)))
        .unwrap_or((None, None));
    let (zoom_min, zoom_max) = parse_zoom(details);

    Some(CameraInfo {
        id,
        facing: facing.clone(),
        max_width,
        max_height,
        fps: parse_fps(details),
        zoom_min,
        zoom_max,
        sizes: Vec::new(),
        torch_likely: facing == "back",
    })
}

pub(crate) fn parse_camera_output(raw: &str) -> Vec<CameraInfo> {
    let mut cameras: Vec<CameraInfo> = Vec::new();
    let mut current_id: Option<String> = None;

    for line in raw.lines() {
        if let Some(parsed) = parse_camera_header(line) {
            current_id = Some(parsed.id.clone());
            if let Some(existing) = cameras.iter_mut().find(|camera| camera.id == parsed.id) {
                if existing.facing == "unknown" {
                    existing.facing = parsed.facing;
                }
                if existing.max_width.is_none() {
                    existing.max_width = parsed.max_width;
                    existing.max_height = parsed.max_height;
                }
                if existing.fps.is_empty() {
                    existing.fps = parsed.fps;
                }
                if existing.zoom_min.is_none() {
                    existing.zoom_min = parsed.zoom_min;
                    existing.zoom_max = parsed.zoom_max;
                }
                existing.torch_likely |= parsed.torch_likely;
            } else {
                cameras.push(parsed);
            }
            continue;
        }

        let trimmed = line.trim();
        if trimmed.starts_with("- ") && !trimmed.starts_with("--") {
            let Some(id) = current_id.as_deref() else {
                continue;
            };
            let Some((w, h)) = first_dimensions(trimmed) else {
                continue;
            };
            let value = format!("{w}x{h}");
            if let Some(camera) = cameras.iter_mut().find(|camera| camera.id == id) {
                if !camera.sizes.contains(&value) {
                    camera.sizes.push(value);
                }
            }
        }
    }

    for camera in &mut cameras {
        camera.fps.sort_unstable();
        camera.fps.dedup();
        camera.sizes.sort_by(|a, b| {
            let area = |value: &String| {
                dimensions(value)
                    .map(|(w, h)| u64::from(w) * u64::from(h))
                    .unwrap_or(0)
            };
            area(b).cmp(&area(a))
        });
    }

    cameras.sort_by(|a, b| {
        let a_num = a.id.parse::<u32>();
        let b_num = b.id.parse::<u32>();
        match (a_num, b_num) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            _ => a.id.cmp(&b.id),
        }
    });
    cameras
}

#[tauri::command]
pub(crate) fn list_camera_capabilities(serial: String) -> Result<CameraCapabilities, String> {
    let devices = list_devices()?;
    let device = devices
        .iter()
        .find(|device| device.serial == serial)
        .ok_or_else(|| "Selected device is no longer connected.".to_string())?;
    if device.state != "device" {
        return Err("Authorize the device before probing its cameras.".into());
    }

    let scrcpy = scrcpy_path()?;
    let mut command = Command::new(scrcpy);
    command.args(["-s", &serial, "--list-cameras", "--list-camera-sizes"]);
    let raw = command_output(command)?;
    let cameras = parse_camera_output(&raw);
    let recommended_camera_id = cameras
        .iter()
        .find(|camera| camera.facing == "back")
        .or_else(|| cameras.first())
        .map(|camera| camera.id.clone());

    Ok(CameraCapabilities {
        camera_supported: !cameras.is_empty(),
        recommended_camera_id,
        cameras,
        note: "Camera capabilities are reported by Android. SCRCPY Studio uses conservative defaults and automatically retries safer settings if a declared combination fails."
            .into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_camera_list() {
        let raw = r#"
[server] INFO: List of cameras:
    --camera-id=0    (back, 4624x3472, fps={10, 15, 20, 30}, zoom-range=[1, 10])
        - 3840x2160
        - 1920x1080
    --camera-id=1    (front, 4208x3120, fps=[15, 30, 60], zoom-range=[1, 4])
        - 1920x1080
        - 1280x720
"#;
        let cameras = parse_camera_output(raw);
        assert_eq!(cameras.len(), 2);
        assert_eq!(cameras[0].id, "0");
        assert_eq!(cameras[0].facing, "back");
        assert_eq!(cameras[0].fps, vec![10, 15, 20, 30]);
        assert_eq!(cameras[0].zoom_max, Some(10.0));
        assert!(cameras[0].sizes.contains(&"1920x1080".to_string()));
        assert_eq!(cameras[1].facing, "front");
        assert_eq!(cameras[1].fps, vec![15, 30, 60]);
    }

    #[test]
    fn merges_repeated_camera_headers() {
        let raw = "--camera-id=0 (back, 4000x3000, fps=[15, 30])\n--camera-id=0 (back, 4000x3000, fps=[15, 30])\n    - 1920x1080\n";
        let cameras = parse_camera_output(raw);
        assert_eq!(cameras.len(), 1);
        assert_eq!(cameras[0].sizes, vec!["1920x1080"]);
    }
}
