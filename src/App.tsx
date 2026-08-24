import { invoke } from "@tauri-apps/api/core";
import {
  Camera,
  CheckCircle2,
  ChevronDown,
  CircleAlert,
  Clapperboard,
  Download,
  FolderOpen,
  Image,
  Monitor,
  MonitorSmartphone,
  Play,
  Radio,
  RefreshCw,
  Usb,
  Wifi,
  X
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import AdvancedSettings from "./AdvancedSettings";
import CameraControls from "./CameraControls";
import DesktopControls from "./DesktopControls";
import type {
  CameraCapabilities,
  DeviceInfo,
  DeviceProfile,
  DesktopProbeState,
  LaunchConfig,
  LaunchResult,
  Recommendation,
  RememberedWirelessDevice,
  RuntimeStatus,
  SessionMode,
  TransportSwitchResult
} from "./types";

const modeMeta: Array<{
  id: Extract<SessionMode, "creator" | "camera" | "desktop">;
  label: string;
  icon: typeof MonitorSmartphone;
}> = [
  { id: "creator", label: "Mirror Phone", icon: Clapperboard },
  { id: "camera", label: "Camera Mode", icon: Camera },
  { id: "desktop", label: "Desktop Mode", icon: Monitor }
];

function pill(text: string) {
  return <span className="pill">{text}</span>;
}

function shortVersion(value?: string | null) {
  return value?.split(" <")[0]?.trim() || value || "";
}

function modePreparationText(mode: SessionMode) {
  if (mode === "camera") return "Loading camera settings…";
  if (mode === "desktop") return "Checking desktop support…";
  return "Preparing mirror settings…";
}

function App() {
  const [runtime, setRuntime] = useState<RuntimeStatus | null>(null);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedSerial, setSelectedSerial] = useState<string>("");
  const [profile, setProfile] = useState<DeviceProfile | null>(null);
  const [mode, setMode] = useState<SessionMode>("creator");
  const [recommendation, setRecommendation] = useState<Recommendation | null>(null);
  const [config, setConfig] = useState<LaunchConfig | null>(null);
  const [modeLoading, setModeLoading] = useState(false);
  const [modeLoadError, setModeLoadError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [lastLaunchResult, setLastLaunchResult] = useState<LaunchResult | null>(null);
  const [cameraCapabilities, setCameraCapabilities] = useState<CameraCapabilities | null>(null);
  const [desktopProbe, setDesktopProbe] = useState<DesktopProbeState>({
    serial: "",
    checking: false,
    capabilities: null,
    error: null
  });
  const [installingRuntime, setInstallingRuntime] = useState(false);
  const [creatorBusy, setCreatorBusy] = useState(false);
  const [wirelessBusy, setWirelessBusy] = useState(false);
  const [statusText, setStatusText] = useState("Checking your setup…");
  const [wirelessOpen, setWirelessOpen] = useState(false);
  const [rememberedWireless, setRememberedWireless] = useState<RememberedWirelessDevice[]>([]);
  const [wirelessFeedback, setWirelessFeedback] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const [pairAddress, setPairAddress] = useState("");
  const [pairCode, setPairCode] = useState("");
  const [connectAddress, setConnectAddress] = useState("");

  const selectedDevice = useMemo(
    () => devices.find((d) => d.serial === selectedSerial) ?? null,
    [devices, selectedSerial]
  );

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      const runtimeResult = await invoke<RuntimeStatus>("runtime_status");
      setRuntime(runtimeResult);

      const deviceResult = runtimeResult.adbFound
        ? await invoke<DeviceInfo[]>("list_devices").catch(() => [] as DeviceInfo[])
        : [];
      setDevices(deviceResult);

      const firstReady = deviceResult.find((d) => d.state === "device");
      setSelectedSerial((current) => {
        if (current && deviceResult.some((d) => d.serial === current)) return current;
        return firstReady?.serial ?? deviceResult[0]?.serial ?? "";
      });

      if (!runtimeResult.adbFound && runtimeResult.scrcpyFound) {
        setStatusText("ADB required — install Android Platform Tools from your system package manager");
      } else if (!runtimeResult.adbFound || !runtimeResult.scrcpyFound) {
        setStatusText("Runtime missing — install the verified official scrcpy package to get started");
      } else {
        setStatusText(deviceResult.length ? `${deviceResult.length} device${deviceResult.length > 1 ? "s" : ""} detected` : "Runtime ready — connect an Android device");
      }
    } catch (error) {
      setStatusText(String(error));
    } finally {
      setBusy(false);
    }
  }, []);

  const loadRememberedWireless = useCallback(async () => {
    try {
      setRememberedWireless(await invoke<RememberedWirelessDevice[]>("list_remembered_wireless"));
    } catch {
      setRememberedWireless([]);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (wirelessOpen) void loadRememberedWireless();
  }, [wirelessOpen, loadRememberedWireless]);

  const selectMode = (nextMode: SessionMode) => {
    if (nextMode === mode) return;
    setMode(nextMode);
    setModeLoading(Boolean(selectedSerial && selectedDevice?.state === "device"));
    setModeLoadError(null);
    setRecommendation(null);
    setConfig(null);
    setCameraCapabilities(null);
    setLastLaunchResult(null);
  };

  useEffect(() => {
    if (!selectedSerial || selectedDevice?.state !== "device") {
      setModeLoading(false);
      setModeLoadError(null);
      setProfile(null);
      setRecommendation(null);
      setConfig(null);
      setDesktopProbe({ serial: selectedSerial, checking: false, capabilities: null, error: null });
      return;
    }

    let cancelled = false;
    setModeLoading(true);
    setModeLoadError(null);
    setDesktopProbe({
      serial: selectedSerial,
      checking: mode === "desktop",
      capabilities: null,
      error: null
    });
    const load = async () => {
      try {
        const [p, r] = await Promise.all([
          invoke<DeviceProfile>("inspect_device", { serial: selectedSerial }),
          invoke<Recommendation>("recommend_settings", { serial: selectedSerial, mode })
        ]);
        if (cancelled) return;
        setLastLaunchResult(null);
        setProfile(p);
        setRecommendation(r);
        setConfig({
          serial: selectedSerial,
          mode,
          maxSize: r.maxSize,
          maxFps: r.maxFps,
          codec: r.codec,
          videoBitRate: 8,
          videoEncoder: null,
          audio: r.audio,
          audioSource: mode === "camera" ? (r.audio ? "mic" : "off") : (r.audio ? "output" : "off"),
          stayAwake: r.stayAwake,
          turnScreenOff: r.turnScreenOff,
          showTouches: r.showTouches,
          record: false,
          fullscreen: false,
          captureOrientation: "auto",
          crop: null,
          cameraId: null,
          cameraFacing: null,
          cameraZoom: null,
          cameraTorch: false,
          cameraSize: null,
          cameraAspectRatio: "auto",
          cameraHighSpeed: false,
          desktopWidth: r.maxSize >= 1920 ? 1920 : 1280,
          desktopHeight: r.maxSize >= 1920 ? 1080 : 720,
          desktopDensity: r.maxSize >= 1920 ? 240 : 200,
          desktopFlex: false,
          desktopNoDecorations: false,
          desktopKeepContent: false,
          desktopStartApp: null,
          desktopEnvironment: null,
          desktopDisplayId: null
        });
      } catch (error) {
        if (!cancelled) {
          const message = String(error);
          setModeLoadError(message);
          setStatusText(`Could not inspect device: ${message}`);
        }
      } finally {
        if (!cancelled) setModeLoading(false);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [selectedSerial, selectedDevice?.state, mode]);

  const installRuntime = async () => {
    setInstallingRuntime(true);
    setStatusText("Downloading and verifying the latest official scrcpy runtime…");
    try {
      const installed = await invoke<RuntimeStatus>("install_official_runtime");
      setRuntime(installed);
      await refresh();
      if (!installed.adbFound) {
        setStatusText("scrcpy installed. Install ADB from your Linux package manager, then reopen SCRCPY Studio.");
      }
    } catch (error) {
      setStatusText(`Runtime install failed: ${String(error)}`);
    } finally {
      setInstallingRuntime(false);
    }
  };

  const launch = async () => {
    if (!config || config.mode !== mode || config.serial !== selectedSerial) {
      setStatusText("Finishing the selected mode setup — try again in a moment.");
      return;
    }
    setLaunching(true);
    setStatusText("Starting a smart session…");
    try {
      const result = await invoke<LaunchResult>("launch_session", { config, requestedMode: mode });
      setLastLaunchResult(result);
      setStatusText(result.recordingPath ? `${result.message} Recording: ${result.recordingPath}` : result.message);
    } catch (error) {
      setStatusText(`Launch failed: ${String(error)}`);
    } finally {
      setLaunching(false);
    }
  };

  const captureScreenshot = async () => {
    if (!selectedSerial) return;
    if (mode === "camera") {
      setStatusText("Camera Mode streams the camera directly. Enable Camera recording to save its output.");
      return;
    }
    const displayId = mode === "desktop"
      ? lastLaunchResult?.desktopDiagnostics?.displayId ?? null
      : null;
    if (mode === "desktop" && displayId == null) {
      setStatusText("Launch Desktop Mode before taking a screenshot of its display.");
      return;
    }
    setCreatorBusy(true);
    setStatusText(mode === "desktop" ? "Capturing Desktop Mode screenshot…" : "Capturing Android screenshot…");
    try {
      const path = await invoke<string>("capture_screenshot", { serial: selectedSerial, displayId });
      setStatusText(`Screenshot saved — ${path}`);
    } catch (error) {
      setStatusText(`Screenshot failed: ${String(error)}`);
    } finally {
      setCreatorBusy(false);
    }
  };

  const openMediaFolder = async () => {
    setCreatorBusy(true);
    try {
      const path = await invoke<string>("open_media_folder");
      setStatusText(`Opened media folder — ${path}`);
    } catch (error) {
      setStatusText(`Could not open media folder: ${String(error)}`);
    } finally {
      setCreatorBusy(false);
    }
  };

  const applyTransportResult = async (result: TransportSwitchResult) => {
    await Promise.all([refresh(), loadRememberedWireless()]);
    if (result.activeSerial) setSelectedSerial(result.activeSerial);
    setStatusText(result.message);
    setWirelessFeedback({ kind: "success", text: result.message });
  };

  const pair = async () => {
    setWirelessBusy(true);
    setWirelessFeedback(null);
    try {
      const message = await invoke<string>("pair_device", { address: pairAddress, code: pairCode });
      setStatusText(message);
      setWirelessFeedback({ kind: "success", text: message });
    } catch (error) {
      const message = `Pairing failed: ${String(error)}`;
      setStatusText(message);
      setWirelessFeedback({ kind: "error", text: message });
    } finally {
      setWirelessBusy(false);
    }
  };

  const connect = async () => {
    setWirelessBusy(true);
    setWirelessFeedback(null);
    try {
      const message = await invoke<string>("connect_device", { address: connectAddress });
      await Promise.all([refresh(), loadRememberedWireless()]);
      setSelectedSerial(connectAddress.trim());
      setStatusText(message);
      setWirelessFeedback({ kind: "success", text: `${message}. Wireless is now the active connection.` });
    } catch (error) {
      const message = `Wireless connection failed: ${String(error)}`;
      setStatusText(message);
      setWirelessFeedback({ kind: "error", text: message });
    } finally {
      setWirelessBusy(false);
    }
  };

  const reconnectWireless = async (address: string) => {
    setWirelessBusy(true);
    setWirelessFeedback(null);
    try {
      const message = await invoke<string>("reconnect_wireless_device", { address });
      await Promise.all([refresh(), loadRememberedWireless()]);
      setSelectedSerial(address);
      setStatusText(message);
      setWirelessFeedback({ kind: "success", text: `${message}. Wireless is now the active connection.` });
    } catch (error) {
      const message = `Reconnect failed: ${String(error)}`;
      setStatusText(message);
      setWirelessFeedback({ kind: "error", text: message });
    } finally {
      setWirelessBusy(false);
    }
  };

  const forgetWireless = async (address: string) => {
    setWirelessBusy(true);
    setWirelessFeedback(null);
    try {
      const result = await invoke<TransportSwitchResult>("forget_wireless_device", { address });
      await applyTransportResult(result);
    } catch (error) {
      const message = `Could not forget device: ${String(error)}`;
      setStatusText(message);
      setWirelessFeedback({ kind: "error", text: message });
    } finally {
      setWirelessBusy(false);
    }
  };

  const enableUsbWireless = async () => {
    if (!selectedSerial) return;
    setWirelessBusy(true);
    setWirelessFeedback(null);
    setStatusText("Switching the USB-connected phone to wireless ADB…");
    try {
      const result = await invoke<TransportSwitchResult>("enable_usb_wireless", { serial: selectedSerial });
      await applyTransportResult(result);
    } catch (error) {
      const message = `USB to wireless failed: ${String(error)}`;
      setStatusText(message);
      setWirelessFeedback({ kind: "error", text: message });
    } finally {
      setWirelessBusy(false);
    }
  };

  const useUsbInstead = async () => {
    if (!selectedSerial) return;
    setWirelessBusy(true);
    setWirelessFeedback(null);
    setStatusText("Checking for the same phone over USB…");
    try {
      const result = await invoke<TransportSwitchResult>("switch_to_usb", { serial: selectedSerial });
      await applyTransportResult(result);
    } catch (error) {
      const message = `Could not switch to USB: ${String(error)}`;
      setStatusText(message);
      setWirelessFeedback({ kind: "error", text: message });
    } finally {
      setWirelessBusy(false);
    }
  };

  const runtimeHealthy = Boolean(runtime?.adbFound && runtime?.scrcpyFound);
  const desktopReady = mode !== "desktop" || Boolean(
    desktopProbe.serial === selectedSerial &&
    !desktopProbe.checking &&
    desktopProbe.capabilities?.supported === true
  );
  const configReady = Boolean(config && config.mode === mode && config.serial === selectedSerial);
  const canLaunch = Boolean(configReady && selectedDevice?.state === "device" && runtimeHealthy && desktopReady);
  const activeConfig = configReady ? config : null;
  const deviceReady = selectedDevice?.state === "device";
  const preparationText = modePreparationText(mode);
  const workspacePlaceholder = !deviceReady
    ? "Connect a phone to begin"
    : modeLoadError
      ? "Could not prepare this mode"
      : preparationText;
  const settingsPlaceholder = !deviceReady
    ? "Waiting for device"
    : modeLoadError
      ? "Settings unavailable"
      : preparationText;
  const desktopCaptureId = mode === "desktop" && lastLaunchResult?.started
    ? lastLaunchResult.desktopDiagnostics?.displayId ?? null
    : null;
  const screenshotLabel = mode === "camera" ? "Capture Frame" : "Screenshot";
  const recordingLabel = mode === "camera"
    ? "Camera recording"
    : mode === "desktop"
      ? "Desktop recording"
      : "Screen recording";
  const screenshotTitle = mode === "camera"
    ? "Direct still-frame capture is not available for scrcpy camera streams. Enable Camera recording to save the camera output."
    : mode === "desktop" && desktopCaptureId == null
      ? "Launch Desktop Mode first so SCRCPY Studio can target its display."
      : mode === "desktop"
        ? "Capture the launched Desktop Mode display"
        : "Capture the phone display";
  const screenshotDisabled = creatorBusy || !canLaunch || mode === "camera" || (mode === "desktop" && desktopCaptureId == null);
  const deviceName = selectedDevice?.model?.replaceAll("_", " ") || profile?.model || "No device";
  const deviceMeta = profile
    ? [
        profile.connectionKind === "usb" ? "USB" : "Wireless",
        `Android ${profile.androidVersion}`,
        profile.width && profile.height ? `${profile.width}×${profile.height}` : null,
        profile.h265Available ? "H.265" : "H.264"
      ].filter(Boolean).join(" · ")
    : runtimeHealthy ? "Connect an authorized Android device" : "Runtime required";
  const desktopLaunchLabel = desktopProbe.checking
    ? "Checking Display Support…"
    : desktopProbe.capabilities?.launchLabel ?? (desktopProbe.error ? "Display Support Unavailable" : "Checking Display Support…");

  const resetToAuto = () => {
    if (!activeConfig || !recommendation) return;
    const recommendedCamera = cameraCapabilities?.cameras.find((camera) => camera.id === cameraCapabilities.recommendedCameraId)
      ?? cameraCapabilities?.cameras[0]
      ?? null;
    const desktopCapabilities = desktopProbe.capabilities;
    setConfig({
      ...activeConfig,
      maxSize: recommendation.maxSize,
      maxFps: recommendation.maxFps,
      codec: recommendation.codec,
      videoBitRate: 8,
      videoEncoder: null,
      audio: recommendation.audio,
      audioSource: mode === "camera" ? (recommendation.audio ? "mic" : "off") : (recommendation.audio ? "output" : "off"),
      stayAwake: recommendation.stayAwake,
      turnScreenOff: recommendation.turnScreenOff,
      showTouches: recommendation.showTouches,
      fullscreen: false,
      captureOrientation: "auto",
      crop: null,
      cameraId: recommendedCamera?.id ?? null,
      cameraFacing: recommendedCamera && ["front", "back", "external"].includes(recommendedCamera.facing)
        ? recommendedCamera.facing as "front" | "back" | "external"
        : null,
      cameraZoom: recommendedCamera?.zoomMax && recommendedCamera.zoomMax > 1
        ? Math.max(1, recommendedCamera.zoomMin ?? 1)
        : null,
      cameraTorch: false,
      cameraSize: null,
      cameraAspectRatio: "auto",
      cameraHighSpeed: false,
      desktopWidth: desktopCapabilities?.recommendedWidth ?? (recommendation.maxSize >= 1920 ? 1920 : 1280),
      desktopHeight: desktopCapabilities?.recommendedHeight ?? (recommendation.maxSize >= 1920 ? 1080 : 720),
      desktopDensity: desktopCapabilities?.recommendedDensity ?? (recommendation.maxSize >= 1920 ? 240 : 200),
      desktopFlex: false,
      desktopNoDecorations: false,
      desktopKeepContent: false,
      desktopStartApp: null,
      desktopEnvironment: desktopCapabilities?.environmentKind ?? activeConfig.desktopEnvironment,
      desktopDisplayId: desktopCapabilities?.existingDisplayId ?? activeConfig.desktopDisplayId
    });
    setStatusText(`${mode === "camera" ? "Camera" : mode === "desktop" ? "Desktop" : "Mirror"} settings restored to the automatic profile.`);
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><MonitorSmartphone size={22} /></div>
          <strong>SCRCPY Studio</strong>
        </div>

        <nav className="mode-nav">
          {modeMeta.map(({ id, label, icon: Icon }) => (
            <button className={mode === id ? "mode-item active" : "mode-item"} onClick={() => selectMode(id)} key={id}>
              <Icon size={19} />
              <strong>{label}</strong>
            </button>
          ))}
        </nav>

        <div className="sidebar-bottom">
          {!runtimeHealthy && (
            <button
              className="primary wide"
              onClick={() => void installRuntime()}
              disabled={installingRuntime || Boolean(runtime?.scrcpyFound && !runtime?.adbFound)}
            >
              {installingRuntime ? <RefreshCw size={17} className="spin" /> : <Download size={17} />}
              {installingRuntime
                ? "Installing runtime…"
                : runtime?.scrcpyFound && !runtime?.adbFound
                  ? "Install ADB to continue"
                  : "Install official runtime"}
            </button>
          )}
          <button className="secondary wide" onClick={() => { setWirelessFeedback(null); setWirelessOpen(true); }}><Wifi size={17} /> Wireless setup</button>
          <div className="runtime-mini">
            <span className={runtimeHealthy ? "dot ok" : "dot warn"} />
            {runtimeHealthy ? "Runtime ready" : "Runtime needs attention"}
          </div>
        </div>
      </aside>

      <main className="main-content">
        <section className="device-bar panel">
          <div className="device-identity">
            <div className="device-icon"><MonitorSmartphone size={19} /><span className={selectedDevice?.state === "device" ? "online-dot" : "online-dot offline"} /></div>
            <div><strong>{deviceName}</strong><span>{deviceMeta}</span></div>
          </div>
          <div className="device-actions">
            <div className="device-select-wrap">
              <select value={selectedSerial} onChange={(e) => setSelectedSerial(e.target.value)} aria-label="Connected device">
                <option value="">Choose device</option>
                {devices.map((device) => <option key={device.serial} value={device.serial}>{device.model?.replaceAll("_", " ") || device.serial} — {device.state}</option>)}
              </select>
              <ChevronDown size={16} />
            </div>
            <button className="icon-button" onClick={() => void refresh()} disabled={busy} title="Refresh devices" aria-label="Refresh devices">
              <RefreshCw size={18} className={busy ? "spin" : ""} />
            </button>
          </div>
        </section>

        <div className="dashboard-grid">
          <section className="panel workspace-card">
            {recommendation ? (
              <div className="profile-bar">
                <span>Auto profile</span>
                <div className="pills">
                  {pill(recommendation.maxSize ? `${recommendation.maxSize}px` : "Native")}
                  {pill(`${recommendation.maxFps} FPS`)}
                  {pill(recommendation.codec.toUpperCase())}
                  {pill(recommendation.audio ? "Audio" : "Muted")}
                </div>
              </div>
            ) : (
              <div className="workspace-empty loading-placeholder" role="status" aria-live="polite">
                {deviceReady && modeLoading && <RefreshCw size={16} className="spin" />}
                <span>{workspacePlaceholder}</span>
              </div>
            )}

            {activeConfig && (
              <div className="capture-toolbar" aria-label="Capture tools">
                <button
                  className={activeConfig.record ? "secondary quick-action active" : "secondary quick-action"}
                  onClick={() => setConfig({ ...activeConfig, record: !activeConfig.record })}
                  disabled={launching}
                  title={`Record the ${mode === "camera" ? "camera stream" : mode === "desktop" ? "Desktop Mode display" : "phone display"} when the session starts`}
                >
                  <Clapperboard size={17} /> {activeConfig.record ? "Recording enabled" : recordingLabel}
                </button>
                <button
                  className="secondary quick-action"
                  onClick={() => void captureScreenshot()}
                  disabled={screenshotDisabled}
                  title={screenshotTitle}
                >
                  <Image size={16} /> {screenshotLabel}
                </button>
                <button className="secondary quick-action" onClick={() => void openMediaFolder()} disabled={creatorBusy} title="Open recordings and screenshots">
                  <FolderOpen size={16} /> Media Folder
                </button>
              </div>
            )}

            <div className="workspace-body">
              {mode === "camera" && activeConfig?.mode === "camera" && selectedSerial && (
                <CameraControls serial={selectedSerial} config={activeConfig} onChange={setConfig} onStatus={setStatusText} onCapabilitiesChange={setCameraCapabilities} />
              )}

              {mode === "desktop" && activeConfig?.mode === "desktop" && selectedSerial && (
                <DesktopControls serial={selectedSerial} config={activeConfig} onChange={setConfig} onStatus={setStatusText} onProbeStateChange={setDesktopProbe} lastLaunchResult={lastLaunchResult} />
              )}
            </div>

            <button className="primary launch" onClick={() => void launch()} disabled={!canLaunch || launching}>
              {launching ? <RefreshCw className="spin" size={20} /> : <Play size={20} fill="currentColor" />}
              {launching ? "Starting…" : mode === "creator" ? (activeConfig?.record ? "Start & Record" : "Mirror Phone") : mode === "camera" ? "Open Camera" : desktopLaunchLabel}
            </button>
          </section>

          <section className="panel settings-card">
            {activeConfig ? (
              <AdvancedSettings
                mode={mode}
                config={activeConfig}
                profile={profile}
                cameraCapabilities={cameraCapabilities}
                desktopCapabilities={desktopProbe.capabilities}
                onChange={setConfig}
                onReset={resetToAuto}
              />
            ) : (
              <div className="settings-empty loading-placeholder">
                {deviceReady && modeLoading && <RefreshCw size={16} className="spin" />}
                <span>{settingsPlaceholder}</span>
              </div>
            )}
          </section>
        </div>

        <footer className="statusbar">
          <span className="pulse-dot" /> {statusText}
          {runtime?.scrcpyVersion && <span className="version">{shortVersion(runtime.scrcpyVersion)}</span>}
        </footer>
      </main>

      {wirelessOpen && (
        <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && setWirelessOpen(false)}>
          <div className="modal wireless-modal">
            <ModalHeader title="Wireless setup" close={() => setWirelessOpen(false)} />

            {wirelessFeedback && (
              <div className={`finding ${wirelessFeedback.kind === "success" ? "ok" : "error"}`} style={{ marginBottom: 12 }}>
                {wirelessFeedback.kind === "success" ? <CheckCircle2 size={18} /> : <CircleAlert size={18} />}
                <div><strong>{wirelessFeedback.kind === "success" ? "Connection updated" : "Connection needs attention"}</strong><span>{wirelessFeedback.text}</span></div>
              </div>
            )}

            {selectedDevice?.state === "device" && selectedDevice.connectionKind === "usb" && (
              <div className="wireless-block wireless-quick">
                <div className="wireless-block-copy">
                  <h3><Usb size={17} /> Use this phone wirelessly</h3>
                  <span>Keep the phone and PC on the same Wi-Fi.</span>
                </div>
                <button className="primary compact" onClick={() => void enableUsbWireless()} disabled={wirelessBusy}>
                  {wirelessBusy ? <RefreshCw size={16} className="spin" /> : <Wifi size={16} />} Use Wireless
                </button>
              </div>
            )}

            {selectedDevice?.state === "device" && selectedDevice.connectionKind === "wireless" && (
              <div className="wireless-block wireless-quick">
                <div className="wireless-block-copy">
                  <h3><Wifi size={17} /> Connected wirelessly</h3>
                  <span>Connect the USB cable first to switch back.</span>
                </div>
                <button className="secondary" onClick={() => void useUsbInstead()} disabled={wirelessBusy}>
                  {wirelessBusy ? <RefreshCw size={16} className="spin" /> : <Usb size={16} />} Use USB Instead
                </button>
              </div>
            )}

            {rememberedWireless.length > 0 && (
              <div className="wireless-block wireless-saved">
                <h3><RefreshCw size={17} /> Saved phones</h3>
                <div className="wireless-device-list">
                  {rememberedWireless.map((item) => (
                    <div className="wireless-device-row" key={item.address}>
                      <div className="wireless-device-name">
                        <strong>{item.label}</strong>
                        <span>{item.address} · {item.connected ? "Connected" : "Saved"}</span>
                      </div>
                      <button className="secondary" onClick={() => void reconnectWireless(item.address)} disabled={wirelessBusy || item.connected}>{item.connected ? "Connected" : "Reconnect"}</button>
                      <button className="secondary" onClick={() => void forgetWireless(item.address)} disabled={wirelessBusy}>{item.connected ? "Disconnect & Forget" : "Forget"}</button>
                    </div>
                  ))}
                </div>
              </div>
            )}

            <details className="wireless-block wireless-manual">
              <summary>
                <span><Radio size={17} /><strong>Manual setup</strong></span>
                <small>New phones and connection recovery</small>
                <ChevronDown size={15} />
              </summary>
              <div className="wireless-method-grid">
                <section className="wireless-method">
                  <div className="wireless-method-title"><span>1</span><strong>Pair</strong></div>
                  <small>Wireless debugging → Pair with code</small>
                  <div className="row"><input aria-label="Pairing address" placeholder="Pairing IP:port" value={pairAddress} onChange={(e) => setPairAddress(e.target.value)} /><input aria-label="Pairing code" className="code-input" placeholder="Code" value={pairCode} onChange={(e) => setPairCode(e.target.value)} /><button className="secondary" onClick={() => void pair()} disabled={wirelessBusy || !pairAddress || !pairCode}>Pair</button></div>
                </section>
                <section className="wireless-method">
                  <div className="wireless-method-title"><span>2</span><strong>Connect</strong></div>
                  <small>Use the main Wireless debugging address</small>
                  <div className="row"><input aria-label="Connection address" placeholder="Connection IP:port" value={connectAddress} onChange={(e) => setConnectAddress(e.target.value)} /><button className="primary compact" onClick={() => void connect()} disabled={wirelessBusy || !connectAddress}>{wirelessBusy ? "Working…" : "Connect"}</button></div>
                </section>
              </div>
            </details>
          </div>
        </div>
      )}
    </div>
  );
}

function ModalHeader({ title, close }: { title: string; close: () => void }) {
  return <div className="modal-header"><h2>{title}</h2><button className="icon-button" onClick={close} aria-label="Close"><X size={19} /></button></div>;
}

export default App;
