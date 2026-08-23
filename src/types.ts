export type SessionMode = "mirror" | "creator" | "camera" | "desktop";
export type DesktopEnvironment = "unavailable" | "virtual_display" | "android_freeform_windowing" | "android_desktop_windowing" | "samsung_dex";

export interface RuntimeStatus {
  adbFound: boolean;
  scrcpyFound: boolean;
  adbPath?: string | null;
  scrcpyPath?: string | null;
  adbVersion?: string | null;
  scrcpyVersion?: string | null;
}

export interface DeviceInfo {
  serial: string;
  state: string;
  model?: string | null;
  product?: string | null;
  device?: string | null;
  connectionKind: "usb" | "wireless";
}

export interface DeviceProfile {
  serial: string;
  model: string;
  brand: string;
  androidVersion: string;
  sdk: number;
  width?: number | null;
  height?: number | null;
  density?: number | null;
  connectionKind: "usb" | "wireless";
  supportsAudio: boolean;
  supportsCamera: boolean;
  canAttemptVirtualDisplay: boolean;
  h265Available: boolean;
}

export interface Recommendation {
  mode: SessionMode;
  maxSize: number;
  maxFps: number;
  codec: "h264" | "h265";
  audio: boolean;
  stayAwake: boolean;
  turnScreenOff: boolean;
  showTouches: boolean;
  qualityLabel: string;
  rationale: string[];
}

export interface LaunchConfig {
  serial: string;
  mode: SessionMode;
  maxSize: number;
  maxFps: number;
  codec: "h264" | "h265";
  audio: boolean;
  stayAwake: boolean;
  turnScreenOff: boolean;
  showTouches: boolean;
  record: boolean;
  fullscreen: boolean;
  cameraId?: string | null;
  cameraFacing?: "front" | "back" | "external" | null;
  cameraZoom?: number | null;
  cameraTorch?: boolean;
  desktopWidth?: number | null;
  desktopHeight?: number | null;
  desktopDensity?: number | null;
  desktopFlex?: boolean;
  desktopNoDecorations?: boolean;
  desktopKeepContent?: boolean;
  desktopStartApp?: string | null;
  desktopEnvironment?: DesktopEnvironment | null;
  desktopDisplayId?: number | null;
}

export interface LaunchResult {
  started: boolean;
  fallbackUsed: boolean;
  attempts: number;
  commandPreview: string;
  recordingPath?: string | null;
  message: string;
  desktopDiagnostics?: DesktopDiagnostics | null;
}

export interface DoctorFinding {
  level: "ok" | "info" | "warning" | "error";
  title: string;
  detail: string;
  action?: string | null;
}

export interface RememberedWirelessDevice {
  address: string;
  label: string;
  connected: boolean;
  lastUsed: number;
}

export interface TransportSwitchResult {
  activeSerial: string;
  activeConnection: "usb" | "wireless" | "none";
  message: string;
  safeToUnplugUsb: boolean;
}

export interface CameraInfo {
  id: string;
  facing: "front" | "back" | "external" | string;
  maxWidth?: number | null;
  maxHeight?: number | null;
  fps: number[];
  zoomMin?: number | null;
  zoomMax?: number | null;
  sizes: string[];
  torchLikely: boolean;
}

export interface CameraCapabilities {
  cameraSupported: boolean;
  recommendedCameraId?: string | null;
  cameras: CameraInfo[];
  note: string;
}

export interface DesktopCapabilities {
  supported: boolean;
  environmentKind: DesktopEnvironment;
  environmentLabel: string;
  launchLabel: string;
  virtualDisplaySupported: boolean;
  androidDesktopWindowingAvailable: boolean;
  androidDesktopWindowingActive: boolean;
  samsungDexAvailable: boolean;
  samsungDexActive: boolean;
  existingDisplayId?: number | null;
  recommendedWidth: number;
  recommendedHeight: number;
  recommendedDensity: number;
  flexSupported: boolean;
  systemDecorationsSupported: boolean;
  keepContentSupported: boolean;
  launcherPackage?: string | null;
  startupPackage: string;
  desktopExperiencePrepared: boolean;
  desktopExperienceCanPrepare: boolean;
  desktopExperienceBackupAvailable: boolean;
  desktopExperienceSummary: string;
  message: string;
  diagnostics: DesktopDiagnostics;
}

export interface DesktopProbeState {
  serial: string;
  checking: boolean;
  capabilities: DesktopCapabilities | null;
  error?: string | null;
}

export interface DesktopSettingDiagnostic {
  key: string;
  value?: string | null;
}

export interface DesktopDiagnostics {
  command: string;
  exitResult: string;
  displayId?: number | null;
  displayName?: string | null;
  resolution?: string | null;
  density?: number | null;
  launcherActivity?: string | null;
  windowingMode: string;
  relevantSettings: DesktopSettingDiagnostic[];
  platformEvidence: string[];
  scrcpyOutput: string;
  logPath?: string | null;
}

export interface DesktopExperienceResult {
  prepared: boolean;
  backupAvailable: boolean;
  rebootStarted: boolean;
  message: string;
}

