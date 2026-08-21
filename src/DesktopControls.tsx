import { invoke } from "@tauri-apps/api/core";
import {
  CheckCircle2,
  CircleAlert,
  FolderOpen,
  Monitor,
  RefreshCw,
  RotateCcw,
  Sparkles
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type Dispatch, type SetStateAction } from "react";
import type {
  DesktopCapabilities,
  DesktopDiagnostics,
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

function readinessLabel(available: boolean, active: boolean, activeLabel: string) {
  if (active) return activeLabel;
  return available ? "Available, not active" : "Not exposed";
}

function CapabilityRow({ label, value, ready }: { label: string; value: string; ready: boolean }) {
  return (
    <div className="desktop-capability-row">
      {ready ? <CheckCircle2 size={16} /> : <CircleAlert size={16} />}
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function Diagnostics({ diagnostics }: { diagnostics: DesktopDiagnostics }) {
  const display = diagnostics.displayId == null
    ? "Not created or not exposed"
    : `#${diagnostics.displayId}${diagnostics.displayName ? ` · ${diagnostics.displayName}` : ""}`;
  const geometry = diagnostics.resolution
    ? `${diagnostics.resolution}${diagnostics.density ? ` / ${diagnostics.density} dpi` : ""}`
    : "Unknown";

  return (
    <details className="desktop-diagnostics">
      <summary>Technical diagnostics</summary>
      <div className="desktop-diagnostic-grid">
        <span>Exit result</span><code>{diagnostics.exitResult || "not run"}</code>
        <span>Display</span><code>{display}</code>
        <span>Size / density</span><code>{geometry}</code>
        <span>Windowing</span><code>{diagnostics.windowingMode || "unknown"}</code>
        <span>Running activity</span><code>{diagnostics.launcherActivity || "Not observed"}</code>
      </div>
      <label>Exact scrcpy command</label>
      <pre>{diagnostics.command || "Probe not run"}</pre>
      <label>Relevant Android settings</label>
      <pre>{diagnostics.relevantSettings.length
        ? diagnostics.relevantSettings.map((item) => `${item.key}=${item.value ?? "<unset>"}`).join("\n")
        : "No settings captured"}</pre>
      {diagnostics.platformEvidence.length > 0 && (
        <>
          <label>Platform evidence</label>
          <pre>{diagnostics.platformEvidence.join("\n")}</pre>
        </>
      )}
      {diagnostics.scrcpyOutput && (
        <>
          <label>scrcpy output</label>
          <pre>{diagnostics.scrcpyOutput}</pre>
        </>
      )}
      {diagnostics.logPath && <small>Saved to {diagnostics.logPath}</small>}
    </details>
  );
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

  const diagnostics = lastLaunchResult?.desktopDiagnostics ?? capabilities?.diagnostics;
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
        <div className="smart-note"><RefreshCw size={17} className="spin" /><span>The check takes a few seconds because SCRCPY Studio creates a real temporary display and inspects WindowManager instead of trusting developer settings.</span></div>
      )}

      {error && <div className="finding error"><CircleAlert size={18} /><div><strong>Desktop check failed</strong><span>{error}</span></div></div>}

      {capabilities && (
        <>
          <div className="desktop-capabilities">
            <CapabilityRow label="Virtual Display" value={capabilities.virtualDisplaySupported ? "Ready" : "Unavailable"} ready={capabilities.virtualDisplaySupported} />
            <CapabilityRow
              label="Android Desktop Windowing"
              value={readinessLabel(capabilities.androidDesktopWindowingAvailable, capabilities.androidDesktopWindowingActive, "Verified active")}
              ready={capabilities.androidDesktopWindowingActive}
            />
            <CapabilityRow
              label="Samsung DeX"
              value={readinessLabel(capabilities.samsungDexAvailable, capabilities.samsungDexActive, "Active and capturable")}
              ready={capabilities.samsungDexActive}
            />
          </div>

          <div className="desktop-verdict">
            <Monitor size={18} />
            <div><strong>{capabilities.desktopExperienceSummary}</strong><span>{capabilities.message}</span></div>
          </div>

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
              <div className="smart-note"><Sparkles size={17} /><span>Android exposes desktop-related support, but it is not active. You can try the reversible developer settings; the next probe must still verify the real windowing mode.</span></div>
              <button className="secondary wide" onClick={() => void changeDeveloperSettings("enable_desktop_experience")} disabled={busy}>
                {busy ? <RefreshCw size={16} className="spin" /> : <Sparkles size={16} />} Try Android Desktop Windowing & Restart
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

          {diagnostics && <Diagnostics diagnostics={diagnostics} />}
        </>
      )}

      <div className="desktop-actions">
        <button className="secondary" onClick={() => void probe()} disabled={busy}><RefreshCw size={16} className={busy ? "spin" : ""} /> Recheck</button>
        <button className="secondary" onClick={() => void openDiagnostics()}><FolderOpen size={16} /> Open Logs</button>
      </div>
    </div>
  );
}
