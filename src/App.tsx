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
  Settings2,
  Usb,
  Wifi,
  X
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import CameraControls from "./CameraControls";
import DesktopControls from "./DesktopControls";
import type {
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

function App() {
  const [runtime, setRuntime] = useState<RuntimeStatus | null>(null);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedSerial, setSelectedSerial] = useState<string>("");
  const [profile, setProfile] = useState<DeviceProfile | null>(null);
  const [mode, setMode] = useState<SessionMode>("creator");
  const [recommendation, setRecommendation] = useState<Recommendation | null>(null);
  const [config, setConfig] = useState<LaunchConfig | null>(null);
  const [busy, setBusy] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [lastLaunchResult, setLastLaunchResult] = useState<LaunchResult | null>(null);
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

      if (!runtimeResult.adbFound || !runtimeResult.scrcpyFound) {
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
    setRecommendation(null);
    setConfig(null);
    setLastLaunchResult(null);
  };

  useEffect(() => {
    if (!selectedSerial || selectedDevice?.state !== "device") {
      setProfile(null);
      setRecommendation(null);
      setConfig(null);
      setDesktopProbe({ serial: selectedSerial, checking: false, capabilities: null, error: null });
      return;
    }

    let cancelled = false;
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
          audio: r.audio,
          stayAwake: r.stayAwake,
          turnScreenOff: r.turnScreenOff,
          showTouches: r.showTouches,
          record: false,
          fullscreen: false,
          cameraId: null,
          cameraFacing: null,
          cameraZoom: null,
          cameraTorch: false,
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
        if (!cancelled) setStatusText(`Could not inspect device: ${String(error)}`);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [selectedSerial, selectedDevice?.state, mode]);

  const installRuntime = async () => {
    setInstallingRuntime(true);
    setStatusText("Downloading and verifying the latest official scrcpy Windows runtime…");
    try {
      const installed = await invoke<RuntimeStatus>("install_official_runtime");
      setRuntime(installed);
      setStatusText(installed.scrcpyVersion ? `Runtime installed — ${shortVersion(installed.scrcpyVersion)}` : "Official runtime installed successfully");
      await refresh();
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
    setCreatorBusy(true);
    setStatusText("Capturing Android screenshot…");
    try {
      const path = await invoke<string>("capture_screenshot", { serial: selectedSerial });
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
            <button className="primary wide" onClick={() => void installRuntime()} disabled={installingRuntime}>
              {installingRuntime ? <RefreshCw size={17} className="spin" /> : <Download size={17} />}
              {installingRuntime ? "Installing runtime…" : "Install official runtime"}
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
        <header className="topbar">
          <h1>{modeMeta.find((m) => m.id === mode)?.label}</h1>
          <button className="icon-button" onClick={() => void refresh()} disabled={busy} title="Refresh devices">
            <RefreshCw size={19} className={busy ? "spin" : ""} />
          </button>
        </header>

        <section className="device-bar panel">
          <div className="device-identity">
            <div className="device-icon"><MonitorSmartphone size={19} /><span className={selectedDevice?.state === "device" ? "online-dot" : "online-dot offline"} /></div>
            <div><strong>{deviceName}</strong><span>{deviceMeta}</span></div>
          </div>
          <div className="device-select-wrap">
            <select value={selectedSerial} onChange={(e) => setSelectedSerial(e.target.value)}>
              <option value="">Choose device</option>
              {devices.map((device) => <option key={device.serial} value={device.serial}>{device.model?.replaceAll("_", " ") || device.serial} — {device.state}</option>)}
            </select>
            <ChevronDown size={16} />
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
            ) : <div className="workspace-empty">Connect a phone to begin</div>}

            <div className="workspace-body">
              {mode === "creator" && activeConfig?.mode === "creator" && (
                <div className="quick-actions">
                  <button className={activeConfig.record ? "secondary quick-action active" : "secondary quick-action"} onClick={() => setConfig({ ...activeConfig, record: !activeConfig.record })}>
                    <Clapperboard size={17} /> {activeConfig.record ? "Recording enabled" : "Screen recording"}
                  </button>
                  <button className="secondary quick-action" onClick={() => void captureScreenshot()} disabled={!canLaunch || creatorBusy}>
                    <Image size={16} /> Screenshot
                  </button>
                  <button className="secondary quick-action" onClick={() => void openMediaFolder()} disabled={creatorBusy}>
                    <FolderOpen size={16} /> Media Folder
                  </button>
                </div>
              )}

              {mode === "camera" && activeConfig?.mode === "camera" && selectedSerial && (
                <CameraControls serial={selectedSerial} config={activeConfig} onChange={setConfig} onStatus={setStatusText} />
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
            <div className="settings-heading"><Settings2 size={18} /><h2>Advanced Settings</h2></div>
            {activeConfig ? (
              <>
                <div className="settings-selects">
                  <Field label="Resolution"><select value={activeConfig.maxSize} onChange={(e) => setConfig({ ...activeConfig, maxSize: Number(e.target.value) })}><option value={0}>Native</option><option value={1280}>1280 px</option><option value={1920}>1920 px</option><option value={2560}>2560 px</option></select></Field>
                  <Field label="Frame rate"><select value={activeConfig.maxFps} onChange={(e) => setConfig({ ...activeConfig, maxFps: Number(e.target.value) })}><option value={30}>30 FPS</option><option value={60}>60 FPS</option><option value={90}>90 FPS</option><option value={120}>120 FPS</option></select></Field>
                  <Field label="Codec"><select value={activeConfig.codec} onChange={(e) => setConfig({ ...activeConfig, codec: e.target.value as "h264" | "h265" })}><option value="h264">H.264</option><option value="h265">H.265</option></select></Field>
                </div>
                <div className="toggle-list settings-toggles">
                  <Toggle label="Forward audio" checked={activeConfig.audio} onChange={(v) => setConfig({ ...activeConfig, audio: v })} />
                  <Toggle label="Keep phone awake" checked={activeConfig.stayAwake} onChange={(v) => setConfig({ ...activeConfig, stayAwake: v })} />
                  <Toggle label="Turn screen off" checked={activeConfig.turnScreenOff} onChange={(v) => setConfig({ ...activeConfig, turnScreenOff: v })} />
                  <Toggle label="Show touches" checked={activeConfig.showTouches} onChange={(v) => setConfig({ ...activeConfig, showTouches: v })} />
                  <Toggle label="Record session" checked={activeConfig.record} onChange={(v) => setConfig({ ...activeConfig, record: v })} />
                  <Toggle label="Start fullscreen" checked={activeConfig.fullscreen} onChange={(v) => setConfig({ ...activeConfig, fullscreen: v })} />
                </div>
              </>
            ) : <div className="settings-empty">Waiting for device</div>}
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
            <ModalHeader title="Wireless setup" subtitle="Simple controls on top, advanced transport handling underneath." close={() => setWirelessOpen(false)} />

            {wirelessFeedback && (
              <div className={`finding ${wirelessFeedback.kind === "success" ? "ok" : "error"}`} style={{ marginBottom: 12 }}>
                {wirelessFeedback.kind === "success" ? <CheckCircle2 size={18} /> : <CircleAlert size={18} />}
                <div><strong>{wirelessFeedback.kind === "success" ? "Connection updated" : "Connection needs attention"}</strong><span>{wirelessFeedback.text}</span></div>
              </div>
            )}

            {selectedDevice?.state === "device" && selectedDevice.connectionKind === "usb" && (
              <div className="wireless-block">
                <h3><Usb size={18} /> Switch this USB phone to wireless</h3>
                <p>Keep the phone and PC on the same Wi-Fi. SCRCPY Studio will detect the phone, enable wireless ADB, verify the connection, select Wi-Fi as the active transport, and remember it.</p>
                <button className="primary compact" onClick={() => void enableUsbWireless()} disabled={wirelessBusy}>
                  {wirelessBusy ? <RefreshCw size={16} className="spin" /> : <Wifi size={16} />} Use Wireless Now
                </button>
              </div>
            )}

            {selectedDevice?.state === "device" && selectedDevice.connectionKind === "wireless" && (
              <div className="wireless-block">
                <h3><Wifi size={18} /> Currently using wireless</h3>
                <p>Wireless is the active connection. To return to USB, connect this phone with a USB data cable, approve debugging if asked, then click below. SCRCPY Studio verifies it is the same phone before disconnecting Wi-Fi.</p>
                <button className="secondary" onClick={() => void useUsbInstead()} disabled={wirelessBusy}>
                  {wirelessBusy ? <RefreshCw size={16} className="spin" /> : <Usb size={16} />} Use USB Instead
                </button>
              </div>
            )}

            <div className="wireless-block">
              <h3><RefreshCw size={18} /> Remembered phones</h3>
              <p>Successful wireless connections are saved locally. Reconnect without entering the IP address again.</p>
              {rememberedWireless.length ? rememberedWireless.map((item) => (
                <div className="row" key={item.address} style={{ alignItems: "center", marginTop: 8 }}>
                  <div style={{ flex: 1, minWidth: 0, display: "grid", gap: 2 }}>
                    <strong style={{ fontSize: 12, overflow: "hidden", textOverflow: "ellipsis" }}>{item.label}</strong>
                    <span style={{ fontSize: 10, color: "#71809a" }}>{item.address} · {item.connected ? "Connected" : "Saved"}</span>
                  </div>
                  <button className="secondary" onClick={() => void reconnectWireless(item.address)} disabled={wirelessBusy || item.connected}>{item.connected ? "Connected" : "Reconnect"}</button>
                  <button className="secondary" onClick={() => void forgetWireless(item.address)} disabled={wirelessBusy}>{item.connected ? "Disconnect & Forget" : "Forget"}</button>
                </div>
              )) : <p className="muted">No remembered wireless phones yet.</p>}
            </div>

            <div className="wireless-block">
              <h3><Radio size={18} /> Pair a new Android 11+ phone</h3>
              <p>On Android, open Developer options → Wireless debugging → Pair device with pairing code. Use the temporary pairing IP:port here.</p>
              <div className="row"><input placeholder="192.168.1.20:37123" value={pairAddress} onChange={(e) => setPairAddress(e.target.value)} /><input className="code-input" placeholder="123456" value={pairCode} onChange={(e) => setPairCode(e.target.value)} /><button className="secondary" onClick={() => void pair()} disabled={wirelessBusy || !pairAddress || !pairCode}>Pair</button></div>
            </div>

            <div className="wireless-block">
              <h3><Wifi size={18} /> Connect with an address</h3>
              <p>After pairing, enter the separate IP:port shown on the main Wireless debugging page. Successful connections are remembered automatically.</p>
              <div className="row"><input placeholder="192.168.1.20:41277" value={connectAddress} onChange={(e) => setConnectAddress(e.target.value)} /><button className="primary compact" onClick={() => void connect()} disabled={wirelessBusy || !connectAddress}>{wirelessBusy ? "Working…" : "Connect"}</button></div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function ModalHeader({ title, subtitle, close }: { title: string; subtitle: string; close: () => void }) {
  return <div className="modal-header"><div><h2>{title}</h2><p>{subtitle}</p></div><button className="icon-button" onClick={close}><X size={19} /></button></div>;
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return <label className="field"><span>{label}</span>{children}</label>;
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="toggle-row"><span>{label}</span><input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} /><i /></label>;
}

export default App;

