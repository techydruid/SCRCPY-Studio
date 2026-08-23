import { ChevronDown, RotateCcw, Settings2 } from "lucide-react";
import type {
  CameraCapabilities,
  DesktopCapabilities,
  DeviceProfile,
  LaunchConfig,
  SessionMode
} from "./types";

type Props = {
  mode: SessionMode;
  config: LaunchConfig;
  profile: DeviceProfile | null;
  cameraCapabilities: CameraCapabilities | null;
  desktopCapabilities: DesktopCapabilities | null;
  onChange: (next: LaunchConfig) => void;
  onReset: () => void;
};

const desktopLayouts = [
  { label: "1280 × 720", value: "1280x720", width: 1280, height: 720 },
  { label: "1920 × 1080", value: "1920x1080", width: 1920, height: 1080 },
  { label: "2560 × 1440", value: "2560x1440", width: 2560, height: 1440 }
];

const bitrateOptions = [4, 8, 12, 16, 24, 32, 50];

function Field({ label, children, className = "" }: { label: string; children: React.ReactNode; className?: string }) {
  return <label className={`field ${className}`.trim()}><span>{label}</span>{children}</label>;
}

function Toggle({ label, checked, onChange, disabled = false }: { label: string; checked: boolean; onChange: (value: boolean) => void; disabled?: boolean }) {
  return (
    <label className={`toggle-row${disabled ? " disabled" : ""}`}>
      <span>{label}</span>
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} disabled={disabled} />
      <i />
    </label>
  );
}

function maxDensity(height: number) {
  return Math.floor((height * 160) / 600);
}

function densityOptions(height: number) {
  return [160, 180, 200, 240, 284, 320].filter((density) => density <= maxDensity(height));
}

function densityForLayout(height: number, preferred: number) {
  const options = densityOptions(height);
  return options.filter((density) => density <= preferred).at(-1) ?? options[0] ?? 160;
}

export default function AdvancedSettings({
  mode,
  config,
  profile,
  cameraCapabilities,
  desktopCapabilities,
  onChange,
  onReset
}: Props) {
  const update = (patch: Partial<LaunchConfig>) => onChange({ ...config, ...patch });
  const title = mode === "camera" ? "Camera Settings" : mode === "desktop" ? "Desktop Settings" : "Mirror Settings";
  const selectedCamera = cameraCapabilities?.cameras.find((camera) => camera.id === config.cameraId)
    ?? cameraCapabilities?.cameras[0]
    ?? null;
  const highSpeedModes = selectedCamera?.highSpeedModes ?? [];
  const selectedHighSpeedMode = highSpeedModes.find((item) => item.size === config.cameraSize) ?? highSpeedModes[0] ?? null;
  const regularCameraFps = [...new Set((selectedCamera?.fps ?? [30]).filter((fps) => fps > 0 && fps <= 60))].sort((a, b) => a - b);
  const cameraFps = config.cameraHighSpeed
    ? selectedHighSpeedMode?.fps ?? [120]
    : regularCameraFps.length ? regularCameraFps : [30];
  const codecOptions: Array<"h264" | "h265" | "av1"> = [
    "h264",
    ...(profile?.h265Available ? ["h265" as const] : []),
    ...(profile?.av1Available ? ["av1" as const] : [])
  ];
  const encoders = profile?.videoEncoders.filter((encoder) => encoder.codec === config.codec) ?? [];
  const desktopIsDex = desktopCapabilities?.environmentKind === "samsung_dex";
  const desktopHeight = config.desktopHeight ?? desktopCapabilities?.recommendedHeight ?? 1080;
  const densities = densityOptions(desktopHeight);

  const changeCodec = (codec: "h264" | "h265" | "av1") => {
    const encoderMatches = profile?.videoEncoders.some((encoder) => encoder.codec === codec && encoder.name === config.videoEncoder);
    update({ codec, videoEncoder: encoderMatches ? config.videoEncoder : null });
  };

  const setDesktopLayout = (value: string) => {
    const layout = desktopLayouts.find((item) => item.value === value);
    if (!layout) return;
    update({
      desktopWidth: layout.width,
      desktopHeight: layout.height,
      desktopDensity: densityForLayout(layout.height, desktopCapabilities?.recommendedDensity ?? 240)
    });
  };

  const setHighSpeed = (enabled: boolean) => {
    if (!enabled) {
      update({ cameraHighSpeed: false, cameraSize: null, maxFps: 30 });
      return;
    }
    const first = highSpeedModes[0];
    if (!first) return;
    update({ cameraHighSpeed: true, cameraSize: first.size, cameraAspectRatio: "auto", maxFps: first.fps[0] ?? 120 });
  };

  const qualityFields = mode === "desktop" ? (
    <>
      {!desktopIsDex && (
        <>
          <Field label="Display size" className="settings-span-two">
            <select value={`${config.desktopWidth ?? 1920}x${config.desktopHeight ?? 1080}`} onChange={(event) => setDesktopLayout(event.target.value)}>
              {desktopLayouts.map((layout) => <option key={layout.value} value={layout.value}>{layout.label}</option>)}
            </select>
          </Field>
          <Field label="Density">
            <select value={config.desktopDensity ?? 240} onChange={(event) => update({ desktopDensity: Number(event.target.value) })}>
              {densities.map((density) => <option key={density} value={density}>{density} dpi</option>)}
            </select>
          </Field>
        </>
      )}
      <Field label="Frame rate">
        <select value={config.maxFps} onChange={(event) => update({ maxFps: Number(event.target.value) })}>
          <option value={30}>30 FPS</option><option value={60}>60 FPS</option><option value={90}>90 FPS</option><option value={120}>120 FPS</option>
        </select>
      </Field>
      <Field label="Codec">
        <select value={config.codec} onChange={(event) => changeCodec(event.target.value as "h264" | "h265" | "av1")}>
          {codecOptions.map((codec) => <option key={codec} value={codec}>{codec.toUpperCase()}</option>)}
        </select>
      </Field>
      <Bitrate config={config} update={update} />
    </>
  ) : mode === "camera" ? (
    <>
      {config.cameraHighSpeed ? (
        <Field label="Camera size">
          <select value={config.cameraSize ?? selectedHighSpeedMode?.size ?? ""} onChange={(event) => {
            const selected = highSpeedModes.find((item) => item.size === event.target.value);
            update({ cameraSize: event.target.value, maxFps: selected?.fps[0] ?? 120 });
          }}>
            {highSpeedModes.map((item) => <option key={item.size} value={item.size}>{item.size}</option>)}
          </select>
        </Field>
      ) : (
        <Resolution config={config} update={update} label="Max size" />
      )}
      <Field label="Camera FPS">
        <select value={config.maxFps} onChange={(event) => update({ maxFps: Number(event.target.value) })}>
          {cameraFps.map((fps) => <option key={fps} value={fps}>{fps} FPS</option>)}
        </select>
      </Field>
      <Codec config={config} options={codecOptions} change={changeCodec} />
      <Bitrate config={config} update={update} />
    </>
  ) : (
    <>
      <Resolution config={config} update={update} />
      <Field label="Frame rate">
        <select value={config.maxFps} onChange={(event) => update({ maxFps: Number(event.target.value) })}>
          <option value={30}>30 FPS</option><option value={60}>60 FPS</option><option value={90}>90 FPS</option><option value={120}>120 FPS</option>
        </select>
      </Field>
      <Codec config={config} options={codecOptions} change={changeCodec} />
      <Bitrate config={config} update={update} />
    </>
  );

  return (
    <>
      <div className="settings-heading"><Settings2 size={18} /><h2>{title}</h2></div>
      <span className="settings-section-label">Quality</span>
      <div className="settings-selects mode-settings-selects">{qualityFields}</div>

      <span className="settings-section-label behavior-label">Behavior</span>
      {mode === "camera" ? (
        <>
          <div className="settings-selects mode-settings-selects">
            <Field label="Audio source">
              <select value={config.audioSource ?? (config.audio ? "mic" : "off")} onChange={(event) => {
                const audioSource = event.target.value as "mic" | "output" | "off";
                update({ audioSource, audio: audioSource !== "off" });
              }}>
                <option value="mic">Microphone</option><option value="output">Device audio</option><option value="off">Off</option>
              </select>
            </Field>
            <Orientation config={config} update={update} label="Orientation" />
          </div>
          <div className="toggle-list settings-toggles compact-toggles">
            {highSpeedModes.length > 0 && <Toggle label="High-speed capture" checked={Boolean(config.cameraHighSpeed)} onChange={setHighSpeed} />}
            <Toggle label="Start fullscreen" checked={config.fullscreen} onChange={(fullscreen) => update({ fullscreen })} />
          </div>
        </>
      ) : mode === "desktop" ? (
        <div className="toggle-list settings-toggles compact-toggles">
          {!desktopIsDex && <Toggle label="System decorations" checked={!config.desktopNoDecorations} onChange={(enabled) => update({ desktopNoDecorations: !enabled })} disabled={!desktopCapabilities?.systemDecorationsSupported} />}
          {!desktopIsDex && <Toggle label="Keep apps open" checked={Boolean(config.desktopKeepContent)} onChange={(desktopKeepContent) => update({ desktopKeepContent })} disabled={!desktopCapabilities?.keepContentSupported} />}
          {!desktopIsDex && <Toggle label="Flex compatibility" checked={Boolean(config.desktopFlex)} onChange={(desktopFlex) => update({ desktopFlex })} disabled={!desktopCapabilities?.flexSupported} />}
          <Toggle label="Forward audio" checked={config.audio} onChange={(audio) => update({ audio, audioSource: audio ? "output" : "off" })} />
          <Toggle label="Start fullscreen" checked={config.fullscreen} onChange={(fullscreen) => update({ fullscreen })} />
        </div>
      ) : (
        <div className="toggle-list settings-toggles compact-toggles">
          <Toggle label="Forward audio" checked={config.audio} onChange={(audio) => update({ audio, audioSource: audio ? "output" : "off" })} />
          <Toggle label="Keep phone awake" checked={config.stayAwake} onChange={(stayAwake) => update({ stayAwake })} />
          <Toggle label="Turn screen off" checked={config.turnScreenOff} onChange={(turnScreenOff) => update({ turnScreenOff })} />
          <Toggle label="Show touches" checked={config.showTouches} onChange={(showTouches) => update({ showTouches })} />
          <Toggle label="Start fullscreen" checked={config.fullscreen} onChange={(fullscreen) => update({ fullscreen })} />
        </div>
      )}

      <details className="advanced-more">
        <summary><span>More settings</span><ChevronDown size={14} /></summary>
        <div className="advanced-more-content">
          {mode === "camera" && (
            <Field label="Aspect ratio">
              <select value={config.cameraAspectRatio ?? "auto"} onChange={(event) => update({ cameraAspectRatio: event.target.value as "auto" | "sensor" | "16:9" | "4:3" })} disabled={config.cameraHighSpeed}>
                <option value="auto">Automatic</option><option value="sensor">Camera sensor</option><option value="16:9">16:9</option><option value="4:3">4:3</option>
              </select>
            </Field>
          )}
          {mode !== "camera" && <Orientation config={config} update={update} label="Orientation lock" />}
          <Field label="Video encoder">
            <select value={config.videoEncoder ?? ""} onChange={(event) => update({ videoEncoder: event.target.value || null })} disabled={encoders.length === 0}>
              <option value="">Automatic</option>
              {encoders.map((encoder) => <option key={encoder.name} value={encoder.name}>{encoder.name}</option>)}
            </select>
          </Field>
          <Field label="Crop (width:height:x:y)">
            <input value={config.crop ?? ""} onChange={(event) => update({ crop: event.target.value || null })} placeholder="Optional" inputMode="numeric" />
          </Field>
          {mode === "desktop" && (
            <Field label="Start app package">
              <input value={config.desktopStartApp ?? ""} onChange={(event) => update({ desktopStartApp: event.target.value || null })} placeholder="Optional" />
            </Field>
          )}
        </div>
      </details>

      <button className="secondary wide reset-auto" onClick={onReset}><RotateCcw size={14} /> Reset to Auto</button>
    </>
  );
}

function Resolution({ config, update, label = "Resolution" }: { config: LaunchConfig; update: (patch: Partial<LaunchConfig>) => void; label?: string }) {
  return (
    <Field label={label}>
      <select value={config.maxSize} onChange={(event) => update({ maxSize: Number(event.target.value) })}>
        <option value={0}>Native</option><option value={1280}>1280 px</option><option value={1920}>1920 px</option><option value={2560}>2560 px</option>
      </select>
    </Field>
  );
}

function Codec({ config, options, change }: { config: LaunchConfig; options: Array<"h264" | "h265" | "av1">; change: (codec: "h264" | "h265" | "av1") => void }) {
  return (
    <Field label="Codec">
      <select value={config.codec} onChange={(event) => change(event.target.value as "h264" | "h265" | "av1")}>
        {options.map((codec) => <option key={codec} value={codec}>{codec.toUpperCase()}</option>)}
      </select>
    </Field>
  );
}

function Bitrate({ config, update }: { config: LaunchConfig; update: (patch: Partial<LaunchConfig>) => void }) {
  return (
    <Field label="Bitrate">
      <select value={config.videoBitRate} onChange={(event) => update({ videoBitRate: Number(event.target.value) })}>
        {bitrateOptions.map((bitrate) => <option key={bitrate} value={bitrate}>{bitrate} Mbps</option>)}
      </select>
    </Field>
  );
}

function Orientation({ config, update, label }: { config: LaunchConfig; update: (patch: Partial<LaunchConfig>) => void; label: string }) {
  return (
    <Field label={label}>
      <select value={config.captureOrientation ?? "auto"} onChange={(event) => update({ captureOrientation: event.target.value as "auto" | "initial" | "0" | "90" | "180" | "270" })}>
        <option value="auto">Follow device</option><option value="initial">Lock current</option><option value="0">Lock 0°</option><option value="90">Lock 90°</option><option value="180">Lock 180°</option><option value="270">Lock 270°</option>
      </select>
    </Field>
  );
}
