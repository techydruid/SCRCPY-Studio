import { invoke } from "@tauri-apps/api/core";
import { Camera, Flashlight, RefreshCw, ZoomIn } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import "./camera.css";
import type { CameraCapabilities, CameraInfo, LaunchConfig } from "./types";

type Props = {
  serial: string;
  config: LaunchConfig;
  onChange: (next: LaunchConfig) => void;
  onStatus: (message: string) => void;
};

function facingValue(value: string): "front" | "back" | "external" | null {
  return value === "front" || value === "back" || value === "external" ? value : null;
}

function cameraLabel(camera: CameraInfo, cameras: CameraInfo[]) {
  const sameFacing = cameras.filter((item) => item.facing === camera.facing);
  const index = sameFacing.findIndex((item) => item.id === camera.id) + 1;
  const facing = camera.facing === "front" ? "Front" : camera.facing === "back" ? "Back" : camera.facing === "external" ? "External" : "Camera";
  const suffix = sameFacing.length > 1 ? ` ${index}` : "";
  const max = camera.maxWidth && camera.maxHeight ? ` · up to ${camera.maxWidth}×${camera.maxHeight}` : "";
  return `${facing}${suffix} · ID ${camera.id}${max}`;
}

function usefulFps(camera?: CameraInfo | null) {
  const reported = camera?.fps.filter((fps) => fps > 0 && fps <= 60) ?? [];
  const values = reported.length ? reported : [30];
  return [...new Set(values)].sort((a, b) => a - b);
}

function preferredFps(current: number, camera?: CameraInfo | null) {
  const values = usefulFps(camera);
  if (current === 0 || values.includes(current)) return current;
  if (values.includes(30)) return 30;
  const atOrBelow30 = values.filter((fps) => fps <= 30);
  return atOrBelow30.length ? atOrBelow30[atOrBelow30.length - 1] : values[0] ?? 0;
}

function initialZoom(camera?: CameraInfo | null) {
  if (camera?.zoomMin == null || camera.zoomMax == null || camera.zoomMax <= 1) return null;
  return Math.max(1, camera.zoomMin);
}

function zoomOptions(camera?: CameraInfo | null) {
  const min = Math.max(1, camera?.zoomMin ?? 1);
  const max = camera?.zoomMax ?? 1;
  if (max <= min + 0.01) return [1];
  const candidates = [1, 1.5, 2, 3, 4, 5, 6, 8, 10];
  const values = candidates.filter((value) => value >= min - 0.01 && value <= max + 0.01);
  if (!values.some((value) => Math.abs(value - min) < 0.01)) values.unshift(Number(min.toFixed(1)));
  return [...new Set(values)].sort((a, b) => a - b);
}

export default function CameraControls({ serial, config, onChange, onStatus }: Props) {
  const [capabilities, setCapabilities] = useState<CameraCapabilities | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setCapabilities(null);

    void invoke<CameraCapabilities>("list_camera_capabilities", { serial })
      .then((result) => {
        if (cancelled) return;
        setCapabilities(result);
        const recommended = result.cameras.find((camera) => camera.id === result.recommendedCameraId) ?? result.cameras[0];
        if (recommended && !config.cameraId) {
          onChange({
            ...config,
            cameraId: recommended.id,
            cameraFacing: facingValue(recommended.facing),
            cameraZoom: initialZoom(recommended),
            cameraTorch: false,
            maxSize: config.maxSize || 1920,
            maxFps: preferredFps(config.maxFps, recommended)
          });
        }
        onStatus(result.cameras.length ? `${result.cameras.length} camera${result.cameras.length === 1 ? "" : "s"} detected. Camera Mode is ready.` : "Camera details were not reported. SCRCPY Studio will use automatic camera selection.");
      })
      .catch((reason) => {
        if (cancelled) return;
        const message = String(reason);
        setError(message);
        onStatus(`Camera probe could not read lens details: ${message}`);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
    // The probe should rerun only when the active ADB transport changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serial]);

  const selectedCamera = useMemo(
    () => capabilities?.cameras.find((camera) => camera.id === config.cameraId) ?? capabilities?.cameras[0] ?? null,
    [capabilities, config.cameraId]
  );
  const zoomValues = zoomOptions(selectedCamera);

  const chooseCamera = (id: string) => {
    const camera = capabilities?.cameras.find((item) => item.id === id);
    if (!camera) return;
    onChange({
      ...config,
      cameraId: camera.id,
      cameraFacing: facingValue(camera.facing),
      cameraZoom: initialZoom(camera),
      cameraTorch: false,
      maxFps: preferredFps(config.maxFps, camera)
    });
  };

  return (
    <div className="camera-controls">
      <div className="compact-section-heading">
        <strong>Camera</strong>
        <span>{loading ? "Detecting…" : selectedCamera ? `${capabilities?.cameras.length ?? 0} lenses` : "Automatic"}</span>
      </div>

      {loading ? (
        <div className="inline-state"><RefreshCw size={16} className="spin" /> Detecting lenses</div>
      ) : error ? (
        <div className="inline-state"><Camera size={16} /> Automatic camera selection</div>
      ) : capabilities?.cameras.length ? (
          <div className="camera-grid">
            <label className="field camera-lens-field">
              <span>Camera / lens</span>
              <select value={selectedCamera?.id ?? ""} onChange={(event) => chooseCamera(event.target.value)}>
                {capabilities.cameras.map((camera) => <option key={camera.id} value={camera.id}>{cameraLabel(camera, capabilities.cameras)}</option>)}
              </select>
            </label>
            <label className="field">
              <span><ZoomIn size={13} /> Initial zoom</span>
              <select value={config.cameraZoom ?? 1} onChange={(event) => onChange({ ...config, cameraZoom: Number(event.target.value) })} disabled={zoomValues.length <= 1}>
                {zoomValues.map((zoom) => <option key={zoom} value={zoom}>{zoom}×</option>)}
              </select>
            </label>
            <label className="toggle-row camera-toggle">
              <span><Flashlight size={14} /> Torch at startup</span>
              <input type="checkbox" checked={Boolean(config.cameraTorch)} onChange={(event) => onChange({ ...config, cameraTorch: event.target.checked })} disabled={!selectedCamera?.torchLikely} />
              <i />
            </label>
          </div>
      ) : (
        <div className="inline-state"><Camera size={16} /> Automatic camera selection</div>
      )}
    </div>
  );
}

