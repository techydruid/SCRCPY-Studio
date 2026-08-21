import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, CircleAlert, RefreshCw, RotateCcw, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import type { DesktopCapabilities, DesktopExperienceResult, LaunchConfig } from "./types";

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
  const [desktopBusy, setDesktopBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setActionMessage(null);
    setCapabilities(null);
    onChange({ ...config, desktopSupported: false });
    onStatus("Checking virtual-display support and Android desktop-windowing settings…");

    void invoke<DesktopCapabilities>("probe_desktop_capabilities", { serial })
      .then((result) => {
        if (cancelled) return;
        setCapabilities(result);
        onChange({
          ...config,
          // Desktop Mode is only launchable when BOTH pieces are true: scrcpy
          // can create a virtual display and Android is prepared to render a
          // desktop-style environment on secondary displays.
          desktopSupported: result.supported && result.desktopExperiencePrepared,
          desktopWidth: result.recommendedWidth,
          desktopHeight: result.recommendedHeight,
          desktopDensity: result.recommendedDensity,
          desktopFlex: false,
          desktopNoDecorations: false,
          desktopKeepContent: false,
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

  const enableDesktopUi = async () => {
    setDesktopBusy(true);
    setActionMessage(null);
    onStatus("Backing up the phone's current desktop developer settings, enabling Desktop UI, then restarting once…");
    try {
      const result = await invoke<DesktopExperienceResult>("enable_desktop_experience", { serial });
      setActionMessage(result.message);
      onChange({ ...config, desktopSupported: false });
      onStatus(result.message);
    } catch (reason) {
      const message = `Could not prepare Desktop UI: ${String(reason)}`;
      setActionMessage(message);
      onStatus(message);
    } finally {
      setDesktopBusy(false);
    }
  };

  const restoreDesktopUi = async () => {
    setDesktopBusy(true);
    setActionMessage(null);
    onStatus("Restoring the phone's original desktop developer settings…");
    try {
      const result = await invoke<DesktopExperienceResult>("restore_desktop_experience", { serial });
      setActionMessage(result.message);
      onChange({ ...config, desktopSupported: false });
      onStatus(result.message);
    } catch (reason) {
      const message = `Could not restore Desktop UI settings: ${String(reason)}`;
      setActionMessage(message);
      onStatus(message);
    } finally {
      setDesktopBusy(false);
    }
  };

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

  const status = loading
    ? "Checking…"
    : !capabilities?.supported
      ? "Unavailable"
      : capabilities.desktopExperiencePrepared
        ? "Ready"
        : "Needs setup";

  return (
    <div className="creator-tools">
      <div className="creator-tools-heading">
        <div><span className="eyebrow">SMART DESKTOP</span><strong>Desktop environment</strong></div>
        <span>{status}</span>
      </div>

      {loading ? (
        <div className="smart-note"><RefreshCw size={17} className="spin" /><span>SCRCPY Studio is checking two separate things: whether scrcpy can create a secondary display, and whether Android is configured to render desktop windowing on that display.</span></div>
      ) : error ? (
        <div className="finding error"><CircleAlert size={18} /><div><strong>Desktop Mode could not be verified</strong><span>{error}</span></div></div>
      ) : !capabilities?.supported ? (
        <div className="finding warning"><CircleAlert size={18} /><div><strong>Virtual display unavailable</strong><span>{capabilities?.message || "This phone/runtime combination did not pass the virtual-display check."}</span></div></div>
      ) : !capabilities.desktopExperiencePrepared ? (
        <>
          <div className="finding warning">
            <CircleAlert size={18} />
            <div>
              <strong>Virtual display works, but Desktop UI is not prepared</strong>
              <span>{capabilities.desktopExperienceSummary} SCRCPY Studio will not call a phone-style secondary launcher “Desktop Mode.”</span>
            </div>
          </div>
          <div className="smart-note">
            <Sparkles size={17} />
            <span>Enable Desktop UI backs up the current Android developer-setting values, enables desktop-on-secondary-display, freeform windows and resizable-app support, then restarts the phone once. No root is used. You can restore the original values later.</span>
          </div>
          <button className="primary launch" onClick={() => void enableDesktopUi()} disabled={desktopBusy || !capabilities.desktopExperienceCanPrepare}>
            {desktopBusy ? <RefreshCw size={17} className="spin" /> : <Sparkles size={17} />}
            {desktopBusy ? "Preparing Desktop UI…" : "Enable Desktop UI & Restart"}
          </button>
          {actionMessage && <div className="smart-note"><CheckCircle2 size={17} /><span>{actionMessage}</span></div>}
        </>
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
            <span>{capabilities.desktopExperienceSummary} {capabilities.message} Flex Display starts off so Android remains above desktop-class dimensions.</span>
          </div>

          {capabilities.desktopExperienceBackupAvailable && (
            <button className="secondary wide" onClick={() => void restoreDesktopUi()} disabled={desktopBusy}>
              {desktopBusy ? <RefreshCw size={16} className="spin" /> : <RotateCcw size={16} />}
              {desktopBusy ? "Restoring…" : "Restore Phone Defaults & Restart"}
            </button>
          )}
          {actionMessage && <div className="smart-note"><CheckCircle2 size={17} /><span>{actionMessage}</span></div>}
        </>
      )}
    </div>
  );
}
