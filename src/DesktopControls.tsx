import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, CircleAlert, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { DesktopCapabilities, LaunchConfig } from "./types";

type Props = {
  serial: string;
  config: LaunchConfig;
  onChange: (next: LaunchConfig) => void;
  onStatus: (message: string) => void;
};

function layoutValue(width?: number | null, height?: number | null) {
  return `${width ?? 1920}x${height ?? 1080}`;
}

function densityForLayout(height: number, recommended: number) {
  if (height <= 720) return 180;
  if (height <= 900) return 240;
  return recommended;
}

function densityLabel(value: number) {
  if (value === 180) return "Wide desktop · 180 dpi";
  if (value === 200) return "Compact · 200 dpi";
  if (value === 240) return "Balanced · 240 dpi";
  if (value === 284) return "Samsung desktop · 284 dpi";
  return `${value} dpi`;
}

export default function DesktopControls({ serial, config, onChange, onStatus }: Props) {
  const [capabilities, setCapabilities] = useState<DesktopCapabilities | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setCapabilities(null);
    onChange({ ...config, desktopSupported: false });
    onStatus("Checking whether this phone can create a secondary Android display…");

    void invoke<DesktopCapabilities>("probe_desktop_capabilities", { serial })
      .then((result) => {
        if (cancelled) return;
        setCapabilities(result);
        onChange({
          ...config,
          desktopSupported: result.supported,
          desktopWidth: result.recommendedWidth,
          desktopHeight: result.recommendedHeight,
          desktopDensity: result.recommendedDensity,
          // Flex Display is useful, but must stay opt-in for Desktop Mode. If a
          // user shrinks the window below desktop-class dimensions, Android or
          // an OEM launcher may legitimately switch back to a phone UI.
          desktopFlex: false,
          desktopNoDecorations: false,
          desktopKeepContent: false,
          // Do not force the phone's HOME package. Samsung/Android must be free
          // to choose its secondary-display desktop/DeX environment.
          desktopStartApp: null,
          stayAwake: true
        });
        onStatus(result.message);
      })
      .catch((reason) => {
        if (cancelled) return;
        const message = String(reason);
        setError(message);
        onChange({ ...config, desktopSupported: false });
        onStatus(`Desktop Mode check failed: ${message}`);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
    // Probe only when the active ADB transport changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serial]);

  const setLayout = (value: string) => {
    const [width, height] = value.split("x").map(Number);
    if (!width || !height || !capabilities) return;
    onChange({
      ...config,
      desktopWidth: width,
      desktopHeight: height,
      desktopDensity: densityForLayout(height, capabilities.recommendedDensity)
    });
  };

  const desktopHeight = config.desktopHeight ?? capabilities?.recommendedHeight ?? 1080;
  // 600dp is Android's important large/desktop-screen boundary. Hide density
  // choices that would make the selected virtual display smaller than that.
  const maxDesktopDensity = Math.floor((desktopHeight * 160) / 600);
  const densityOptions = [180, 200, 240, 284].filter((value) => value <= maxDesktopDensity);
  const currentDensity = config.desktopDensity ?? capabilities?.recommendedDensity ?? 240;
  if (!densityOptions.includes(currentDensity) && currentDensity <= maxDesktopDensity) {
    densityOptions.push(currentDensity);
    densityOptions.sort((a, b) => a - b);
  }

  return (
    <div className="creator-tools">
      <div className="creator-tools-heading">
        <div><span className="eyebrow">SMART DESKTOP</span><strong>Verified secondary display</strong></div>
        <span>{loading ? "Checking support…" : capabilities?.supported ? "Ready" : "Unavailable"}</span>
      </div>

      {loading ? (
        <div className="smart-note"><RefreshCw size={17} className="spin" /><span>SCRCPY Studio is creating a one-second hidden test display, then removing it. This verifies real virtual-display support instead of guessing from the Android version.</span></div>
      ) : error ? (
        <div className="finding error"><CircleAlert size={18} /><div><strong>Desktop Mode could not be verified</strong><span>{error}</span></div></div>
      ) : !capabilities?.supported ? (
        <div className="finding warning"><CircleAlert size={18} /><div><strong>Virtual display unavailable</strong><span>{capabilities?.message || "This phone/runtime combination did not pass the virtual-display check."}</span></div></div>
      ) : (
        <>
          <div className="form-grid">
            <label className="field">
              <span>Desktop size</span>
              <select value={layoutValue(config.desktopWidth, config.desktopHeight)} onChange={(event) => setLayout(event.target.value)}>
                <option value="1920x1080">1920×1080 · Full HD</option>
                <option value="1600x900">1600×900 · Balanced</option>
                <option value="1280x720">1280×720 · Lightweight</option>
              </select>
            </label>
            <label className="field">
              <span>Interface scale</span>
              <select value={currentDensity} onChange={(event) => onChange({ ...config, desktopDensity: Number(event.target.value) })}>
                {densityOptions.map((value) => <option value={value} key={value}>{densityLabel(value)}</option>)}
              </select>
            </label>
            <label className="field">
              <span>Frame rate</span>
              <select value={config.maxFps} onChange={(event) => onChange({ ...config, maxFps: Number(event.target.value) })}>
                <option value={30}>30 FPS · efficient</option>
                <option value={60}>60 FPS · smooth</option>
              </select>
            </label>
          </div>

          <div className="toggle-list">
            <label className="toggle-row">
              <span>Flex Display · advanced, resize Android with the PC window</span>
              <input type="checkbox" checked={Boolean(config.desktopFlex)} onChange={(event) => onChange({ ...config, desktopFlex: event.target.checked })} disabled={!capabilities.flexSupported} />
              <i />
            </label>
            <label className="toggle-row">
              <span>System bars & navigation</span>
              <input type="checkbox" checked={!config.desktopNoDecorations} onChange={(event) => onChange({ ...config, desktopNoDecorations: !event.target.checked })} disabled={!capabilities.systemDecorationsSupported} />
              <i />
            </label>
            <label className="toggle-row">
              <span>Keep desktop apps after closing the window</span>
              <input type="checkbox" checked={Boolean(config.desktopKeepContent)} onChange={(event) => onChange({ ...config, desktopKeepContent: event.target.checked })} disabled={!capabilities.keepContentSupported} />
              <i />
            </label>
          </div>

          <div className="smart-note">
            <CheckCircle2 size={17} />
            <span>{capabilities.message} {capabilities.launcherPackage ? `The normal phone launcher (${capabilities.launcherPackage}) was detected but will intentionally NOT be forced onto this display.` : "SCRCPY Studio will not force a phone launcher onto the new display."} Flex Display starts off because shrinking below desktop-class dimensions can make Android return to a phone-style layout.</span>
          </div>
        </>
      )}
    </div>
  );
}
