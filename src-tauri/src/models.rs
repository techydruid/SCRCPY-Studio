use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeStatus {
    pub(crate) adb_found: bool,
    pub(crate) scrcpy_found: bool,
    pub(crate) adb_path: Option<String>,
    pub(crate) scrcpy_path: Option<String>,
    pub(crate) adb_version: Option<String>,
    pub(crate) scrcpy_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceInfo {
    pub(crate) serial: String,
    pub(crate) state: String,
    pub(crate) model: Option<String>,
    pub(crate) product: Option<String>,
    pub(crate) device: Option<String>,
    pub(crate) connection_kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceProfile {
    pub(crate) serial: String,
    pub(crate) model: String,
    pub(crate) brand: String,
    pub(crate) android_version: String,
    pub(crate) sdk: u32,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) density: Option<u32>,
    pub(crate) connection_kind: String,
    pub(crate) supports_audio: bool,
    pub(crate) supports_camera: bool,
    pub(crate) can_attempt_virtual_display: bool,
    pub(crate) h265_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Recommendation {
    pub(crate) mode: String,
    pub(crate) max_size: u32,
    pub(crate) max_fps: u32,
    pub(crate) codec: String,
    pub(crate) audio: bool,
    pub(crate) stay_awake: bool,
    pub(crate) turn_screen_off: bool,
    pub(crate) show_touches: bool,
    pub(crate) quality_label: String,
    pub(crate) rationale: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchConfig {
    pub(crate) serial: String,
    pub(crate) mode: String,
    pub(crate) max_size: u32,
    pub(crate) max_fps: u32,
    pub(crate) codec: String,
    pub(crate) audio: bool,
    pub(crate) stay_awake: bool,
    pub(crate) turn_screen_off: bool,
    pub(crate) show_touches: bool,
    pub(crate) record: bool,
    pub(crate) fullscreen: bool,
    #[serde(default)]
    pub(crate) camera_id: Option<String>,
    #[serde(default)]
    pub(crate) camera_facing: Option<String>,
    #[serde(default)]
    pub(crate) camera_zoom: Option<f64>,
    #[serde(default)]
    pub(crate) camera_torch: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchResult {
    pub(crate) started: bool,
    pub(crate) fallback_used: bool,
    pub(crate) attempts: usize,
    pub(crate) command_preview: String,
    pub(crate) recording_path: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorFinding {
    pub(crate) level: String,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) action: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RememberedWirelessDevice {
    pub(crate) address: String,
    pub(crate) label: String,
    pub(crate) connected: bool,
    pub(crate) last_used: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransportSwitchResult {
    pub(crate) active_serial: String,
    pub(crate) active_connection: String,
    pub(crate) message: String,
    pub(crate) safe_to_unplug_usb: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CameraInfo {
    pub(crate) id: String,
    pub(crate) facing: String,
    pub(crate) max_width: Option<u32>,
    pub(crate) max_height: Option<u32>,
    pub(crate) fps: Vec<u32>,
    pub(crate) zoom_min: Option<f64>,
    pub(crate) zoom_max: Option<f64>,
    pub(crate) sizes: Vec<String>,
    pub(crate) torch_likely: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CameraCapabilities {
    pub(crate) camera_supported: bool,
    pub(crate) recommended_camera_id: Option<String>,
    pub(crate) cameras: Vec<CameraInfo>,
    pub(crate) note: String,
}
