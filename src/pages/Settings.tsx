import { useEffect, useState, type ReactNode } from "react";
import {
  Cpu,
  FolderOpen,
  Keyboard,
  Palette,
  RefreshCw,
  ShieldCheck,
  SlidersHorizontal,
  Terminal,
  Trash2,
  Type,
  Volume2,
  type LucideIcon,
} from "lucide-react";
import { useApp } from "../hooks/useApp";
import { api, toAppError } from "../services/api";
import type {
  Profile,
  RefinementMode,
  Settings,
  Theme,
} from "../types";
import {
  LANGUAGES,
  PROFILE_DESCRIPTIONS,
  PROFILE_LABELS,
  REFINEMENT_DESCRIPTIONS,
  REFINEMENT_LABELS,
  formatBytes,
  languageLabel,
} from "../utils/format";
import { platformInfo } from "../utils/platform";
import {
  DesktopShortcutSetup,
  MicTester,
  ShortcutRecorder,
} from "../components/recording";
import {
  Badge,
  Button,
  Field,
  LimitationsList,
  Modal,
  Toggle,
} from "../components/ui";

type TabId =
  | "general"
  | "audio"
  | "transcription"
  | "shortcuts"
  | "refinement"
  | "privacy"
  | "appearance"
  | "developer";

const TABS: { id: TabId; label: string; icon: LucideIcon }[] = [
  { id: "general", label: "General", icon: SlidersHorizontal },
  { id: "audio", label: "Audio", icon: Volume2 },
  { id: "transcription", label: "Transcription", icon: Cpu },
  { id: "shortcuts", label: "Shortcuts", icon: Keyboard },
  { id: "refinement", label: "Refinement", icon: Type },
  { id: "privacy", label: "Privacy", icon: ShieldCheck },
  { id: "appearance", label: "Appearance", icon: Palette },
  { id: "developer", label: "Developer", icon: Terminal },
];

const PROFILES: Profile[] = [
  "general",
  "coding",
  "email",
  "messaging",
  "academic",
  "professional",
];

const REFINEMENTS: RefinementMode[] = [
  "profile",
  "raw",
  "clean",
  "professional",
  "casual",
  "email",
  "technical",
  "developer",
];

function Section({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <section className="card settings-section">
      <header>
        <h2>{title}</h2>
        {description ? <p>{description}</p> : null}
      </header>
      <div className="settings-body">{children}</div>
    </section>
  );
}

export function SettingsPage() {
  const {
    settings,
    devices,
    models,
    capabilities,
    saveSettings,
    notify,
    refreshDevices,
    refreshModels,
    clearHistory,
    openDataDir,
    diagnostics,
    copyText,
    navigate,
  } = useApp();
  const platform = platformInfo(capabilities);
  const [tab, setTab] = useState<TabId>("general");
  const [draft, setDraft] = useState<Settings | null>(settings);
  const [saving, setSaving] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);
  const [report, setReport] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  useEffect(() => {
    if (!draft) return;
    if (!draft.shortcut.includes("+")) {
      setShortcutError("Use modifiers plus a key, for example Ctrl+Space.");
      return;
    }
    let active = true;
    const timer = window.setTimeout(async () => {
      try {
        await api.validateShortcut(draft.shortcut);
        if (active) setShortcutError(null);
      } catch (raw) {
        if (active) setShortcutError(toAppError(raw).message);
      }
    }, 280);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [draft?.shortcut]);

  if (!settings || !draft) return null;

  const dirty = JSON.stringify(draft) !== JSON.stringify(settings);
  const patch = (values: Partial<Settings>) =>
    setDraft((current) => (current ? { ...current, ...values } : current));
  const patchProvider = (values: Partial<Settings["provider"]>) =>
    setDraft((current) =>
      current
        ? { ...current, provider: { ...current.provider, ...values } }
        : current,
    );
  const patchInsertion = (values: Partial<Settings["insertion"]>) =>
    setDraft((current) =>
      current
        ? { ...current, insertion: { ...current.insertion, ...values } }
        : current,
    );

  const save = async () => {
    if (shortcutError) {
      setTab("shortcuts");
      notify("warning", shortcutError);
      return;
    }
    setSaving(true);
    const saved = await saveSettings(draft);
    setSaving(false);
    if (saved) {
      setDraft(saved);
      notify("success", "Settings saved.");
    }
  };

  const runPreview = async () => {
    setPreviewing(true);
    setPreviewError(null);
    try {
      const outcome = await api.refinePreview(
        "um i think we should update the api and then test it",
      );
      setPreview(outcome.text);
      if (outcome.warning) setPreviewError(outcome.warning);
    } catch (raw) {
      setPreviewError(toAppError(raw).message);
    } finally {
      setPreviewing(false);
    }
  };

  const loadDiagnostics = async () => {
    const text = await diagnostics();
    if (text) setReport(text);
  };

  const installedModels = models.filter((model) => model.installed);

  return (
    <div className="page settings-page">
      <header className="page-head">
        <div>
          <h1>Settings</h1>
          <p className="page-subtitle">
            Everything here changes real behavior. Settings are stored locally.
          </p>
        </div>
        <div className="page-actions">
          {dirty ? <Badge tone="warning">Unsaved changes</Badge> : <Badge>Saved</Badge>}
          <Button
            variant="ghost"
            size="sm"
            icon={<RefreshCw size={14} />}
            onClick={() => {
              void refreshDevices();
              void refreshModels();
            }}
          >
            Refresh system
          </Button>
        </div>
      </header>

      <div className="settings-layout">
        <nav className="settings-tabs" aria-label="Settings sections">
          {TABS.map((entry) => {
            const Icon = entry.icon;
            return (
              <button
                key={entry.id}
                type="button"
                className={`settings-tab${tab === entry.id ? " is-active" : ""}`}
                onClick={() => setTab(entry.id)}
                aria-current={tab === entry.id ? "page" : undefined}
              >
                <Icon size={16} strokeWidth={1.8} aria-hidden="true" />
                {entry.label}
              </button>
            );
          })}
        </nav>

        <div className="settings-content">
          {tab === "general" ? (
            <>
              <Section
                title="Dictation"
                description="Control how long DictaFox can listen and what it keeps."
              >
                <div className="form-grid">
                  <Field label="Active profile" hint={PROFILE_DESCRIPTIONS[draft.profile]}>
                    <select
                      value={draft.profile}
                      onChange={(event) =>
                        patch({ profile: event.target.value as Profile })
                      }
                    >
                      {PROFILES.map((profile) => (
                        <option key={profile} value={profile}>
                          {PROFILE_LABELS[profile]}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <Field
                    label="Maximum recording length"
                    hint="The recorder stops automatically at this limit."
                  >
                    <div className="input-unit">
                      <input
                        type="number"
                        min={5}
                        max={600}
                        value={draft.maxRecordingSeconds}
                        onChange={(event) =>
                          patch({
                            maxRecordingSeconds: Number(event.target.value) || 5,
                          })
                        }
                      />
                      <span>seconds</span>
                    </div>
                  </Field>
                </div>
                <div className="form-grid">
                  <Toggle
                    checked={draft.persistHistory}
                    onChange={(checked) => patch({ persistHistory: checked })}
                    label="Save transcript history"
                    description="Newest transcripts are kept in the local SQLite database."
                  />
                  <Toggle
                    checked={draft.saveAudio}
                    onChange={(checked) => patch({ saveAudio: checked })}
                    label="Keep recorded audio"
                    description="Recordings are deleted after transcription unless this is on."
                  />
                </div>
                <Toggle
                  checked={draft.developerMode}
                  onChange={(checked) => patch({ developerMode: checked })}
                  label="Developer mode"
                  description="Show technical details and diagnostics sections more prominently."
                />
              </Section>

              <Section
                title="Text insertion"
                description="DictaFox copies the transcript first, then pastes it when your desktop allows."
              >
                <div className="form-grid">
                  <Field
                    label="Insertion mode"
                    hint="Clipboard only never simulates keyboard input."
                  >
                    <select
                      value={draft.insertion.mode}
                      onChange={(event) =>
                        patchInsertion({
                          mode: event.target.value as Settings["insertion"]["mode"],
                        })
                      }
                    >
                      <option value="auto">Automatic when supported</option>
                      <option value="clipboardOnly">Clipboard only</option>
                    </select>
                  </Field>
                  <Field
                    label="Paste delay"
                    hint={`Wait before simulating ${platform.pasteKeys} so focus can return.`}
                  >
                    <div className="input-unit">
                      <input
                        type="number"
                        min={0}
                        max={2000}
                        step={10}
                        value={draft.insertion.pasteDelayMs}
                        onChange={(event) =>
                          patchInsertion({
                            pasteDelayMs: Number(event.target.value) || 0,
                          })
                        }
                      />
                      <span>ms</span>
                    </div>
                  </Field>
                </div>
                <Toggle
                  checked={draft.insertion.restoreClipboard}
                  onChange={(checked) => patchInsertion({ restoreClipboard: checked })}
                  label="Restore the previous clipboard after pasting"
                  description="Only when a paste was simulated; clipboard-only mode keeps the transcript available."
                />
              </Section>

              <Section
                title="Interface"
                description="Shortcuts and data locations."
              >
                <div className="settings-links">
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<Keyboard size={15} />}
                    onClick={() => setTab("shortcuts")}
                  >
                    Configure shortcut
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<Cpu size={15} />}
                    onClick={() => navigate("models")}
                  >
                    Manage models
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<FolderOpen size={15} />}
                    onClick={() => void openDataDir()}
                  >
                    Open data folder
                  </Button>
                </div>
              </Section>

              <Section
                title="First-run setup"
                description="Replay the guided microphone, model, shortcut and test flow."
              >
                <p className="muted">
                  Your current settings are kept. The setup wizard guides you
                  through verifying each component again.
                </p>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() =>
                    void saveSettings({ ...draft, onboardingComplete: false })
                  }
                >
                  Run onboarding again
                </Button>
              </Section>
            </>
          ) : null}

          {tab === "audio" ? (
            <>
              <Section
                title="Microphone"
                description={
                  platform.isLinux
                    ? "DictaFox captures 16 kHz mono audio through PipeWire or PulseAudio."
                    : `DictaFox captures audio through ${platform.isMac ? "CoreAudio" : "WASAPI"} and converts it to 16 kHz mono.`
                }
              >
                <div className="form-grid">
                  <Field
                    label="Input device"
                    hint="System default follows your desktop audio settings."
                  >
                    <select
                      value={draft.microphone ?? ""}
                      onChange={(event) =>
                        patch({ microphone: event.target.value || null })
                      }
                    >
                      <option value="">System default</option>
                      {devices
                        .filter((device) => !device.isMonitor)
                        .map((device) => (
                          <option key={device.id} value={device.id}>
                            {device.name}
                            {device.isDefault ? " (default)" : ""}
                          </option>
                        ))}
                    </select>
                  </Field>
                  <Field
                    label="Input gain"
                    hint="Only amplifies quiet recordings; loud audio is clipped safely."
                  >
                    <div className="range-row">
                      <input
                        type="range"
                        min={0.5}
                        max={3}
                        step={0.1}
                        value={draft.inputGain}
                        onChange={(event) =>
                          patch({ inputGain: Number(event.target.value) })
                        }
                      />
                      <span className="range-value">
                        {draft.inputGain.toFixed(1)}×
                      </span>
                    </div>
                  </Field>
                </div>
                <Toggle
                  checked={draft.vadEnabled}
                  onChange={(checked) => patch({ vadEnabled: checked })}
                  label="Trim leading and trailing silence"
                  description="Energy-based voice activity trimming keeps 200 ms of padding around speech."
                />
                <MicTester
                  devices={devices}
                  device={draft.microphone}
                  onDeviceChange={(device) => patch({ microphone: device })}
                />
                <div className="settings-links">
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={<RefreshCw size={14} />}
                    onClick={() => void refreshDevices()}
                  >
                    Refresh devices
                  </Button>
                  <span className="muted small">
                    Backend: {capabilities?.audioBackend ?? "unavailable"}
                  </span>
                </div>
              </Section>
            </>
          ) : null}

          {tab === "transcription" ? (
            <>
              <Section
                title="Speech recognition"
                description="whisper.cpp runs locally. No audio leaves this machine."
              >
                <div className="form-grid">
                  <Field
                    label="Model"
                    hint={
                      draft.customModelPath
                        ? `Custom file: ${draft.customModelPath}`
                        : "Only installed models can be selected."
                    }
                  >
                    <select
                      value={draft.customModelPath ? "__custom" : draft.model}
                      onChange={(event) => {
                        if (event.target.value === "__custom") return;
                        patch({
                          model: event.target.value,
                          customModelPath: null,
                        });
                      }}
                    >
                      {draft.customModelPath ? (
                        <option value="__custom">Custom model file</option>
                      ) : null}
                      {installedModels.map((model) => (
                        <option key={model.id} value={model.id}>
                          {model.label} · {formatBytes(model.sizeBytes)}
                        </option>
                      ))}
                      {!installedModels.length ? (
                        <option value={draft.model}>
                          {draft.model} (not installed)
                        </option>
                      ) : null}
                    </select>
                  </Field>
                  <Field
                    label="Language"
                    hint="Auto-detection needs a multilingual model."
                  >
                    <select
                      value={draft.language}
                      onChange={(event) =>
                        patch({ language: event.target.value })
                      }
                    >
                      {LANGUAGES.map((language) => (
                        <option key={language.value} value={language.value}>
                          {language.label}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <Field
                    label="Inference threads"
                    hint="0 lets DictaFox use available cores minus one."
                  >
                    <input
                      type="number"
                      min={0}
                      max={64}
                      value={draft.threads}
                      onChange={(event) =>
                        patch({ threads: Number(event.target.value) || 0 })
                      }
                    />
                  </Field>
                </div>
                <Toggle
                  checked={draft.keepModelLoaded}
                  onChange={(checked) => patch({ keepModelLoaded: checked })}
                  label="Keep the model loaded between dictations"
                  description="Faster follow-up dictations at the cost of RAM. Turn off to free memory after each transcript."
                />
                <div className="settings-links">
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<Cpu size={15} />}
                    onClick={() => navigate("models")}
                  >
                    Download or verify models
                  </Button>
                  <span className="muted small">
                    {languageLabel(draft.language)} ·{" "}
                    {installedModels.length
                      ? `${installedModels.length} installed`
                      : "No model installed"}
                  </span>
                </div>
              </Section>
            </>
          ) : null}

          {tab === "shortcuts" ? (
            <>
              <Section
                title="Global shortcut"
                description={
                  platform.isLinux
                    ? "X11 registers global keys directly. Wayland blocks global grabs, so bind the CLI trigger in your desktop settings."
                    : `The shortcut works in every app on ${platform.label}.`
                }
              >
                <ShortcutRecorder
                  value={draft.shortcut}
                  onChange={(value) => patch({ shortcut: value })}
                  hint={`Use Ctrl, ${platform.isMac ? "Option" : "Alt"}, Shift or ${platform.superKey} plus a letter, digit, F-key or Space.`}
                />
                {shortcutError ? (
                  <p className="form-error">{shortcutError}</p>
                ) : null}
                <div className="radio-group" role="radiogroup" aria-label="Shortcut mode">
                  <label className={`radio-card${draft.shortcutMode === "pushToTalk" ? " is-selected" : ""}`}>
                    <input
                      type="radio"
                      name="shortcut-mode"
                      checked={draft.shortcutMode === "pushToTalk"}
                      onChange={() => patch({ shortcutMode: "pushToTalk" })}
                    />
                    <span>
                      <strong>Hold to talk</strong>
                      <small>
                        Record while the key is held.
                        {platform.isLinux
                          ? " Release detection works on X11; on Wayland the second press stops the recording."
                          : " Releasing the key transcribes."}
                      </small>
                    </span>
                  </label>
                  <label className={`radio-card${draft.shortcutMode === "toggle" ? " is-selected" : ""}`}>
                    <input
                      type="radio"
                      name="shortcut-mode"
                      checked={draft.shortcutMode === "toggle"}
                      onChange={() => patch({ shortcutMode: "toggle" })}
                    />
                    <span>
                      <strong>Press to start / stop</strong>
                      <small>{platform.isLinux ? "Best for Wayland or for long dictations." : "Best for long dictations."}</small>
                    </span>
                  </label>
                </div>
                <DesktopShortcutSetup shortcut={settings?.shortcut ?? null} />
                {capabilities?.limitations
                  .filter((limitation) => limitation.code === "wayland_shortcut")
                  .map((limitation) => (
                    <div className="settings-note" key={limitation.code}>
                      <strong>{limitation.title}</strong>
                      <p>{limitation.hint}</p>
                      <code>dictafox --toggle</code>
                    </div>
                  ))}
              </Section>
              <Section title="Cancel and stop">
                <p className="muted">
                  Pressing the shortcut again while recording stops and
                  transcribes. Cancel from the dashboard or tray. A watchdog
                  stops any recording longer than your maximum length.
                </p>
              </Section>
            </>
          ) : null}

          {tab === "refinement" ? (
            <>
              <Section
                title="Text cleanup"
                description="Local refinement always runs first. Cloud refinement is optional and never required for dictation."
              >
                <div className="form-grid">
                  <Field
                    label="Active profile"
                    hint={PROFILE_DESCRIPTIONS[draft.profile]}
                  >
                    <select
                      value={draft.profile}
                      onChange={(event) =>
                        patch({ profile: event.target.value as Profile })
                      }
                    >
                      {PROFILES.map((profile) => (
                        <option key={profile} value={profile}>
                          {PROFILE_LABELS[profile]}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <Field
                    label="Refinement mode"
                    hint={REFINEMENT_DESCRIPTIONS[draft.refinement]}
                  >
                    <select
                      value={draft.refinement}
                      onChange={(event) =>
                        patch({
                          refinement: event.target.value as RefinementMode,
                        })
                      }
                    >
                      {REFINEMENTS.map((mode) => (
                        <option key={mode} value={mode}>
                          {REFINEMENT_LABELS[mode]}
                        </option>
                      ))}
                    </select>
                  </Field>
                </div>
                <Toggle
                  checked={draft.dictionaryEnabled}
                  onChange={(checked) => patch({ dictionaryEnabled: checked })}
                  label="Apply personal dictionary"
                  description="Spoken forms are replaced locally before any provider sees the text."
                />
                <div className="preview-box">
                  <div className="preview-head">
                    <strong>Preview refinement</strong>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => void runPreview()}
                      disabled={previewing}
                    >
                      {previewing ? "Refining…" : "Run preview"}
                    </Button>
                  </div>
                  <p className="muted small">
                    Input: “um i think we should update the api and then test it”
                  </p>
                  {preview ? <p className="preview-result">{preview}</p> : null}
                  {previewError ? (
                    <p className="result-warning">{previewError}</p>
                  ) : null}
                </div>
              </Section>

              <Section
                title="Optional cloud refinement"
                description="Send the transcript (never the audio) to an OpenAI-compatible endpoint. Disabled by default."
              >
                <Toggle
                  checked={draft.provider.enabled}
                  onChange={(checked) => patchProvider({ enabled: checked })}
                  label="Enable cloud refinement"
                  description="If the provider is unreachable, DictaFox keeps the locally refined transcript."
                />
                <div className="form-grid">
                  <Field label="API base URL" hint="Must expose /chat/completions.">
                    <input
                      type="url"
                      value={draft.provider.baseUrl}
                      onChange={(event) =>
                        patchProvider({ baseUrl: event.target.value })
                      }
                      placeholder="https://api.openai.com/v1"
                    />
                  </Field>
                  <Field label="Model">
                    <input
                      type="text"
                      value={draft.provider.model}
                      onChange={(event) =>
                        patchProvider({ model: event.target.value })
                      }
                      placeholder="gpt-4o-mini"
                    />
                  </Field>
                  <Field
                    label="API key"
                    hint="Stored locally. DICTAFOX_OPENAI_API_KEY overrides this value."
                  >
                    <input
                      type="password"
                      autoComplete="off"
                      value={draft.provider.apiKey ?? ""}
                      onChange={(event) =>
                        patchProvider({ apiKey: event.target.value || null })
                      }
                      placeholder="sk-…"
                    />
                  </Field>
                  <Field label="Timeout" hint="Seconds before falling back to local text.">
                    <input
                      type="number"
                      min={3}
                      max={120}
                      value={draft.provider.timeoutSeconds}
                      onChange={(event) =>
                        patchProvider({
                          timeoutSeconds: Number(event.target.value) || 20,
                        })
                      }
                    />
                  </Field>
                </div>
                {draft.provider.enabled ? (
                  <div className="settings-note is-warning">
                    <strong>Cloud privacy notice</strong>
                    <p>
                      With this enabled, each final transcript is sent to{" "}
                      {draft.provider.baseUrl || "your provider"} for rewriting.
                      Audio is never uploaded. Your provider's retention policy
                      applies. Turn this off to keep everything local.
                    </p>
                  </div>
                ) : null}
              </Section>
            </>
          ) : null}

          {tab === "privacy" ? (
            <>
              <Section
                title="Local by default"
                description="DictaFox has no account system, no telemetry and no background uploads."
              >
                <ul className="check-list">
                  <li>Whisper models and inference are fully local.</li>
                  <li>History is stored in SQLite under your user data directory.</li>
                  <li>
                    Audio is captured only while a recording is active; the
                    sidebar and tray show recording state.
                  </li>
                  <li>
                    API keys are written to a 0600 database file and never logged.
                  </li>
                </ul>
                <div className="settings-links">
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<FolderOpen size={15} />}
                    onClick={() => void openDataDir()}
                  >
                    Open data folder
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={<Trash2 size={15} />}
                    onClick={() => setConfirmClear(true)}
                  >
                    Delete all history
                  </Button>
                </div>
              </Section>
              <Section title="Data retention">
                <Toggle
                  checked={draft.persistHistory}
                  onChange={(checked) => patch({ persistHistory: checked })}
                  label="Save transcript history"
                  description="Turning this off stops new entries. Existing entries remain until you delete them."
                />
                <Toggle
                  checked={draft.saveAudio}
                  onChange={(checked) => patch({ saveAudio: checked })}
                  label="Keep recorded audio files"
                  description="Stored in the local cache directory. Off by default."
                />
                <Toggle
                  checked={draft.provider.enabled}
                  onChange={(checked) => patchProvider({ enabled: checked })}
                  label="Cloud refinement"
                  description="When on, final text is sent to your configured provider."
                />
              </Section>
            </>
          ) : null}

          {tab === "appearance" ? (
            <Section
              title="Appearance"
              description="DictaFox is designed dark-first with a light theme for bright rooms."
            >
              <div className="theme-grid" role="radiogroup" aria-label="Theme">
                {(
                  [
                    ["dark", "Dark", "Night surfaces with jade accents."],
                    ["light", "Light", "Paper surfaces with the same accent."],
                    ["system", "System", "Follow the desktop preference."],
                  ] as [Theme, string, string][]
                ).map(([value, label, description]) => (
                  <label
                    key={value}
                    className={`theme-card${draft.theme === value ? " is-selected" : ""}`}
                  >
                    <input
                      type="radio"
                      name="theme"
                      checked={draft.theme === value}
                      onChange={() => patch({ theme: value })}
                    />
                    <span className={`theme-swatch theme-${value}`} aria-hidden="true" />
                    <span>
                      <strong>{label}</strong>
                      <small>{description}</small>
                    </span>
                  </label>
                ))}
              </div>
              <Toggle
                checked={draft.reduceMotion}
                onChange={(checked) => patch({ reduceMotion: checked })}
                label="Reduce motion"
                description="Disable the recording pulse and transitions."
              />
              <p className="muted small">
                Appearance changes apply when you save.
              </p>
            </Section>
          ) : null}

          {tab === "developer" ? (
            <>
              <Section
                title="Diagnostics"
                description="A redacted report for bug reports. It never contains transcripts, audio or API keys."
              >
                <div className="settings-links">
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<RefreshCw size={14} />}
                    onClick={() => void loadDiagnostics()}
                  >
                    Generate report
                  </Button>
                  {report ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => void copyText(report)}
                    >
                      Copy report
                    </Button>
                  ) : null}
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={<FolderOpen size={14} />}
                    onClick={() => void openDataDir()}
                  >
                    Data folder
                  </Button>
                </div>
                <textarea
                  className="diagnostics-box"
                  readOnly
                  value={report ?? "Generate a report to inspect this machine's setup."}
                  aria-label="Diagnostics report"
                />
              </Section>

              <Section title="System dependencies">
                {!(capabilities?.dependencies ?? []).length ? (
                  <p className="muted">
                    Nothing extra to install: DictaFox uses built-in {platform.label} audio,
                    clipboard and keyboard APIs.
                  </p>
                ) : null}
                <ul className="dependency-list">
                  {(capabilities?.dependencies ?? []).map((dependency) => (
                    <li key={dependency.name}>
                      <div>
                        <strong>{dependency.name}</strong>
                        <span className="muted small">{dependency.purpose}</span>
                      </div>
                      <div className="dependency-state">
                        {dependency.installed ? (
                          <Badge tone="success">Installed</Badge>
                        ) : (
                          <Badge tone={dependency.required ? "danger" : "warning"}>
                            Missing{dependency.required ? " · required" : " · optional"}
                          </Badge>
                        )}
                        {!dependency.installed ? (
                          <code>{dependency.installHint}</code>
                        ) : null}
                      </div>
                    </li>
                  ))}
                </ul>
              </Section>

              <Section title="Session limitations">
                <LimitationsList limitations={capabilities?.limitations ?? []} />
                {!capabilities?.limitations.length ? (
                  <p className="muted">No {platform.label} integration limitations detected.</p>
                ) : null}
              </Section>
            </>
          ) : null}
        </div>
      </div>

      {dirty ? (
        <div className="settings-savebar" role="region" aria-label="Unsaved changes">
          <span>Unsaved changes</span>
          <div>
            <Button variant="ghost" size="sm" onClick={() => setDraft(settings)}>
              Discard
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => void save()}
              disabled={saving}
            >
              {saving ? "Saving…" : "Save changes"}
            </Button>
          </div>
        </div>
      ) : null}

      {confirmClear ? (
        <Modal
          title="Delete all transcripts?"
          description="This deletes every transcript from the local database. Models and dictionary entries are kept."
          onClose={() => setConfirmClear(false)}
          footer={
            <>
              <Button variant="ghost" onClick={() => setConfirmClear(false)}>
                Keep transcripts
              </Button>
              <Button
                variant="danger"
                onClick={async () => {
                  await clearHistory();
                  setConfirmClear(false);
                }}
              >
                Delete everything
              </Button>
            </>
          }
        />
      ) : null}
    </div>
  );
}
