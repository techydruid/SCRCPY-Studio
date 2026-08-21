import { invoke } from "@tauri-apps/api/core";
import { Camera, CheckCircle2, Flashlight, RefreshCw, ZoomIn } from "lucide-react";
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
  const values = new Set<number>([30]);
  camera?.fps.filter((fps) => fps > 0 && fps <= 60).forEach((fps) => values.add(fps));
  return [...values].sort((a, b) => a - b);
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
          const fps = usefulFps(recommended);
          onChange({
            ...config,
            cameraId: recommended.id,
            cameraFacing: facingValue(recommended.facing),
            cameraZoom: 1,
            cameraTorch: false,
            maxSize: config.maxSize || 1920,
            maxFps: fps.includes(config.maxFps) ? config.maxFps : (fps.includes(30) ? 30 : fps[0] || 30)
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
  const fpsValues = usefulFps(selectedCamera);
  const zoomValues = zoomOptions(selectedCamera);

  const chooseCamera = (id: string) => {
    const camera = capabilities?.cameras.find((item) => item.id === id);
    if (!camera) return;
    const fps = usefulFps(camera);
    onChange({
      ...config,
      cameraId: camera.id,
      cameraFacing: facingValue(camera.facing),
      cameraZoom: 1,
      cameraTorch: false,
      maxFps: fps.includes(config.maxFps) ? config.maxFps : (fps.includes(30) ? 30 : fps[0] || 30)
    });
  };

  return (
    <div className="creator-tools">
      <div className="creator-tools-heading">
        <div><span className="eyebrow">SMART CAMERA</span><strong>Useful controls only</strong></div>
        <span>{loading ? "Detecting lenses…" : selectedCamera ? `${capabilities?.cameras.length ?? 0} detected` : "Auto camera"}</span>
      </div>

      {loading ? (
        <div className="smart-note"><RefreshCw size={17} className="spin" /><span>Reading the cameras, declared frame rates and zoom ranges from the phone.</span></div>
      ) : error ? (
        <div className="smart-note"><Camera size={17} /><span>Lens details could not be read, so SCRCPY Studio will fall back to scrcpy's automatic camera selection. You can still open Camera Mode.</span></div>
      ) : capabilities?.cameras.length ? (
        <>
          <div className="form-grid camera-form">
            <label className="field">
              <span>Camera / lens</span>
              <select value={selectedCamera?.id ?? ""} onChange={(event) => chooseCamera(event.target.value)}>
                {capabilities.cameras.map((camera) => <option key={camera.id} value={camera.id}>{cameraLabel(camera, capabilities.cameras)}</option>)}
              </select>
            </label>
            <label className="field">
              <span>Quality</span>
              <select value={config.maxSize} onChange={(event) => onChange({ ...config, maxSize: Number(event.target.value) })}>
                <option value={1280}>720-class · stable</option>
                <option value={1920}>1080-class · recommended</option>
                <option value={0}>Maximum available</option>
              </select>
            </label>
            <label className="field">
              <span>Camera FPS</span>
              <select value={config.maxFps} onChange={(event) => onChange({ ...config, maxFps: Number(event.target.value) })}>
                <option value={0}>Auto</option>
                {fpsValues.map((fps) => <option key={fps} value={fps}>{fps} FPS</option>)}
              </select>
            </label>
          </div>

          <div className="form-grid camera-form secondary-camera-row">
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
            <div className="camera-summary">
              <CheckCircle2 size={15} />
              <span>{selectedCamera?.fps.length ? `Reported FPS: ${selectedCamera.fps.join(", ")}` : "Frame rates not reported"}{selectedCamera?.zoomMax ? ` · Zoom up to ${selectedCamera.zoomMax}×` : ""}</span>
            </div>
          </div>

          <div className="smart-note"><Camera size={17} /><span>{capabilities.note} Torch and zoom are automatically removed first if they prevent a camera from opening.</span></div>
        </>
      ) : (
        <div className="smart-note"><Camera size={17} /><span>No individual camera IDs were reported. Camera Mode will use scrcpy's automatic camera selection and conservative 1080-class settings.</span></div>
      )}
    </div>
  );
}
