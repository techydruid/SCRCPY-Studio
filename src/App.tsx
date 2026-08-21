import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  Cable,
  Camera,
  CheckCircle2,
  ChevronDown,
  CircleAlert,
  Clapperboard,
  Download,
  FolderOpen,
  Image,
  Monitor,
  Gauge,
  HeartPulse,
  MonitorSmartphone,
  Play,
  Radio,
  RefreshCw,
  Settings2,
  Smartphone,
  Sparkles,
  Usb,
  Wifi,
  X
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import type {
  DeviceInfo,
  DeviceProfile,
  DoctorFinding,
  LaunchConfig,
  LaunchResult,
  Recommendation,
  RuntimeStatus,
  SessionMode
} from "./types";

const modeMeta: Array<{
  id: SessionMode;
  label: string;
  sub: string;
  icon: typeof Smartphone;
}> = [
  { id: "mirror", label: "Mirror Phone", sub: "Fast everyday control", icon: Smartphone },
  { id: "creator", label: "Creator Mode", sub: "Tutorial-ready quality", icon: Clapperboard },
  { id: "camera", label: "Camera Mode", sub: "Use phone cameras", icon: Camera },
  { id: "desktop", label: "Desktop Mode", sub: "Virtual Android display", icon: Monitor }
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
  const [mode, setMode] = useState<SessionMode>("mirror");
  const [recommendation, setRecommendation] = useState<Recommendation | null>(null);
  const [config, setConfig] = useState<LaunchConfig | null>(null);
  const [doctor, setDoctor] = useState<DoctorFinding[]>([]);
  const [busy, setBusy] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [installingRuntime, setInstallingRuntime] = useState(false);
  const [creatorBusy, setCreatorBusy] = useState(false);
  const [statusText, setStatusText] = useState("Checking your setup…");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [wirelessOpen, setWirelessOpen] = useState(false);
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

      const [deviceResult, findings] = await Promise.all([
        runtimeResult.adbFound
          ? invoke<DeviceInfo[]>("list_devices").catch(() => [] as DeviceInfo[])
          : Promise.resolve([] as DeviceInfo[]),
        invoke<DoctorFinding[]>("run_doctor").catch(() => [] as DoctorFinding[])
      ]);
      setDevices(deviceResult);
      setDoctor(findings);

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

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!selectedSerial || selectedDevice?.state !== "device") {
      setProfile(null);
      setRecommendation(null);
      setConfig(null);
      return;
    }

    let cancelled = false;
    const load = async () => {
      try {
        const [p, r] = await Promise.all([
          invoke<DeviceProfile>("inspect_device", { serial: selectedSerial }),
          invoke<Recommendation>("recommend_settings", { serial: selectedSerial, mode })
        ]);
        if (cancelled) return;
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
          fullscreen: false
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
    if (!config) return;
    setLaunching(true);
    setStatusText("Starting a smart session…");
    try {
      const result = await invoke<LaunchResult>("launch_session", { config });
      setStatusText(result.recordingPath ? `${result.message} Recording: ${result.recordingPath}` : result.message);
    } catch (error) {
      setStatusText(`Launch failed: ${String(error)}`);
      const findings = await invoke<DoctorFinding[]>("run_doctor").catch(() => []);
      setDoctor(findings);
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

  const pair = async () => {
    setBusy(true);
    try {
      setStatusText(await invoke<string>("pair_device", { address: pairAddress, code: pairCode }));
      await refresh();
    } catch (error) {
      setStatusText(String(error));
    } finally {
      setBusy(false);
    }
  };

  const connect = async () => {
    setBusy(true);
    try {
      setStatusText(await invoke<string>("connect_device", { address: connectAddress }));
      await refresh();
    } catch (error) {
      setStatusText(String(error));
    } finally {
      setBusy(false);
    }
  };

  const runtimeHealthy = Boolean(runtime?.adbFound && runtime?.scrcpyFound);
  const canLaunch = Boolean(config && selectedDevice?.state === "device" && runtimeHealthy);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><MonitorSmartphone size={22} /></div>
          <div>
            <strong>SCRCPY Studio</strong>
            <span>Smart scrcpy frontend</span>
          </div>
        </div>

        <div className="side-section-label">MODES</div>
        <nav className="mode-nav">
          {modeMeta.map(({ id, label, sub, icon: Icon }) => (
            <button className={mode === id ? "mode-item active" : "mode-item"} onClick={() => setMode(id)} key={id}>
              <Icon size={19} />
              <span><strong>{label}</strong><small>{sub}</small></span>
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
          <button className="secondary wide" onClick={() => setWirelessOpen(true)}><Wifi size={17} /> Wireless setup</button>
          <button className="secondary wide" onClick={() => setAdvancedOpen(true)}><Settings2 size={17} /> Advanced settings</button>
          <div className="runtime-mini">
            <span className={runtimeHealthy ? "dot ok" : "dot warn"} />
            {runtimeHealthy ? "Runtime ready" : "Runtime needs attention"}
          </div>
        </div>
      </aside>

      <main className="main-content">
        <header className="topbar">
          <div>
            <h1>{modeMeta.find((m) => m.id === mode)?.label}</h1>
            <p>Pick a device. SCRCPY Studio chooses sensible settings for you.</p>
          </div>
          <button className="icon-button" onClick={() => void refresh()} disabled={busy} title="Refresh devices">
            <RefreshCw size={19} className={busy ? "spin" : ""} />
          </button>
        </header>

        <section className="device-strip panel">
          <div className="section-heading">
            <div><span className="eyebrow">CONNECTED DEVICE</span><h2>{selectedDevice?.model?.replaceAll("_", " ") || profile?.model || "No device selected"}</h2></div>
            <div className="device-select-wrap">
              <select value={selectedSerial} onChange={(e) => setSelectedSerial(e.target.value)}>
                <option value="">Choose device</option>
                {devices.map((device) => <option key={device.serial} value={device.serial}>{device.model?.replaceAll("_", " ") || device.serial} — {device.state}</option>)}
              </select>
              <ChevronDown size={16} />
            </div>
          </div>

          {profile && selectedDevice?.state === "device" ? (
            <div className="device-facts">
              <div className="phone-visual"><Smartphone size={51} /><span className="online-dot" /></div>
              <div className="facts-grid">
                <Fact label="Connection" value={profile.connectionKind === "usb" ? "USB" : "Wireless"} icon={profile.connectionKind === "usb" ? Usb : Wifi} />
                <Fact label="Android" value={`${profile.androidVersion} · API ${profile.sdk}`} icon={Activity} />
                <Fact label="Display" value={profile.width && profile.height ? `${profile.width}×${profile.height}` : "Detected by scrcpy"} icon={MonitorSmartphone} />
                <Fact label="Encoder" value={profile.h265Available ? "H.264 + H.265" : "H.264"} icon={Gauge} />
              </div>
            </div>
          ) : (
            <div className="empty-state">
              <Cable size={30} />
              <div><strong>{runtimeHealthy ? "Connect an Android phone" : "Install the runtime first"}</strong><span>{runtimeHealthy ? "Enable USB debugging, connect a data cable, and approve the debugging prompt." : "SCRCPY Studio can download the official Windows scrcpy package, verify its SHA-256 checksum, and configure it automatically."}</span></div>
            </div>
          )}
        </section>

        <div className="dashboard-grid">
          <section className="panel smart-card">
            <div className="smart-title">
              <div className="spark"><Sparkles size={20} /></div>
              <div><span className="eyebrow">SMART AUTO-TUNE</span><h2>{recommendation?.qualityLabel || "Waiting for device"}</h2></div>
            </div>

            {recommendation ? (
              <>
                <div className="pills">
                  {pill(recommendation.maxSize ? `Max dimension ${recommendation.maxSize}` : "Native size")}
                  {pill(`${recommendation.maxFps} FPS`)}
                  {pill(recommendation.codec.toUpperCase())}
                  {pill(recommendation.audio ? "Audio on" : "Audio off")}
                </div>
                <ul className="rationale">
                  {recommendation.rationale.map((reason) => <li key={reason}><CheckCircle2 size={15} />{reason}</li>)}
                </ul>
                <div className="smart-note"><HeartPulse size={17} /><span>If this profile fails immediately, SCRCPY Studio automatically retries safer codec, resolution and FPS combinations.</span></div>
              </>
            ) : <p className="muted">Connect an authorized device to generate a recommendation.</p>}

            {mode === "creator" && config && (
              <div className="creator-tools">
                <div className="creator-tools-heading">
                  <div><span className="eyebrow">CREATOR SHORTCUTS</span><strong>Capture tools</strong></div>
                  <span>{config.record ? "Recording enabled" : "Ready"}</span>
                </div>
                <div className="creator-actions">
                  <button className={config.record ? "secondary creator-action active" : "secondary creator-action"} onClick={() => setConfig({ ...config, record: !config.record })}>
                    <Clapperboard size={16} /> {config.record ? "Record on start" : "Enable recording"}
                  </button>
                  <button className="secondary creator-action" onClick={() => void captureScreenshot()} disabled={!canLaunch || creatorBusy}>
                    <Image size={16} /> Screenshot
                  </button>
                  <button className="secondary creator-action" onClick={() => void openMediaFolder()} disabled={creatorBusy}>
                    <FolderOpen size={16} /> Media Folder
                  </button>
                </div>
              </div>
            )}

            <button className="primary launch" onClick={() => void launch()} disabled={!canLaunch || launching}>
              {launching ? <RefreshCw className="spin" size={20} /> : <Play size={20} fill="currentColor" />}
              {launching ? "Starting…" : mode === "creator" ? (config?.record ? "Start & Record Creator Session" : "Start Creator Session") : mode === "camera" ? "Open Camera" : mode === "desktop" ? "Launch Desktop" : "Mirror Phone"}
            </button>
          </section>

          <section className="panel doctor-card">
            <div className="doctor-heading"><div><span className="eyebrow">CONNECTION DOCTOR</span><h2>Setup health</h2></div><HeartPulse size={21} /></div>
            <div className="doctor-list">
              {doctor.slice(0, 4).map((item, idx) => (
                <div className={`finding ${item.level}`} key={`${item.title}-${idx}`}>
                  {item.level === "ok" ? <CheckCircle2 size={18} /> : <CircleAlert size={18} />}
                  <div><strong>{item.title}</strong><span>{item.detail}</span>{item.action && <small>{item.action}</small>}</div>
                </div>
              ))}
              {!doctor.length && <div className="finding info"><Activity size={18} /><div><strong>Running checks</strong><span>SCRCPY Studio is inspecting ADB and scrcpy.</span></div></div>}
            </div>
          </section>
        </div>

        <footer className="statusbar">
          <span className="pulse-dot" /> {statusText}
          {runtime?.scrcpyVersion && <span className="version">{shortVersion(runtime.scrcpyVersion)}</span>}
        </footer>
      </main>

      {advancedOpen && config && (
        <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && setAdvancedOpen(false)}>
          <div className="modal">
            <ModalHeader title="Advanced settings" subtitle="Useful controls only. Smart defaults stay available." close={() => setAdvancedOpen(false)} />
            <div className="form-grid">
              <Field label="Max dimension"><select value={config.maxSize} onChange={(e) => setConfig({ ...config, maxSize: Number(e.target.value) })}><option value={0}>Native</option><option value={1280}>1280 px</option><option value={1920}>1920 px</option><option value={2560}>2560 px</option></select></Field>
              <Field label="Frame rate"><select value={config.maxFps} onChange={(e) => setConfig({ ...config, maxFps: Number(e.target.value) })}><option value={30}>30 FPS</option><option value={60}>60 FPS</option><option value={90}>90 FPS</option><option value={120}>120 FPS</option></select></Field>
              <Field label="Video codec"><select value={config.codec} onChange={(e) => setConfig({ ...config, codec: e.target.value as "h264" | "h265" })}><option value="h264">H.264 — safest</option><option value="h265">H.265 — efficient</option></select></Field>
            </div>
            <div className="toggle-list">
              <Toggle label="Forward audio" checked={config.audio} onChange={(v) => setConfig({ ...config, audio: v })} />
              <Toggle label="Keep phone awake" checked={config.stayAwake} onChange={(v) => setConfig({ ...config, stayAwake: v })} />
              <Toggle label="Turn phone screen off" checked={config.turnScreenOff} onChange={(v) => setConfig({ ...config, turnScreenOff: v })} />
              <Toggle label="Show touches" checked={config.showTouches} onChange={(v) => setConfig({ ...config, showTouches: v })} />
              <Toggle label="Record session" checked={config.record} onChange={(v) => setConfig({ ...config, record: v })} />
              <Toggle label="Start fullscreen" checked={config.fullscreen} onChange={(v) => setConfig({ ...config, fullscreen: v })} />
            </div>
            <div className="modal-actions"><button className="secondary" onClick={() => setAdvancedOpen(false)}>Done</button></div>
          </div>
        </div>
      )}

      {wirelessOpen && (
        <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && setWirelessOpen(false)}>
          <div className="modal wireless-modal">
            <ModalHeader title="Wireless setup" subtitle="Android 11+ pairing or a direct ADB address." close={() => setWirelessOpen(false)} />
            <div className="wireless-block"><h3><Radio size={18} /> Pair a new phone</h3><p>On Android, open Developer options → Wireless debugging → Pair device with pairing code.</p><div className="row"><input placeholder="192.168.1.20:37123" value={pairAddress} onChange={(e) => setPairAddress(e.target.value)} /><input className="code-input" placeholder="123456" value={pairCode} onChange={(e) => setPairCode(e.target.value)} /><button className="secondary" onClick={() => void pair()} disabled={!pairAddress || !pairCode}>Pair</button></div></div>
            <div className="wireless-block"><h3><Wifi size={18} /> Connect</h3><p>Enter the separate IP:port shown on the main Wireless debugging page.</p><div className="row"><input placeholder="192.168.1.20:41277" value={connectAddress} onChange={(e) => setConnectAddress(e.target.value)} /><button className="primary compact" onClick={() => void connect()} disabled={!connectAddress}>Connect</button></div></div>
          </div>
        </div>
      )}
    </div>
  );
}

function Fact({ label, value, icon: Icon }: { label: string; value: string; icon: typeof Smartphone }) {
  return <div className="fact"><Icon size={17} /><div><span>{label}</span><strong>{value}</strong></div></div>;
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
