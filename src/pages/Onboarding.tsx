import { useMemo, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  CheckCircle2,
  Download,
  Keyboard,
  Mic,
  ShieldCheck,
  Square,
  Sparkle,
} from "lucide-react";
import { useApp } from "../hooks/useApp";
import { api, toAppError } from "../services/api";
import type { DictationResult } from "../types";
import { formatBytes, formatDuration, percent } from "../utils/format";
import { platformInfo } from "../utils/platform";
import {
  DesktopShortcutSetup,
  MicTester,
  ShortcutRecorder,
} from "../components/recording";
import { Badge, Button, FoxMark, Spinner } from "../components/ui";

const STEPS = ["Welcome", "Microphone", "Model", "Shortcut", "Test"];

export function Onboarding() {
  const {
    settings,
    capabilities,
    devices,
    models,
    recording,
    modelProgress,
    saveSettings,
    downloadModel,
    selectModel,
    startRecording,
    stopRecording,
    refreshDevices,
    notify,
    result,
  } = useApp();

  const platform = platformInfo(capabilities);
  const [step, setStep] = useState(0);
  const [mic, setMic] = useState<string | null>(settings?.microphone ?? null);
  const [modelId, setModelId] = useState(settings?.model ?? "base.en");
  const [shortcut, setShortcut] = useState(settings?.shortcut ?? platform.defaultShortcut);
  const [shortcutMode, setShortcutMode] = useState<"pushToTalk" | "toggle">(
    settings?.shortcutMode ?? "pushToTalk",
  );
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<DictationResult | null>(null);
  const [testError, setTestError] = useState<string | null>(null);
  const [finishing, setFinishing] = useState(false);

  const installed = useMemo(
    () => models.filter((model) => model.installed),
    [models],
  );
  const inputs = devices.filter((device) => !device.isMonitor);
  const downloading = Object.values(modelProgress).some(
    (progress) => progress.phase === "downloading",
  );

  if (!settings) return null;

  const next = async () => {
    if (step === 3) {
      try {
        const normalized = await api.validateShortcut(shortcut);
        setShortcut(normalized);
        setShortcutError(null);
      } catch (raw) {
        setShortcutError(toAppError(raw).message);
        return;
      }
    }
    setStep((current) => Math.min(STEPS.length - 1, current + 1));
  };

  const back = () => setStep((current) => Math.max(0, current - 1));

  const finish = async (skippedTest = false) => {
    if (finishing) return;
    if (!skippedTest && !testResult) {
      setTestError("Record and transcribe a test phrase, or finish without the test.");
      return;
    }
    setFinishing(true);
    const saved = await saveSettings({
      ...settings,
      onboardingComplete: true,
      microphone: mic,
      model: installed.find((model) => model.id === modelId)?.id ?? settings.model,
      shortcut,
      shortcutMode,
      profile: settings.profile,
    });
    setFinishing(false);
    if (saved) {
      notify("success", "DictaFox is ready. Hold your shortcut and speak.");
    }
  };

  const runTest = async () => {
    setTestError(null);
    setTestResult(null);
    await startRecording("test");
  };

  const stopTest = async () => {
    const outcome = await stopRecording();
    if (outcome) {
      setTestResult(outcome);
      setTestError(null);
    } else {
      setTestError("The test did not produce a transcript. Check the microphone and try again.");
    }
  };

  return (
    <div className="onboarding">
      <aside className="onboarding-brand">
        <div className="onboarding-brand-head">
          <FoxMark size={46} />
          <div>
            <span className="brand-name">DictaFox</span>
            <span className="brand-tag">Speak Naturally. Type Everywhere.</span>
          </div>
        </div>
        <div className="onboarding-pitch">
          <h1>Your voice, typed into any app.</h1>
          <p>
            A private, lightweight dictation tool for {platform.label}. Press a shortcut,
            speak, and polished text appears where you were writing.
          </p>
          <ul className="onboarding-points">
            <li>
              <ShieldCheck size={17} aria-hidden="true" />
              <div>
                <strong>Local speech recognition</strong>
                <span>whisper.cpp runs on your CPU. No account or subscription.</span>
              </div>
            </li>
            <li>
              <Keyboard size={17} aria-hidden="true" />
              <div>
                <strong>Works system-wide</strong>
                <span>
                  {platform.isLinux
                    ? "Clipboard-first insertion with honest Wayland fallbacks."
                    : `Pastes with ${platform.pasteKeys} into the app you were using.`}
                </span>
              </div>
            </li>
            <li>
              <Sparkle size={17} aria-hidden="true" />
              <div>
                <strong>Clean, useful text</strong>
                <span>Filler words removed; code and names preserved.</span>
              </div>
            </li>
          </ul>
        </div>
        <div className="onboarding-steps" aria-label="Setup progress">
          {STEPS.map((label, index) => (
            <div
              key={label}
              className={`onboarding-step${index === step ? " is-current" : ""}${
                index < step ? " is-done" : ""
              }`}
            >
              <span className="step-index">
                {index < step ? <Check size={12} /> : index + 1}
              </span>
              <span>{label}</span>
            </div>
          ))}
        </div>
        <p className="onboarding-privacy">
          No telemetry · No account · Models download only when you ask
        </p>
      </aside>

      <section className="onboarding-panel">
        <div className="onboarding-card card">
          <div className="onboarding-head">
            <span className="eyebrow">
              Step {step + 1} of {STEPS.length}
            </span>
            <h2>{STEPS[step]}</h2>
          </div>

          {step === 0 ? (
            <div className="onboarding-body">
              <p>
                DictaFox turns speech into text in any application. It captures
                audio only while you are dictating, transcribes locally, then
                copies and pastes the result into the window you came from.
              </p>
              <div className="onboarding-facts">
                <div>
                  <strong>1</strong>
                  <span>Press your shortcut</span>
                </div>
                <div>
                  <strong>2</strong>
                  <span>Speak naturally</span>
                </div>
                <div>
                  <strong>3</strong>
                  <span>Keep typing elsewhere</span>
                </div>
              </div>
              <div className="onboarding-notice">
                <ShieldCheck size={17} aria-hidden="true" />
                <p>
                  Before your first dictation, DictaFox needs a local speech
                  model. You will choose one in the next steps. Nothing is
                  downloaded automatically.
                </p>
              </div>
            </div>
          ) : null}

          {step === 1 ? (
            <div className="onboarding-body">
              <p>
                Choose the microphone DictaFox should use. You can change this at
                any time in Settings → Audio.
              </p>
              <MicTester
                devices={devices}
                device={mic}
                onDeviceChange={(device) => setMic(device)}
              />
              {!inputs.length ? (
                <div className="onboarding-warning">
                  <strong>No microphone detected.</strong>
                  <span>
                    Connect an input device and refresh. You can still continue,
                    but dictation needs a microphone.
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => void refreshDevices()}
                  >
                    Refresh devices
                  </Button>
                </div>
              ) : null}
              <p className="muted small">
                {platform.isLinux
                  ? "DictaFox uses PipeWire/PulseAudio capture."
                  : platform.isMac
                    ? "macOS will ask you to allow microphone access the first time you record."
                    : "DictaFox uses the Windows audio system (WASAPI)."}{" "}
                It only listens while you dictate and never records system audio.
              </p>
            </div>
          ) : null}

          {step === 2 ? (
            <div className="onboarding-body">
              <p>
                Pick a whisper.cpp model. Tiny is fastest; Base is a good
                default. Downloads are verified against their published SHA-256
                hash and can be cancelled.
              </p>
              <div className="onboarding-models">
                {models
                  .filter((model) =>
                    ["tiny.en", "base.en", "small.en", "base"].includes(model.id),
                  )
                  .map((model) => {
                    const progress = modelProgress[model.id];
                    const active = model.installed && modelId === model.id;
                    return (
                      <div
                        className={`onboarding-model${active ? " is-selected" : ""}`}
                        key={model.id}
                      >
                        <button
                          type="button"
                          className="onboarding-model-main"
                          disabled={!model.installed}
                          onClick={async () => {
                            setModelId(model.id);
                            await selectModel(model.id);
                          }}
                        >
                          <span className="model-radio" aria-hidden="true">
                            {active ? <Check size={12} /> : null}
                          </span>
                          <span>
                            <strong>{model.label}</strong>
                            <small>{model.note}</small>
                          </span>
                          <span className="model-size">
                            {formatBytes(model.sizeBytes)}
                          </span>
                        </button>
                        {!model.installed ? (
                          <div className="onboarding-model-download">
                            {progress ? (
                              <>
                                <div className="progress-track">
                                  <span
                                    style={{
                                      width: `${progress.phase === "downloading" ? percent(progress.downloaded, progress.total) : 100}%`,
                                    }}
                                  />
                                </div>
                                <span className="muted small">
                                  {progress.phase === "verifying"
                                    ? "Verifying…"
                                    : `${percent(progress.downloaded, progress.total)}%`}
                                </span>
                              </>
                            ) : (
                              <Button
                                size="sm"
                                variant="secondary"
                                icon={<Download size={14} />}
                                disabled={downloading}
                                onClick={() => void downloadModel(model.id)}
                              >
                                Download
                              </Button>
                            )}
                          </div>
                        ) : null}
                      </div>
                    );
                  })}
              </div>
              {!installed.length ? (
                <p className="onboarding-warning-text">
                  Download at least one model to continue. The download needs an
                  internet connection; transcription afterwards does not.
                </p>
              ) : null}
              <p className="muted small">
                Models are stored in {settings.customModelPath ? "a custom path" : "~/.local/share/dictafox/models"}.
              </p>
            </div>
          ) : null}

          {step === 3 ? (
            <div className="onboarding-body">
              <p>
                Set the shortcut that starts dictation. {platform.defaultShortcut} is a safe
                default; change it if another app already uses it.
              </p>
              <ShortcutRecorder
                value={shortcut}
                onChange={setShortcut}
                hint="Hold to record, or press once to start and again to stop. Change the mode below."
              />
              {shortcutError ? (
                <p className="form-error">{shortcutError}</p>
              ) : null}
              <div className="radio-group" role="radiogroup" aria-label="Shortcut mode">
                <label className={`radio-card${shortcutMode === "pushToTalk" ? " is-selected" : ""}`}>
                  <input
                    type="radio"
                    name="onboarding-shortcut-mode"
                    checked={shortcutMode === "pushToTalk"}
                    onChange={() => setShortcutMode("pushToTalk")}
                  />
                  <span>
                    <strong>Hold to talk</strong>
                    <small>
                      Release to transcribe.
                      {platform.isLinux ? " Works globally on X11." : ""}
                    </small>
                  </span>
                </label>
                <label className={`radio-card${shortcutMode === "toggle" ? " is-selected" : ""}`}>
                  <input
                    type="radio"
                    name="onboarding-shortcut-mode"
                    checked={shortcutMode === "toggle"}
                    onChange={() => setShortcutMode("toggle")}
                  />
                  <span>
                    <strong>Press to start / stop</strong>
                    <small>
                      {platform.isLinux
                        ? "Better on Wayland, where global key grabs are restricted."
                        : "Best for long dictations."}
                    </small>
                  </span>
                </label>
              </div>
              <DesktopShortcutSetup shortcut={shortcut} />
              {platform.wayland && !capabilities?.desktopShortcutAvailable ? (
                <div className="onboarding-notice is-warning">
                  <Keyboard size={17} aria-hidden="true" />
                  <p>
                    On Wayland, applications cannot reliably grab global keys. Bind{" "}
                    <code>dictafox --toggle</code> to a custom shortcut in your
                    desktop settings for a dependable shortcut. The tray icon
                    always works.
                  </p>
                </div>
              ) : null}
              {platform.isMac ? (
                <div className="onboarding-notice">
                  <Keyboard size={17} aria-hidden="true" />
                  <p>
                    To paste automatically, allow DictaFox in System Settings →
                    Privacy &amp; Security → Accessibility. Without it, the text is
                    copied and you press Cmd+V yourself.
                  </p>
                </div>
              ) : null}
            </div>
          ) : null}

          {step === 4 ? (
            <div className="onboarding-body">
              <p>
                Record a short phrase and check that it appears below. This is a
                real local Whisper transcription.
              </p>
              {!testResult ? (
                <div className="test-recorder">
                  <button
                    type="button"
                    className={`record-button compact is-${recording.state}`}
                    onClick={() =>
                      recording.state === "recording"
                        ? void stopTest()
                        : void runTest()
                    }
                    disabled={recording.state === "transcribing"}
                    aria-label={
                      recording.state === "recording"
                        ? "Stop test recording"
                        : "Start test recording"
                    }
                  >
                    {recording.state === "recording" ? (
                      <Square size={20} fill="currentColor" />
                    ) : recording.state === "transcribing" ? (
                      <Spinner label="Transcribing" />
                    ) : (
                      <Mic size={24} />
                    )}
                  </button>
                  <div className="test-copy">
                    <strong>
                      {recording.state === "recording"
                        ? "Recording your test…"
                        : recording.state === "transcribing"
                          ? "Transcribing on this device…"
                          : "Record a test phrase"}
                    </strong>
                    <span>
                      {recording.state === "recording"
                        ? "Press stop when you are done."
                        : "Say something like “DictaFox is ready to take notes.”"}
                    </span>
                  </div>
                </div>
              ) : (
                <div className="test-result">
                  <span className="test-success">
                    <CheckCircle2 size={16} aria-hidden="true" /> Test transcript
                  </span>
                  <p>{testResult.text}</p>
                  <div className="chip-row">
                    <Badge>{testResult.model}</Badge>
                    <Badge>{formatDuration(testResult.audioMs)} audio</Badge>
                    <Badge>{testResult.transcriptionMs} ms</Badge>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      setTestResult(null);
                      void runTest();
                    }}
                  >
                    Record again
                  </Button>
                </div>
              )}
              {recording.state === "recording" ? (
                <p className="muted small">
                  Tip: speak at a normal pace. Audio never leaves this computer.
                </p>
              ) : null}
              {testError ? <p className="form-error">{testError}</p> : null}
              {result && !testResult ? (
                <p className="muted small">
                  A previous transcript is available in History.
                </p>
              ) : null}
            </div>
          ) : null}

          <div className="onboarding-actions">
            {step > 0 ? (
              <Button variant="ghost" icon={<ArrowLeft size={15} />} onClick={back}>
                Back
              </Button>
            ) : (
              <button
                type="button"
                className="link-button"
                onClick={() =>
                  void saveSettings({ ...settings, onboardingComplete: true })
                }
              >
                Skip setup for now
              </button>
            )}
            <div className="onboarding-actions-right">
              {step < STEPS.length - 1 ? (
                <Button
                  variant="primary"
                  onClick={() => void next()}
                  disabled={
                    (step === 1 && !inputs.length) ||
                    (step === 2 && !installed.length)
                  }
                  icon={<ArrowRight size={15} />}
                >
                  Continue
                </Button>
              ) : (
                <>
                  <Button
                    variant="ghost"
                    onClick={() => void finish(true)}
                    disabled={finishing}
                  >
                    Finish without test
                  </Button>
                  <Button
                    variant="primary"
                    onClick={() => void finish(false)}
                    disabled={finishing}
                    icon={<Check size={15} />}
                  >
                    {finishing ? "Finishing…" : "Finish setup"}
                  </Button>
                </>
              )}
            </div>
          </div>
        </div>
        <p className="onboarding-footnote">
          DictaFox is free and open source (MIT). No account is ever required.
        </p>
      </section>
    </div>
  );
}
