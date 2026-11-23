import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Check, Keyboard, Mic, Square } from "lucide-react";
import type { AudioDevice, MicTestResult, MicTestState } from "../types";
import { api, toAppError } from "../services/api";
import { formatDuration } from "../utils/format";
import { Button } from "./ui";
import { platformInfo } from "../utils/platform";
import { useApp } from "../hooks/useApp";

/**
 * GNOME on Wayland only delivers global shortcuts that are bound in GNOME's
 * own keyboard settings. This registers `dictafox --toggle` there in one click.
 * Renders nothing on other desktops.
 */
export function DesktopShortcutSetup({ shortcut }: { shortcut: string | null }) {
  const { capabilities, setupDesktopShortcut } = useApp();
  const [busy, setBusy] = useState(false);
  if (!capabilities?.desktopShortcutAvailable) return null;
  const installed = capabilities.desktopShortcutInstalled;
  const setUp = async () => {
    setBusy(true);
    await setupDesktopShortcut(shortcut);
    setBusy(false);
  };
  return (
    <div className="desktop-shortcut">
      {installed ? (
        <p>
          <Check size={14} aria-hidden="true" /> The shortcut is registered in
          GNOME settings and works in every app.
        </p>
      ) : (
        <p>
          GNOME on Wayland only runs shortcuts from its own keyboard settings.
          DictaFox can add one there for you.
        </p>
      )}
      <Button
        variant={installed ? "ghost" : "primary"}
        size="sm"
        onClick={setUp}
        disabled={busy}
        icon={<Keyboard size={14} />}
      >
        {installed ? "Update GNOME shortcut" : "Set up GNOME shortcut"}
      </Button>
    </div>
  );
}

export function Waveform({
  levels,
  active,
  bars = 52,
}: {
  levels: number[];
  active: boolean;
  bars?: number;
}) {
  const padded = [...levels];
  while (padded.length < bars) padded.unshift(0);
  const data = padded.slice(-bars);
  return (
    <div
      className={`waveform${active ? " is-active" : ""}`}
      aria-hidden="true"
      data-testid="waveform"
    >
      {data.map((level, index) => (
        <span
          key={index}
          className="waveform-bar"
          style={{ transform: `scaleY(${Math.max(0.05, level).toFixed(3)})` }}
        />
      ))}
    </div>
  );
}

export function MicTester({
  devices,
  device,
  onDeviceChange,
}: {
  devices: AudioDevice[];
  device: string | null;
  onDeviceChange?: (device: string | null) => void;
}) {
  const [state, setState] = useState<MicTestState | null>(null);
  const [result, setResult] = useState<MicTestResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const busy = useRef(false);

  const stopPolling = () => {
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = undefined;
  };

  const active = useRef(false);
  active.current = Boolean(state?.active);

  // Leaving the page mid-test must release the microphone.
  useEffect(
    () => () => {
      stopPolling();
      if (active.current) void api.stopMicTest().catch(() => {});
    },
    [],
  );

  const poll = () => {
    timer.current = window.setTimeout(async () => {
      const next = await api.micTestState().catch(() => null);
      if (next?.active) {
        setState(next);
        poll();
      }
    }, 100);
  };

  const start = async () => {
    if (busy.current) return;
    busy.current = true;
    setError(null);
    setResult(null);
    try {
      const started = await api.startMicTest(device);
      setState(started);
      poll();
    } catch (raw) {
      setError(toAppError(raw).message);
    } finally {
      busy.current = false;
    }
  };

  const stop = async () => {
    if (busy.current) return;
    busy.current = true;
    stopPolling();
    try {
      const stopped = await api.stopMicTest();
      setResult(stopped);
      setState(null);
    } catch (raw) {
      setError(toAppError(raw).message);
    } finally {
      busy.current = false;
    }
  };

  const level = state?.level ?? 0;

  return (
    <div className="mic-tester">
      {onDeviceChange ? (
        <label className="mic-select">
          <span className="field-label">Input device</span>
          <select
            value={device ?? ""}
            onChange={(event) =>
              onDeviceChange(event.target.value || null)
            }
          >
            <option value="">System default</option>
            {devices
              .filter((entry) => !entry.isMonitor)
              .map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                  {entry.isDefault ? " (default)" : ""}
                </option>
              ))}
          </select>
        </label>
      ) : null}
      <div className="mic-test-surface">
        <div className="level-meter" aria-hidden="true">
          <span style={{ transform: `scaleX(${level.toFixed(3)})` }} />
        </div>
        <div className="mic-test-actions">
          <Button
            variant={state?.active ? "danger" : "secondary"}
            size="sm"
            onClick={state?.active ? stop : start}
            icon={state?.active ? <Square size={14} /> : <Mic size={15} />}
          >
            {state?.active
              ? `Stop test (${formatDuration(state.durationMs)})`
              : "Test microphone"}
          </Button>
          {state?.active ? (
            <span className="mic-test-live">
              Listening — say a few words
            </span>
          ) : null}
        </div>
        {result ? (
          <p
            className={`mic-test-result ${result.success ? "is-success" : "is-error"}`}
          >
            {result.success
              ? `Microphone captured ${formatDuration(result.durationMs)} of audio (peak ${Math.round(result.peak * 100)}%).`
              : result.message}
          </p>
        ) : null}
        {error ? <p className="mic-test-result is-error">{error}</p> : null}
      </div>
    </div>
  );
}

const MODIFIER_KEYS = new Set([
  "Control",
  "Shift",
  "Alt",
  "Meta",
  "CapsLock",
  "OS",
]);

function normalizeKey(event: KeyboardEvent<HTMLDivElement>): string | null {
  const { code, key } = event;
  if (/^Key[A-Z]$/.test(code)) return code.replace("Key", "");
  if (/^Digit[0-9]$/.test(code)) return code.replace("Digit", "");
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  const named: Record<string, string> = {
    Space: "Space",
    Enter: "Enter",
    NumpadEnter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Minus: "Minus",
    Equal: "Equal",
    BracketLeft: "BracketLeft",
    BracketRight: "BracketRight",
    Backslash: "Backslash",
    Semicolon: "Semicolon",
    Quote: "Quote",
    Comma: "Comma",
    Period: "Period",
    Slash: "Slash",
    Backquote: "Backquote",
  };
  if (named[code]) return named[code];
  if (key && key.length === 1) return key.toUpperCase();
  return null;
}

export function ShortcutRecorder({
  value,
  onChange,
  label = "Shortcut",
  hint,
}: {
  value: string;
  onChange: (value: string) => void;
  label?: string;
  hint?: string;
}) {
  const [capturing, setCapturing] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      setCapturing(false);
      setLocalError(null);
      return;
    }
    if (MODIFIER_KEYS.has(event.key)) return;
    const parts: string[] = [];
    if (event.ctrlKey) parts.push("Ctrl");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");
    if (event.metaKey) parts.push(platformInfo().superKey);
    const key = normalizeKey(event);
    if (!key) {
      setLocalError("That key is not supported. Use a letter, digit, F-key or Space.");
      return;
    }
    if (parts.length === 0 && !/^F[0-9]{1,2}$/.test(key)) {
      const { isMac, superKey } = platformInfo();
      setLocalError(`Add Ctrl, ${isMac ? "Option" : "Alt"}, Shift or ${superKey} to avoid typing the shortcut.`);
      return;
    }
    setLocalError(null);
    setCapturing(false);
    onChange([...parts, key].join("+"));
  };

  return (
    <div className="shortcut-field">
      <span className="field-label">{label}</span>
      <div
        className={`shortcut-capture${capturing ? " is-capturing" : ""}`}
        role="button"
        tabIndex={0}
        aria-label={`Set ${label}. Current value ${value}`}
        onClick={() => {
          setCapturing(true);
          setLocalError(null);
        }}
        onKeyDown={(event) => {
          if (!capturing && (event.key === "Enter" || event.key === " ")) {
            event.preventDefault();
            setCapturing(true);
            return;
          }
          if (capturing) onKeyDown(event);
        }}
        onBlur={() => setCapturing(false)}
      >
        {capturing ? (
          <span className="shortcut-listening">Press your shortcut…</span>
        ) : (
          <kbd>{value || "Not set"}</kbd>
        )}
        <span className="shortcut-edit">{capturing ? "Listening" : "Change"}</span>
      </div>
      {localError ? (
        <span className="field-hint is-error">{localError}</span>
      ) : hint ? (
        <span className="field-hint">{hint}</span>
      ) : null}
    </div>
  );
}
