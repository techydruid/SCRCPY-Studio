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
    pub(crate) av1_available: bool,
    pub(crate) video_encoders: Vec<VideoEncoderInfo>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VideoEncoderInfo {
    pub(crate) codec: String,
    pub(crate) name: String,
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
    #[serde(default = "default_video_bit_rate")]
    pub(crate) video_bit_rate: u32,
    #[serde(default)]
    pub(crate) video_encoder: Option<String>,
    pub(crate) audio: bool,
    #[serde(default)]
    pub(crate) audio_source: Option<String>,
    pub(crate) stay_awake: bool,
    pub(crate) turn_screen_off: bool,
    pub(crate) show_touches: bool,
    pub(crate) record: bool,
    pub(crate) fullscreen: bool,
    #[serde(default)]
    pub(crate) capture_orientation: Option<String>,
    #[serde(default)]
    pub(crate) crop: Option<String>,
    #[serde(default)]
    pub(crate) camera_id: Option<String>,
    #[serde(default)]
    pub(crate) camera_facing: Option<String>,
    #[serde(default)]
    pub(crate) camera_zoom: Option<f64>,
    #[serde(default)]
    pub(crate) camera_torch: bool,
    #[serde(default)]
    pub(crate) camera_size: Option<String>,
    #[serde(default)]
    pub(crate) camera_aspect_ratio: Option<String>,
    #[serde(default)]
    pub(crate) camera_high_speed: bool,
    #[serde(default)]
    pub(crate) desktop_width: Option<u32>,
    #[serde(default)]
    pub(crate) desktop_height: Option<u32>,
    #[serde(default)]
    pub(crate) desktop_density: Option<u32>,
    #[serde(default)]
    pub(crate) desktop_flex: bool,
    #[serde(default)]
    pub(crate) desktop_no_decorations: bool,
    #[serde(default)]
    pub(crate) desktop_keep_content: bool,
    #[serde(default)]
    pub(crate) desktop_start_app: Option<String>,
    #[serde(default)]
    pub(crate) desktop_environment: Option<String>,
    #[serde(default)]
    pub(crate) desktop_display_id: Option<u32>,
}

fn default_video_bit_rate() -> u32 {
    8
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
    pub(crate) desktop_diagnostics: Option<DesktopDiagnostics>,
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
    pub(crate) high_speed_modes: Vec<CameraHighSpeedMode>,
    pub(crate) torch_likely: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CameraHighSpeedMode {
    pub(crate) size: String,
    pub(crate) fps: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CameraCapabilities {
    pub(crate) camera_supported: bool,
    pub(crate) recommended_camera_id: Option<String>,
    pub(crate) cameras: Vec<CameraInfo>,
    pub(crate) note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopCapabilities {
    pub(crate) supported: bool,
    pub(crate) environment_kind: String,
    pub(crate) environment_label: String,
    pub(crate) launch_label: String,
    pub(crate) virtual_display_supported: bool,
    pub(crate) android_desktop_windowing_available: bool,
    pub(crate) android_desktop_windowing_active: bool,
    pub(crate) samsung_dex_available: bool,
    pub(crate) samsung_dex_active: bool,
    pub(crate) existing_display_id: Option<u32>,
    pub(crate) recommended_width: u32,
    pub(crate) recommended_height: u32,
    pub(crate) recommended_density: u32,
    pub(crate) flex_supported: bool,
    pub(crate) system_decorations_supported: bool,
    pub(crate) keep_content_supported: bool,
    pub(crate) launcher_package: Option<String>,
    pub(crate) startup_package: String,
    pub(crate) desktop_experience_prepared: bool,
    pub(crate) desktop_experience_can_prepare: bool,
    pub(crate) desktop_experience_backup_available: bool,
    pub(crate) desktop_experience_summary: String,
    pub(crate) message: String,
    pub(crate) diagnostics: DesktopDiagnostics,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopSettingDiagnostic {
    pub(crate) key: String,
    pub(crate) value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopDiagnostics {
    pub(crate) command: String,
    pub(crate) exit_result: String,
    pub(crate) display_id: Option<u32>,
    pub(crate) display_name: Option<String>,
    pub(crate) resolution: Option<String>,
    pub(crate) density: Option<u32>,
    pub(crate) launcher_activity: Option<String>,
    pub(crate) windowing_mode: String,
    pub(crate) relevant_settings: Vec<DesktopSettingDiagnostic>,
    pub(crate) platform_evidence: Vec<String>,
    pub(crate) scrcpy_output: String,
    pub(crate) log_path: Option<String>,
}

impl Default for DesktopDiagnostics {
    fn default() -> Self {
        Self {
            command: String::new(),
            exit_result: "not run".into(),
            display_id: None,
            display_name: None,
            resolution: None,
            density: None,
            launcher_activity: None,
            windowing_mode: "unknown".into(),
            relevant_settings: Vec::new(),
            platform_evidence: Vec::new(),
            scrcpy_output: String::new(),
            log_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopExperienceResult {
    pub(crate) prepared: bool,
    pub(crate) backup_available: bool,
    pub(crate) reboot_started: bool,
    pub(crate) message: String,
}
