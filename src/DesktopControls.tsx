import { invoke } from "@tauri-apps/api/core";
import {
  CheckCircle2,
  CircleAlert,
  FolderOpen,
  RefreshCw,
  RotateCcw,
  Sparkles
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type Dispatch, type SetStateAction } from "react";
import type {
  DesktopCapabilities,
  DesktopExperienceResult,
  DesktopProbeState,
  LaunchConfig,
  LaunchResult
} from "./types";
import "./desktop.css";

interface Props {
  serial: string;
  config: LaunchConfig;
  onChange: Dispatch<SetStateAction<LaunchConfig | null>>;
  onStatus: (status: string) => void;
  onProbeStateChange: (state: DesktopProbeState) => void;
  lastLaunchResult?: LaunchResult | null;
}

const layouts = [
  { label: "Balanced · 1920 × 1080", value: "1920x1080", width: 1920, height: 1080 },
  { label: "Lightweight · 1280 × 720", value: "1280x720", width: 1280, height: 720 },
  { label: "Large canvas · 2560 × 1440", value: "2560x1440", width: 2560, height: 1440 }
];

function layoutValue(width?: number | null, height?: number | null) {
  return `${width ?? 1920}x${height ?? 1080}`;
}

function densityForLayout(height: number, preferred: number) {
  const desktopMinimum = Math.floor((height * 160) / 600);
  return Math.min(preferred, Math.max(120, desktopMinimum));
}

export default function DesktopControls({ serial, config, onChange, onStatus, onProbeStateChange, lastLaunchResult }: Props) {
  const [capabilities, setCapabilities] = useState<DesktopCapabilities | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeSerial = useRef(serial);

  const updateConfig = useCallback((patch: Partial<LaunchConfig>) => {
    onChange((current) => {
      if (!current || current.serial !== serial || current.mode !== "desktop") return current;
      return { ...current, ...patch };
    });
  }, [onChange, serial]);

  const applyCapabilities = useCallback((result: DesktopCapabilities) => {
    setCapabilities(result);
    setError(null);
    updateConfig({
      desktopEnvironment: result.environmentKind,
      desktopDisplayId: result.existingDisplayId ?? null,
      desktopWidth: result.recommendedWidth,
      desktopHeight: result.recommendedHeight,
      desktopDensity: result.recommendedDensity,
      desktopFlex: false,
      desktopNoDecorations: false,
      desktopKeepContent: false,
      desktopStartApp: null
    });
    onProbeStateChange({ serial, checking: false, capabilities: result, error: null });
    onStatus(result.message);
  }, [onProbeStateChange, onStatus, serial, updateConfig]);

  const probe = useCallback(async () => {
    setBusy(true);
    setError(null);
    onProbeStateChange({ serial, checking: true, capabilities: null, error: null });
    onStatus("Creating a temporary display and checking its real Android windowing state…");
    try {
      const result = await invoke<DesktopCapabilities>("probe_desktop_capabilities", { serial });
      if (activeSerial.current === serial) applyCapabilities(result);
      return result;
    } catch (probeError) {
      const message = String(probeError);
      if (activeSerial.current === serial) {
        setCapabilities(null);
        setError(message);
        updateConfig({ desktopEnvironment: "unavailable", desktopDisplayId: null });
        onProbeStateChange({ serial, checking: false, capabilities: null, error: message });
        onStatus(`Desktop check failed: ${message}`);
      }
      return null;
    } finally {
      if (activeSerial.current === serial) setBusy(false);
    }
  }, [applyCapabilities, onProbeStateChange, onStatus, serial, updateConfig]);

  useEffect(() => {
    activeSerial.current = serial;
    setCapabilities(null);
    setError(null);
    void probe();
    return () => {
      activeSerial.current = "";
    };
    // A new serial must trigger one evidence probe. Config is intentionally not a dependency.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serial]);

  const waitForRebootAndProbe = async (message: string) => {
    onStatus(message);
    const deadline = Date.now() + 180_000;
    while (Date.now() < deadline && activeSerial.current === serial) {
      await new Promise((resolve) => window.setTimeout(resolve, 3000));
      try {
        const result = await invoke<DesktopCapabilities>("probe_desktop_capabilities", { serial });
        applyCapabilities(result);
        return;
      } catch {
        onStatus("Phone is restarting. Waiting for ADB to reconnect…");
      }
    }
    throw new Error("The phone did not reconnect within three minutes. Reconnect it, then choose Recheck.");
  };

  const changeDeveloperSettings = async (command: "enable_desktop_experience" | "restore_desktop_experience") => {
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<DesktopExperienceResult>(command, { serial });
      if (result.rebootStarted) {
        await waitForRebootAndProbe(result.message);
      } else {
        await probe();
      }
    } catch (changeError) {
      const message = String(changeError);
      setError(message);
      onStatus(message);
    } finally {
      setBusy(false);
    }
  };

  const openDiagnostics = async () => {
    try {
      const folder = await invoke<string>("open_desktop_diagnostics");
      onStatus(`Opened Desktop Diagnostics: ${folder}`);
    } catch (openError) {
      onStatus(`Could not open Desktop Diagnostics: ${String(openError)}`);
    }
  };

  const setLayout = (value: string) => {
    const layout = layouts.find((item) => item.value === value);
    if (!layout) return;
    updateConfig({
      desktopWidth: layout.width,
      desktopHeight: layout.height,
      desktopDensity: densityForLayout(layout.height, capabilities?.recommendedDensity ?? 240)
    });
  };

  const isExistingDisplay = capabilities?.environmentKind === "samsung_dex";
  const currentDensity = config.desktopDensity ?? capabilities?.recommendedDensity ?? 240;
  const desktopHeight = config.desktopHeight ?? capabilities?.recommendedHeight ?? 1080;
  const maxDesktopDensity = Math.floor((desktopHeight * 160) / 600);
  const densities = useMemo(
    () => [160, 180, 200, 240, 284, 320].filter((density) => density <= maxDesktopDensity),
    [maxDesktopDensity]
  );

  return (
    <div className="desktop-controls">
      <div className="desktop-heading">
        <div>
          <span className="eyebrow">DISPLAY ENVIRONMENT</span>
          <strong>{capabilities?.environmentLabel ?? (busy ? "Checking…" : "Not checked")}</strong>
        </div>
        <span className={`desktop-state ${capabilities?.supported ? "ready" : "waiting"}`}>
          {busy ? <RefreshCw size={14} className="spin" /> : capabilities?.supported ? <CheckCircle2 size={14} /> : <CircleAlert size={14} />}
          {busy ? "Inspecting" : capabilities?.supported ? "Ready" : "Unavailable"}
        </span>
      </div>

      {busy && !capabilities && (
        <div className="inline-state"><RefreshCw size={16} className="spin" /> Checking display support</div>
      )}

      {error && <div className="finding error"><CircleAlert size={18} /><div><strong>Desktop check failed</strong><span>{error}</span></div></div>}

      {capabilities && (
        <>
          {!isExistingDisplay && capabilities.virtualDisplaySupported && (
            <div className="desktop-options">
              <label className="field">
                <span>Display size</span>
                <select value={layoutValue(config.desktopWidth, config.desktopHeight)} onChange={(event) => setLayout(event.target.value)}>
                  {layouts.map((layout) => <option value={layout.value} key={layout.value}>{layout.label}</option>)}
                </select>
              </label>
              <label className="field">
                <span>Interface density</span>
                <select value={currentDensity} onChange={(event) => updateConfig({ desktopDensity: Number(event.target.value) })}>
                  {densities.map((density) => <option value={density} key={density}>{density} dpi</option>)}
                </select>
              </label>
              <div className="toggle-list desktop-toggles">
                <label className="toggle-row"><span>Flex Display compatibility</span><input type="checkbox" checked={Boolean(config.desktopFlex)} onChange={(event) => updateConfig({ desktopFlex: event.target.checked })} disabled={!capabilities.flexSupported} /><i /></label>
                <label className="toggle-row"><span>Android system decorations</span><input type="checkbox" checked={!config.desktopNoDecorations} onChange={(event) => updateConfig({ desktopNoDecorations: !event.target.checked })} disabled={!capabilities.systemDecorationsSupported} /><i /></label>
                <label className="toggle-row"><span>Keep apps after closing</span><input type="checkbox" checked={Boolean(config.desktopKeepContent)} onChange={(event) => updateConfig({ desktopKeepContent: event.target.checked })} disabled={!capabilities.keepContentSupported} /><i /></label>
              </div>
            </div>
          )}

          {capabilities.desktopExperienceCanPrepare && (
            <div className="desktop-setup">
              <button className="secondary wide" onClick={() => void changeDeveloperSettings("enable_desktop_experience")} disabled={busy}>
                {busy ? <RefreshCw size={16} className="spin" /> : <Sparkles size={16} />} Enable Desktop Windowing & Restart
              </button>
            </div>
          )}

          {capabilities.desktopExperienceBackupAvailable && (
            <button className="secondary wide" onClick={() => void changeDeveloperSettings("restore_desktop_experience")} disabled={busy}>
              {busy ? <RefreshCw size={16} className="spin" /> : <RotateCcw size={16} />} Restore Original Android Settings & Restart
            </button>
          )}

          {lastLaunchResult && !lastLaunchResult.started && (
            <div className="finding error"><CircleAlert size={18} /><div><strong>Launch did not stay running</strong><span>{lastLaunchResult.message}</span></div></div>
          )}

        </>
      )}

      <div className="desktop-actions">
        <button className="secondary" onClick={() => void probe()} disabled={busy}><RefreshCw size={16} className={busy ? "spin" : ""} /> Recheck</button>
        <button className="secondary" onClick={() => void openDiagnostics()}><FolderOpen size={16} /> Open Logs</button>
      </div>
    </div>
  );
}
