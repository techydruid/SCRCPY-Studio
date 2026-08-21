import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, CircleAlert, RefreshCw, RotateCcw, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import type { DesktopCapabilities, DesktopExperienceResult, DeviceInfo, LaunchConfig } from "./types";

type Props = {
  serial: string;
  config: LaunchConfig;
  onChange: (next: LaunchConfig) => void;
  onStatus: (message: string) => void;
};

type RestartFlow = "enabling" | "restoring" | null;

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

function sleep(ms: number) {
  return new Promise<void>((resolve) => window.setTimeout(resolve, ms));
}

export default function DesktopControls({ serial, config, onChange, onStatus }: Props) {
  const [capabilities, setCapabilities] = useState<DesktopCapabilities | null>(null);
  const [loading, setLoading] = useState(true);
  const [desktopBusy, setDesktopBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [actionError, setActionError] = useState(false);
  const [restartFlow, setRestartFlow] = useState<RestartFlow>(null);
  const [restartDetail, setRestartDetail] = useState<string | null>(null);
  const [reconnectTimedOut, setReconnectTimedOut] = useState(false);

  // The Desktop card and the global Launch button must never disagree about
  // readiness. The card is driven by fresh capability results, while the
  // Launch button is driven by config.desktopSupported in App.tsx. Reboots
  // and async device re-probes can otherwise leave that config flag stale.
  useEffect(() => {
    const verifiedReady = Boolean(
      capabilities?.supported &&
      capabilities.desktopExperiencePrepared &&
      !loading &&
      !restartFlow &&
      !error
    );

    if (config.desktopSupported !== verifiedReady) {
      onChange({ ...config, desktopSupported: verifiedReady });
    }
  }, [
    capabilities?.supported,
    capabilities?.desktopExperiencePrepared,
    loading,
    restartFlow,
    error,
    config,
    onChange
  ]);

  const applyCapabilities = (result: DesktopCapabilities) => {
    setCapabilities(result);
    onChange({
      ...config,
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
  };

  const probeNow = async (showLoading = true) => {
    if (showLoading) setLoading(true);
    setError(null);
    try {
      const result = await invoke<DesktopCapabilities>("probe_desktop_capabilities", { serial });
      applyCapabilities(result);
      onStatus(result.message);
      return result;
    } catch (reason) {
      const message = String(reason);
      setError(message);
      onChange({ ...config, desktopSupported: false });
      onStatus(`Desktop Mode check failed: ${message}`);
      throw reason;
    } finally {
      if (showLoading) setLoading(false);
    }
  };

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setActionMessage(null);
    setActionError(false);
    setRestartFlow(null);
    setRestartDetail(null);
    setReconnectTimedOut(false);
    setCapabilities(null);
    onChange({ ...config, desktopSupported: false });
    onStatus("Checking virtual-display support and Android desktop-windowing settings…");

    void invoke<DesktopCapabilities>("probe_desktop_capabilities", { serial })
      .then((result) => {
        if (cancelled) return;
        setCapabilities(result);
        onChange({
          ...config,
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

  const waitForPhoneAfterRestart = async (intent: Exclude<RestartFlow, null>) => {
    const deadline = Date.now() + 120_000;
    let deviceSeen = false;

    setReconnectTimedOut(false);
    setRestartFlow(intent);
    setRestartDetail("Phone is restarting. Keep the USB cable connected — SCRCPY Studio will continue automatically.");
    onStatus("Phone restarting — waiting for the same device to reconnect automatically…");

    await sleep(2500);

    while (Date.now() < deadline) {
      const devices = await invoke<DeviceInfo[]>("list_devices").catch(() => [] as DeviceInfo[]);
      const sameDevice = devices.find((device) => device.serial === serial);

      if (!sameDevice) {
        setRestartDetail("Waiting for the same phone to reappear over ADB… Keep the USB cable connected.");
        await sleep(2000);
        continue;
      }

      deviceSeen = true;
      if (sameDevice.state !== "device") {
        if (sameDevice.state === "unauthorized") {
          setRestartDetail("Phone detected. Unlock it and approve the USB debugging prompt if Android asks.");
        } else {
          setRestartDetail(`Phone detected, but ADB is still ${sameDevice.state}. Waiting for Android to finish starting…`);
        }
        await sleep(2000);
        continue;
      }

      setRestartDetail("Phone is back. Waiting a moment for Android services to finish starting…");
      await sleep(3500);

      try {
        const result = await invoke<DesktopCapabilities>("probe_desktop_capabilities", { serial });
        setCapabilities(result);

        if (intent === "enabling") {
          if (result.supported && result.desktopExperiencePrepared) {
            onChange({
              ...config,
              desktopSupported: true,
              desktopWidth: result.recommendedWidth,
              desktopHeight: result.recommendedHeight,
              desktopDensity: result.recommendedDensity,
              desktopFlex: false,
              desktopNoDecorations: false,
              desktopKeepContent: false,
              desktopStartApp: null,
              stayAwake: true
            });
            const message = "Phone restarted and Desktop UI setup is ready. You can click Launch Desktop now.";
            setActionMessage(message);
            setActionError(false);
            setRestartFlow(null);
            setRestartDetail(null);
            onStatus(message);
            return true;
          }

          setRestartDetail("Phone is connected again. Android has not reported the Desktop UI settings as fully active yet, so SCRCPY Studio is checking again…");
        } else {
          onChange({ ...config, desktopSupported: false });
          const message = "Phone restarted and the original Android desktop settings have been restored.";
          setActionMessage(message);
          setActionError(false);
          setRestartFlow(null);
          setRestartDetail(null);
          onStatus(message);
          return true;
        }
      } catch {
        setRestartDetail("Phone is connected, but Android is still finishing startup. Retrying the Desktop UI check…");
      }

      await sleep(3000);
    }

    const message = deviceSeen
      ? "The phone reconnected, but Desktop UI verification did not finish within 2 minutes. Click Recheck Desktop Setup below."
      : "SCRCPY Studio could not see the phone again within 2 minutes. Keep the USB cable connected, unlock the phone, then click Recheck Desktop Setup.";
    setReconnectTimedOut(true);
    setRestartFlow(null);
    setRestartDetail(null);
    setActionMessage(message);
    setActionError(true);
    onStatus(message);
    return false;
  };

  const enableDesktopUi = async () => {
    setDesktopBusy(true);
    setActionMessage(null);
    setActionError(false);
    setReconnectTimedOut(false);
    onStatus("Backing up the phone's current desktop developer settings, enabling Desktop UI, then restarting once…");
    try {
      const result = await invoke<DesktopExperienceResult>("enable_desktop_experience", { serial });
      if (result.rebootStarted) {
        await waitForPhoneAfterRestart("enabling");
      } else {
        setActionMessage(result.message);
        setActionError(!result.prepared);
        onStatus(result.message);
        await probeNow(false).catch(() => undefined);
      }
    } catch (reason) {
      const message = `Could not prepare Desktop UI: ${String(reason)}`;
      setActionMessage(message);
      setActionError(true);
      setRestartFlow(null);
      setRestartDetail(null);
      onStatus(message);
    } finally {
      setDesktopBusy(false);
    }
  };

  const restoreDesktopUi = async () => {
    setDesktopBusy(true);
    setActionMessage(null);
    setActionError(false);
    setReconnectTimedOut(false);
    onStatus("Restoring the phone's original desktop developer settings…");
    try {
      const result = await invoke<DesktopExperienceResult>("restore_desktop_experience", { serial });
      if (result.rebootStarted) {
        await waitForPhoneAfterRestart("restoring");
      } else {
        setActionMessage(result.message);
        setActionError(false);
        onStatus(result.message);
      }
    } catch (reason) {
      const message = `Could not restore Desktop UI settings: ${String(reason)}`;
      setActionMessage(message);
      setActionError(true);
      setRestartFlow(null);
      setRestartDetail(null);
      onStatus(message);
    } finally {
      setDesktopBusy(false);
    }
  };

  const recheckDesktopSetup = async () => {
    setDesktopBusy(true);
    setReconnectTimedOut(false);
    setActionMessage(null);
    setActionError(false);
    onStatus("Rechecking Desktop UI setup on the connected phone…");
    try {
      const result = await probeNow(false);
      if (result.supported && result.desktopExperiencePrepared) {
        const message = "Desktop UI setup is ready. You can click Launch Desktop now.";
        setActionMessage(message);
        setActionError(false);
        onStatus(message);
      } else {
        const message = result.supported
          ? "Virtual display works, but Android still reports that Desktop UI preparation is incomplete."
          : result.message;
        setActionMessage(message);
        setActionError(true);
        onStatus(message);
      }
    } catch {
      setActionMessage("The phone is not ready for the Desktop UI check yet. Confirm USB debugging is connected and try again.");
      setActionError(true);
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
  const maxDesktopDensity = Math.floor((desktopHeight * 160) / 600);
  const densityOptions = [180, 200, 240, 284].filter((value) => value <= maxDesktopDensity);
  const currentDensity = config.desktopDensity ?? capabilities?.recommendedDensity ?? 240;
  if (!densityOptions.includes(currentDensity) && currentDensity <= maxDesktopDensity) {
    densityOptions.push(currentDensity);
    densityOptions.sort((a, b) => a - b);
  }

  const status = restartFlow
    ? "Restarting…"
    : loading
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

      {restartFlow ? (
        <>
          <div className="smart-note">
            <RefreshCw size={17} className="spin" />
            <span>{restartDetail || "Waiting for the phone to restart and reconnect…"}</span>
          </div>
          <div className="finding info" style={{ marginTop: 10 }}>
            <CheckCircle2 size={18} />
            <div>
              <strong>Setup will continue automatically</strong>
              <span>Do not press the Enable button again. SCRCPY Studio is watching ADB and will verify the same phone as soon as Android finishes booting.</span>
            </div>
          </div>
        </>
      ) : loading ? (
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
            <span>Enable Desktop UI backs up the current Android developer-setting values, enables desktop-on-secondary-display, freeform windows and resizable-app support, then restarts the phone once. SCRCPY Studio will now wait for the phone and continue setup automatically after the restart.</span>
          </div>
          <button className="primary launch" onClick={() => void enableDesktopUi()} disabled={desktopBusy || !capabilities.desktopExperienceCanPrepare}>
            {desktopBusy ? <RefreshCw size={17} className="spin" /> : <Sparkles size={17} />}
            {desktopBusy ? "Waiting for phone…" : "Enable Desktop UI & Restart"}
          </button>
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
              {desktopBusy ? "Waiting for phone…" : "Restore Phone Defaults & Restart"}
            </button>
          )}
        </>
      )}

      {actionMessage && !restartFlow && (
        <div className={`finding ${actionError ? "warning" : "ok"}`} style={{ marginTop: 10 }}>
          {actionError ? <CircleAlert size={18} /> : <CheckCircle2 size={18} />}
          <div><strong>{actionError ? "Desktop setup needs attention" : "Desktop setup updated"}</strong><span>{actionMessage}</span></div>
        </div>
      )}

      {reconnectTimedOut && !restartFlow && (
        <button className="secondary wide" style={{ marginTop: 10 }} onClick={() => void recheckDesktopSetup()} disabled={desktopBusy}>
          {desktopBusy ? <RefreshCw size={16} className="spin" /> : <RefreshCw size={16} />}
          {desktopBusy ? "Rechecking…" : "Recheck Desktop Setup"}
        </button>
      )}
    </div>
  );
}
