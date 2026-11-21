import {
  ArrowRight,
  Check,
  ClipboardCopy,
  Cpu,
  Download,
  History as HistoryIcon,
  Keyboard,
  Mic,
  MicOff,
  ShieldCheck,
  Square,
  Type,
  X,
} from "lucide-react";
import { useApp } from "../hooks/useApp";
import { platformInfo } from "../utils/platform";
import {
  PROFILE_LABELS,
  REFINEMENT_LABELS,
  formatBytes,
  formatDuration,
  formatRelativeTime,
  languageLabel,
  truncate,
} from "../utils/format";
import { Waveform } from "../components/recording";
import {
  Badge,
  Button,
  EmptyState,
  LimitationsList,
  Spinner,
  Toggle,
  WarningBanner,
} from "../components/ui";
import type { Profile } from "../types";

const PROFILES: Profile[] = [
  "general",
  "coding",
  "email",
  "messaging",
  "academic",
  "professional",
];

export function Dashboard() {
  const {
    settings,
    capabilities,
    devices,
    models,
    history,
    dictionary,
    recording,
    levels,
    result,
    startRecording,
    stopRecording,
    cancelRecording,
    copyText,
    insertText,
    setProfile,
    saveSettings,
    navigate,
    shortcutWarning,
    dismissShortcutWarning,
  } = useApp();

  if (!settings) return null;

  const state = recording.state;
  const activeModel = models.find((model) => model.active);
  const deviceName =
    (settings.microphone
      ? devices.find((device) => device.id === settings.microphone)?.name
      : devices.find((device) => !device.isMonitor)?.name) ??
    (devices.length ? "System default" : "No input detected");
  const recent = history.slice(0, 4);
  const modelMissing = !capabilities?.modelInstalled;
  const update = async (patch: Partial<typeof settings>) => {
    await saveSettings({ ...settings, ...patch });
  };

  return (
    <div className="page dashboard">
      <header className="page-head">
        <div>
          <h1>Dictate anything, anywhere.</h1>
          <p className="page-subtitle">
            Local Whisper transcription · {settings.shortcut} ·{" "}
            {PROFILE_LABELS[settings.profile]} profile
          </p>
        </div>
        <div className="page-actions">
          <label className="inline-select">
            <span className="field-label">Profile</span>
            <select
              value={settings.profile}
              onChange={(event) =>
                void setProfile(event.target.value as Profile)
              }
            >
              {PROFILES.map((profile) => (
                <option key={profile} value={profile}>
                  {PROFILE_LABELS[profile]}
                </option>
              ))}
            </select>
          </label>
          <Button
            variant="ghost"
            size="sm"
            icon={<Keyboard size={15} />}
            onClick={() => navigate("settings")}
          >
            Shortcut
          </Button>
        </div>
      </header>

      {shortcutWarning ? (
        <WarningBanner
          title="Shortcut warning"
          onDismiss={dismissShortcutWarning}
        >
          {shortcutWarning}
        </WarningBanner>
      ) : null}

      {modelMissing ? (
        <WarningBanner title="Install a speech model to start dictating">
          Transcription runs locally, so DictaFox needs a whisper.cpp model
          file. Nothing is downloaded without your consent.
          <Button
            variant="primary"
            size="sm"
            icon={<Download size={14} />}
            onClick={() => navigate("models")}
          >
            Choose a model
          </Button>
        </WarningBanner>
      ) : null}

      <section className={`hero card is-${state}`}>
        <div className="hero-left">
          <div className="hero-status">
            <span className="status-dot" aria-hidden="true" />
            <span>
              {state === "recording"
                ? "Recording — speak naturally"
                : state === "transcribing"
                  ? "Transcribing locally"
                  : "Ready when you are"}
            </span>
          </div>

          <div className="record-zone">
            <button
              type="button"
              className={`record-button is-${state}`}
              onClick={() => {
                if (state === "recording") void stopRecording();
                else if (state === "idle") void startRecording("ui");
              }}
              disabled={state === "transcribing" || modelMissing}
              aria-label={
                state === "recording"
                  ? "Stop recording and transcribe"
                  : "Start recording"
              }
            >
              {state === "recording" ? (
                <Square size={26} fill="currentColor" />
              ) : state === "transcribing" ? (
                <Spinner label="Transcribing" />
              ) : (
                <Mic size={30} />
              )}
              <span className="record-label">
                {state === "recording"
                  ? formatDuration(recording.durationMs)
                  : state === "transcribing"
                    ? "Working"
                    : "Record"}
              </span>
            </button>

            <div className="record-details">
              <Waveform levels={levels} active={state === "recording"} />
              <div className="record-meta">
                <span>
                  <Mic size={13} aria-hidden="true" /> {deviceName}
                </span>
                <span>
                  {state === "recording"
                    ? `Auto-stop at ${formatDuration(recording.maxSeconds * 1000)}`
                    : settings.shortcutMode === "pushToTalk"
                      ? "Hold the shortcut to dictate"
                      : "Press the shortcut to start and stop"}
                </span>
              </div>
              <div className="record-actions">
                <Button
                  variant="primary"
                  onClick={() =>
                    state === "recording"
                      ? void stopRecording()
                      : void startRecording("ui")
                  }
                  disabled={state === "transcribing" || modelMissing}
                  icon={
                    state === "recording" ? (
                      <Square size={14} />
                    ) : (
                      <Mic size={15} />
                    )
                  }
                >
                  {state === "recording"
                    ? "Stop & transcribe"
                    : "Start recording"}
                </Button>
                {state === "recording" ? (
                  <Button
                    variant="ghost"
                    onClick={() => void cancelRecording()}
                    icon={<X size={14} />}
                  >
                    Cancel
                  </Button>
                ) : (
                  <kbd className="key-hint">{settings.shortcut}</kbd>
                )}
              </div>
            </div>
          </div>
        </div>

        <div className="hero-result">
          <div className="card-heading">
            <h2>Last transcript</h2>
            {result ? (
              <span className="muted">
                {formatRelativeTime(result.createdAt)}
              </span>
            ) : null}
          </div>
          {result ? (
            <div className="result-body">
              <p className="result-text">{result.text}</p>
              <div className="chip-row">
                <Badge tone={result.inserted ? "success" : "neutral"}>
                  {result.inserted
                    ? "Inserted"
                    : result.insertion?.clipboard
                      ? "On clipboard"
                      : "Not inserted"}
                </Badge>
                <Badge>{REFINEMENT_LABELS[result.mode]}</Badge>
                <Badge>{result.model}</Badge>
                <Badge>{formatDuration(result.transcriptionMs)}</Badge>
              </div>
              {result.warning ? (
                <p className="result-warning">{result.warning}</p>
              ) : null}
              <div className="result-actions">
                <Button
                  size="sm"
                  icon={<ClipboardCopy size={14} />}
                  onClick={() => void copyText(result.text)}
                >
                  Copy
                </Button>
                <Button
                  size="sm"
                  variant="primary"
                  icon={<Type size={14} />}
                  onClick={() => void insertText(result.text)}
                >
                  Insert
                </Button>
              </div>
            </div>
          ) : (
            <div className="result-empty">
              <p>
                Your next transcript will appear here. DictaFox never invents
                sample text.
              </p>
              <span className="muted">
                Audio is captured only while recording.
              </span>
            </div>
          )}
        </div>
      </section>

      <div className="dash-grid">
        <section className="card stat-card">
          <div className="card-heading">
            <h2>
              <Mic size={16} aria-hidden="true" /> Microphone
            </h2>
            <Badge tone={devices.length ? "success" : "danger"}>
              {devices.length ? "Connected" : "Missing"}
            </Badge>
          </div>
          <p className="stat-value">{deviceName}</p>
          <p className="stat-detail">
            {capabilities?.audioBackend ?? "Audio backend unavailable"} ·{" "}
            {devices.filter((device) => !device.isMonitor).length} input
            {devices.filter((device) => !device.isMonitor).length === 1
              ? ""
              : "s"}
          </p>
          <div className="card-footer">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => navigate("settings")}
              icon={<ArrowRight size={14} />}
            >
              Audio settings
            </Button>
            {!devices.length ? (
              <MicOff size={16} className="muted" aria-hidden="true" />
            ) : null}
          </div>
        </section>

        <section className="card stat-card">
          <div className="card-heading">
            <h2>
              <Cpu size={16} aria-hidden="true" /> Speech model
            </h2>
            <Badge tone={activeModel?.installed ? "success" : "warning"}>
              {activeModel?.installed ? "Installed" : "Not installed"}
            </Badge>
          </div>
          <p className="stat-value">{activeModel?.label ?? settings.model}</p>
          <p className="stat-detail">
            {activeModel ? formatBytes(activeModel.sizeBytes) : "—"} ·{" "}
            {settings.language === "auto"
              ? "Auto language"
              : languageLabel(settings.language)}{" "}
            · {settings.threads === 0 ? "Auto threads" : `${settings.threads} threads`}
          </p>
          <div className="card-footer">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => navigate("models")}
              icon={<ArrowRight size={14} />}
            >
              Manage models
            </Button>
            <span className="muted small">whisper.cpp · on-device</span>
          </div>
        </section>

        <section className="card stat-card">
          <div className="card-heading">
            <h2>
              <Keyboard size={16} aria-hidden="true" /> Shortcut
            </h2>
            <Badge tone="accent">
              {settings.shortcutMode === "pushToTalk" ? "Push to talk" : "Toggle"}
            </Badge>
          </div>
          <p className="stat-value">
            <kbd>{settings.shortcut}</kbd>
          </p>
          <p className="stat-detail">
            {capabilities?.globalShortcutSupported
              ? `Works in every app${platformInfo(capabilities).isLinux ? " (X11)" : ""}.`
              : "Wayland: bind a desktop shortcut to dictafox --toggle."}
          </p>
          <div className="card-footer">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => navigate("settings")}
              icon={<ArrowRight size={14} />}
            >
              Change shortcut
            </Button>
          </div>
        </section>

        <section className="card stat-card">
          <div className="card-heading">
            <h2>
              <Type size={16} aria-hidden="true" /> Text intelligence
            </h2>
            <Badge tone={settings.provider.enabled ? "warning" : "success"}>
              {settings.provider.enabled ? "Local + cloud" : "Local only"}
            </Badge>
          </div>
          <p className="stat-value">{REFINEMENT_LABELS[settings.refinement]}</p>
          <p className="stat-detail">
            {PROFILE_LABELS[settings.profile]} profile ·{" "}
            {settings.dictionaryEnabled
              ? `${dictionary.length} dictionary entries`
              : "Dictionary off"}
          </p>
          <div className="card-footer">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => navigate("settings")}
              icon={<ArrowRight size={14} />}
            >
              Refinement settings
            </Button>
          </div>
        </section>

        <section className="card recent-card">
          <div className="card-heading">
            <h2>
              <HistoryIcon size={16} aria-hidden="true" /> Recent activity
            </h2>
            <Button variant="ghost" size="sm" onClick={() => navigate("history")}>
              View history
            </Button>
          </div>
          {recent.length ? (
            <ul className="recent-list">
              {recent.map((entry) => (
                <li key={entry.id}>
                  <button
                    type="button"
                    className="recent-row"
                    onClick={() => navigate("history")}
                  >
                    <span className="recent-text">{truncate(entry.text, 110)}</span>
                    <span className="recent-meta">
                      {formatRelativeTime(entry.createdAt)} · {entry.model}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState
              icon={<HistoryIcon size={20} />}
              title="No transcripts yet"
              action={
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => navigate("settings")}
                >
                  Review privacy settings
                </Button>
              }
            >
              Dictate once and it will show up here. History can be turned off at
              any time.
            </EmptyState>
          )}
        </section>

        <section className="card privacy-card">
          <div className="card-heading">
            <h2>
              <ShieldCheck size={16} aria-hidden="true" /> Privacy
            </h2>
            <Badge tone="accent">Local first</Badge>
          </div>
          <ul className="check-list">
            <li>
              <Check size={14} aria-hidden="true" /> Speech recognition runs on
              this device with whisper.cpp.
            </li>
            <li>
              <Check size={14} aria-hidden="true" /> No account, no telemetry and
              no background recording.
            </li>
            <li>
              <Check size={14} aria-hidden="true" />{" "}
              {settings.persistHistory
                ? "Transcript history stays in your local database."
                : "Transcript history is disabled."}
            </li>
            <li>
              <Check size={14} aria-hidden="true" />{" "}
              {settings.saveAudio
                ? "Recorded audio is kept on disk after transcription."
                : "Recorded audio is deleted after transcription."}
            </li>
          </ul>
          <div className="card-footer">
            <Toggle
              checked={settings.persistHistory}
              onChange={(checked) => void update({ persistHistory: checked })}
              label="Save transcripts"
              description="Stored only in DictaFox's local database."
            />
          </div>
        </section>
      </div>

      {capabilities?.limitations.length ? (
        <section className="card limitations-card">
          <div className="card-heading">
            <h2>This {capabilities.osLabel ?? "Linux"} session</h2>
            <Badge>
              {capabilities.displayServer} · {capabilities.desktop}
            </Badge>
          </div>
          <LimitationsList limitations={capabilities.limitations} />
        </section>
      ) : null}
    </div>
  );
}
