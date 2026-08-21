export type SessionMode = "mirror" | "creator" | "camera" | "desktop";

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
}

export interface LaunchResult {
  started: boolean;
  fallbackUsed: boolean;
  attempts: number;
  commandPreview: string;
  recordingPath?: string | null;
  message: string;
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
