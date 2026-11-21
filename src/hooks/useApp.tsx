import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api, toAppError } from "../services/api";
import type {
  AppError,
  AudioDevice,
  Bootstrap,
  Capabilities,
  DictationResult,
  DictionaryEntry,
  DictionaryInput,
  DownloadProgress,
  HistoryEntry,
  InsertionOutcome,
  MicTestResult,
  MicTestState,
  ModelInfo,
  Profile,
  RecordingSnapshot,
  RecordingState,
  Settings,
  ToastMessage,
  Verification,
} from "../types";

export type Route =
  | "dashboard"
  | "history"
  | "dictionary"
  | "models"
  | "settings"
  | "about";

const IDLE_RECORDING: RecordingSnapshot = {
  state: "idle",
  level: 0,
  durationMs: 0,
  maxSeconds: 120,
  origin: null,
  microphone: null,
};

export interface AppContextValue {
  bootstrap: Bootstrap | null;
  settings: Settings | null;
  capabilities: Capabilities | null;
  devices: AudioDevice[];
  models: ModelInfo[];
  dictionary: DictionaryEntry[];
  history: HistoryEntry[];
  recording: RecordingSnapshot;
  levels: number[];
  result: DictationResult | null;
  modelProgress: Record<string, DownloadProgress>;
  route: Route;
  toasts: ToastMessage[];
  error: AppError | null;
  shortcutWarning: string | null;
  loading: boolean;
  fatal: AppError | null;
  navigate: (route: Route) => void;
  reload: () => Promise<void>;
  saveSettings: (settings: Settings) => Promise<Settings | null>;
  setProfile: (profile: Profile) => Promise<void>;
  startRecording: (origin?: string) => Promise<void>;
  stopRecording: () => Promise<DictationResult | null>;
  cancelRecording: () => Promise<void>;
  copyText: (text: string) => Promise<boolean>;
  insertText: (text: string) => Promise<InsertionOutcome | null>;
  deleteHistory: (id: number) => Promise<void>;
  clearHistory: () => Promise<void>;
  refreshHistory: (search?: string) => Promise<HistoryEntry[]>;
  refreshDevices: () => Promise<void>;
  refreshModels: () => Promise<void>;
  upsertDictionary: (entry: DictionaryInput) => Promise<DictionaryEntry | null>;
  deleteDictionary: (id: number) => Promise<void>;
  downloadModel: (modelId: string) => Promise<void>;
  cancelModelDownload: (modelId: string) => Promise<void>;
  deleteModel: (modelId: string) => Promise<void>;
  verifyModel: (modelId: string) => Promise<Verification | null>;
  selectModel: (modelId: string) => Promise<boolean>;
  startMicTest: (device: string | null) => Promise<MicTestState | null>;
  micTestState: () => Promise<MicTestState | null>;
  stopMicTest: () => Promise<MicTestResult | null>;
  validateShortcut: (shortcut: string) => Promise<string | null>;
  setupDesktopShortcut: (shortcut: string | null) => Promise<boolean>;
  diagnostics: () => Promise<string | null>;
  openDataDir: () => Promise<void>;
  openModelsDir: () => Promise<void>;
  notify: (kind: ToastMessage["kind"], message: string) => void;
  dismissToast: (id: number) => void;
  dismissError: () => void;
  dismissShortcutWarning: () => void;
  quit: () => Promise<void>;
}

const AppContext = createContext<AppContextValue | null>(null);

const ROUTES: Route[] = [
  "dashboard",
  "history",
  "dictionary",
  "models",
  "settings",
  "about",
];

function isRoute(value: string): value is Route {
  return (ROUTES as string[]).includes(value);
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [recording, setRecording] = useState<RecordingSnapshot>(IDLE_RECORDING);
  const [levels, setLevels] = useState<number[]>([]);
  const [result, setResult] = useState<DictationResult | null>(null);
  const [modelProgress, setModelProgress] = useState<
    Record<string, DownloadProgress>
  >({});
  const [route, setRoute] = useState<Route>("dashboard");
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const [error, setError] = useState<AppError | null>(null);
  const [shortcutWarning, setShortcutWarning] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [fatal, setFatal] = useState<AppError | null>(null);

  const toastId = useRef(0);
  const recordingRef = useRef(recording);
  recordingRef.current = recording;

  const notify = useCallback(
    (kind: ToastMessage["kind"], message: string) => {
      const id = ++toastId.current;
      setToasts((current) => [...current, { id, kind, message }]);
      window.setTimeout(() => {
        setToasts((current) => current.filter((toast) => toast.id !== id));
      }, 6500);
    },
    [],
  );

  const dismissToast = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const handleError = useCallback(
    (raw: unknown) => {
      const err = toAppError(raw);
      setError(err);
      notify("error", err.message);
      return err;
    },
    [notify],
  );

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.bootstrap();
      setBootstrap(data);
      setRecording(data.recording);
      setResult(data.lastResult);
      setFatal(null);
    } catch (raw) {
      setFatal(toAppError(raw));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const refreshHistory = useCallback(async (search?: string) => {
    try {
      const entries = await api.listHistory(search ?? null, 100);
      if (!search) {
        setBootstrap((current) =>
          current ? { ...current, history: entries } : current,
        );
      }
      return entries;
    } catch (raw) {
      handleError(raw);
      return [];
    }
  }, [handleError]);

  const refreshModels = useCallback(async () => {
    try {
      const models = await api.listModels();
      setBootstrap((current) =>
        current ? { ...current, models } : current,
      );
    } catch (raw) {
      handleError(raw);
    }
  }, [handleError]);

  const refreshDevices = useCallback(async () => {
    try {
      const devices = await api.listDevices();
      setBootstrap((current) =>
        current ? { ...current, devices, audioError: null } : current,
      );
    } catch (raw) {
      const err = toAppError(raw);
      setBootstrap((current) =>
        current ? { ...current, devices: [], audioError: err } : current,
      );
      handleError(err);
    }
  }, [handleError]);

  // Backend events: dictation state, results, downloads and tray navigation.
  useEffect(() => {
    const subscriptions: Promise<UnlistenFn>[] = [
      // Some backend paths emit only `{ state: "transcribing" }`, so merge
      // instead of replacing to keep level/duration/maxSeconds defined.
      listen<Partial<RecordingSnapshot>>("dictafox://state", (event) => {
        setRecording((current) => ({ ...current, ...event.payload }));
        if (event.payload.state !== "recording") {
          setLevels([]);
        }
      }),
      listen<DictationResult>("dictafox://result", (event) => {
        setResult(event.payload);
        setRecording(IDLE_RECORDING);
        void refreshHistory();
      }),
      listen<AppError>("dictafox://error", (event) => {
        setError(event.payload);
      }),
      listen<{ kind: ToastMessage["kind"]; message: string }>(
        "dictafox://toast",
        (event) => {
          notify(event.payload.kind, event.payload.message);
        },
      ),
      listen<Settings>("dictafox://settings", (event) => {
        setBootstrap((current) =>
          current ? { ...current, settings: event.payload } : current,
        );
      }),
      listen<DownloadProgress>("dictafox://model-progress", (event) => {
        setModelProgress((current) => ({
          ...current,
          [event.payload.modelId]: event.payload,
        }));
      }),
      listen("dictafox://models-updated", () => {
        void refreshModels();
      }),
      listen<string>("dictafox://navigate", (event) => {
        if (isRoute(event.payload)) setRoute(event.payload);
      }),
      listen<string>("dictafox://shortcut-warning", (event) => {
        setShortcutWarning(event.payload);
        notify("warning", event.payload);
      }),
    ];
    return () => {
      subscriptions.forEach((subscription) => {
        void subscription.then((unlisten) => unlisten()).catch(() => {});
      });
    };
  }, [notify, refreshHistory, refreshModels]);

  // Poll the real capture level while recording; poll slowly when idle so a
  // tray/CLI dictation still appears without burning CPU.
  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;
    const tick = async () => {
      try {
        const snapshot = await api.recordingState();
        if (cancelled) return;
        setRecording(snapshot);
        if (snapshot.state === "recording") {
          setLevels((current) => [
            ...current.slice(-63),
            Math.min(1, Math.max(0, snapshot.level)),
          ]);
        }
      } catch {
        // The backend may be shutting down; the fatal screen handles startup.
      }
      if (!cancelled) {
        const active =
          recordingRef.current.state === "recording" ||
          recordingRef.current.state === "transcribing";
        timer = window.setTimeout(tick, active ? 100 : 750);
      }
    };
    void tick();
    return () => {
      cancelled = true;
      if (timer) window.clearTimeout(timer);
    };
  }, []);

  const saveSettings = useCallback(
    async (next: Settings): Promise<Settings | null> => {
      try {
        const outcome = await api.saveSettings(next);
        setBootstrap((current) =>
          current ? { ...current, settings: outcome.settings } : current,
        );
        if (outcome.shortcutWarning) {
          setShortcutWarning(outcome.shortcutWarning);
          notify("warning", outcome.shortcutWarning);
        }
        return outcome.settings;
      } catch (raw) {
        handleError(raw);
        return null;
      }
    },
    [handleError, notify],
  );

  const setProfile = useCallback(
    async (profile: Profile) => {
      try {
        const settings = await api.setProfile(profile);
        setBootstrap((current) =>
          current ? { ...current, settings } : current,
        );
        notify("success", `Profile set to ${profile}.`);
      } catch (raw) {
        handleError(raw);
      }
    },
    [handleError, notify],
  );

  const startRecording = useCallback(
    async (origin = "ui") => {
      try {
        const snapshot = await api.startRecording(origin);
        setError(null);
        setRecording(snapshot);
        setLevels([]);
      } catch (raw) {
        handleError(raw);
      }
    },
    [handleError],
  );

  const stopRecording = useCallback(async () => {
    setRecording((current) => ({ ...current, state: "transcribing", level: 0 }));
    try {
      const dictation = await api.stopRecording();
      setResult(dictation);
      setRecording(IDLE_RECORDING);
      await refreshHistory();
      return dictation;
    } catch (raw) {
      handleError(raw);
      try {
        setRecording(await api.recordingState());
      } catch {
        setRecording(IDLE_RECORDING);
      }
      return null;
    }
  }, [handleError, refreshHistory]);

  const cancelRecording = useCallback(async () => {
    try {
      await api.cancelRecording();
      setRecording(IDLE_RECORDING);
      setLevels([]);
      notify("info", "Recording discarded.");
    } catch (raw) {
      handleError(raw);
    }
  }, [handleError, notify]);

  const copyText = useCallback(
    async (text: string) => {
      try {
        await api.copyText(text);
        notify("success", "Copied to clipboard.");
        return true;
      } catch (raw) {
        handleError(raw);
        return false;
      }
    },
    [handleError, notify],
  );

  const insertText = useCallback(
    async (text: string) => {
      try {
        const outcome = await api.insertText(text);
        if (outcome.pasted) {
          notify("success", `Inserted with ${outcome.method}.`);
        }
        return outcome;
      } catch (raw) {
        handleError(raw);
        return null;
      }
    },
    [handleError, notify],
  );

  const deleteHistory = useCallback(
    async (id: number) => {
      try {
        await api.deleteHistory(id);
        await refreshHistory();
        notify("info", "Transcript deleted.");
      } catch (raw) {
        handleError(raw);
      }
    },
    [handleError, notify, refreshHistory],
  );

  const clearHistory = useCallback(async () => {
    try {
      const count = await api.clearHistory();
      await refreshHistory();
      notify("info", `Deleted ${count} transcript${count === 1 ? "" : "s"}.`);
    } catch (raw) {
      handleError(raw);
    }
  }, [handleError, notify, refreshHistory]);

  const upsertDictionary = useCallback(
    async (entry: DictionaryInput) => {
      try {
        const saved = await api.upsertDictionary(entry);
        const dictionary = await api.listDictionary();
        setBootstrap((current) =>
          current ? { ...current, dictionary } : current,
        );
        notify("success", `Saved “${saved.written}”.`);
        return saved;
      } catch (raw) {
        handleError(raw);
        return null;
      }
    },
    [handleError, notify],
  );

  const deleteDictionary = useCallback(
    async (id: number) => {
      try {
        await api.deleteDictionary(id);
        const dictionary = await api.listDictionary();
        setBootstrap((current) =>
          current ? { ...current, dictionary } : current,
        );
        notify("info", "Dictionary entry removed.");
      } catch (raw) {
        handleError(raw);
      }
    },
    [handleError, notify],
  );

  const downloadModel = useCallback(
    async (modelId: string) => {
      setModelProgress((current) => ({
        ...current,
        [modelId]: {
          modelId,
          downloaded: 0,
          total: 0,
          phase: "downloading",
        },
      }));
      try {
        await api.downloadModel(modelId);
        await refreshModels();
        setModelProgress((current) => {
          const next = { ...current };
          delete next[modelId];
          return next;
        });
        notify("success", "Model downloaded and verified.");
      } catch (raw) {
        const err = toAppError(raw);
        setModelProgress((current) => {
          const next = { ...current };
          delete next[modelId];
          return next;
        });
        if (err.code !== "download_cancelled") handleError(err);
        else notify("info", "Download cancelled.");
        await refreshModels();
      }
    },
    [handleError, notify, refreshModels],
  );

  const cancelModelDownload = useCallback(
    async (modelId: string) => {
      try {
        await api.cancelModelDownload(modelId);
      } catch (raw) {
        handleError(raw);
      }
    },
    [handleError],
  );

  const deleteModel = useCallback(
    async (modelId: string) => {
      try {
        await api.deleteModel(modelId);
        await refreshModels();
        notify("info", "Model deleted.");
      } catch (raw) {
        handleError(raw);
      }
    },
    [handleError, notify, refreshModels],
  );

  const verifyModel = useCallback(
    async (modelId: string) => {
      try {
        return await api.verifyModel(modelId);
      } catch (raw) {
        handleError(raw);
        return null;
      }
    },
    [handleError],
  );

  const selectModel = useCallback(
    async (modelId: string) => {
      try {
        const settings = await api.selectModel(modelId);
        setBootstrap((current) =>
          current ? { ...current, settings } : current,
        );
        await refreshModels();
        notify("success", "Speech model selected.");
        return true;
      } catch (raw) {
        handleError(raw);
        return false;
      }
    },
    [handleError, notify, refreshModels],
  );

  const startMicTest = useCallback(
    async (device: string | null) => {
      try {
        return await api.startMicTest(device);
      } catch (raw) {
        handleError(raw);
        return null;
      }
    },
    [handleError],
  );

  const micTestState = useCallback(async () => {
    try {
      return await api.micTestState();
    } catch {
      return null;
    }
  }, []);

  const setupDesktopShortcut = useCallback(
    async (shortcut: string | null) => {
      try {
        const capabilities = await api.setupDesktopShortcut(shortcut);
        setBootstrap((current) =>
          current ? { ...current, capabilities } : current,
        );
        notify("success", "GNOME shortcut added. It works in every app now.");
        return true;
      } catch (raw) {
        handleError(raw);
        return false;
      }
    },
    [handleError, notify],
  );

  const stopMicTest = useCallback(async () => {
    try {
      return await api.stopMicTest();
    } catch (raw) {
      handleError(raw);
      return null;
    }
  }, [handleError]);

  const validateShortcut = useCallback(
    async (shortcut: string) => {
      try {
        return await api.validateShortcut(shortcut);
      } catch (raw) {
        const err = toAppError(raw);
        notify("warning", err.message);
        return null;
      }
    },
    [notify],
  );

  const diagnostics = useCallback(async () => {
    try {
      return await api.diagnostics();
    } catch (raw) {
      handleError(raw);
      return null;
    }
  }, [handleError]);

  const openDataDir = useCallback(async () => {
    try {
      await api.openDataDir();
    } catch (raw) {
      handleError(raw);
    }
  }, [handleError]);

  const openModelsDir = useCallback(async () => {
    try {
      await api.openModelsDir();
    } catch (raw) {
      handleError(raw);
    }
  }, [handleError]);

  const navigate = useCallback((next: Route) => setRoute(next), []);

  const quit = useCallback(async () => {
    try {
      await api.quit();
    } catch {
      // The process may exit before the IPC reply arrives.
    }
  }, []);

  const settings = bootstrap?.settings ?? null;

  const value: AppContextValue = {
    bootstrap,
    settings,
    capabilities: bootstrap?.capabilities ?? null,
    devices: bootstrap?.devices ?? [],
    models: bootstrap?.models ?? [],
    dictionary: bootstrap?.dictionary ?? [],
    history: bootstrap?.history ?? [],
    recording,
    levels,
    result,
    modelProgress,
    route,
    toasts,
    error,
    shortcutWarning,
    loading,
    fatal,
    navigate,
    reload: load,
    saveSettings,
    setProfile,
    startRecording,
    stopRecording,
    cancelRecording,
    copyText,
    insertText,
    deleteHistory,
    clearHistory,
    refreshHistory,
    refreshDevices,
    refreshModels,
    upsertDictionary,
    deleteDictionary,
    downloadModel,
    cancelModelDownload,
    deleteModel,
    verifyModel,
    selectModel,
    startMicTest,
    micTestState,
    stopMicTest,
    validateShortcut,
    setupDesktopShortcut,
    diagnostics,
    openDataDir,
    openModelsDir,
    notify,
    dismissToast,
    dismissError: () => setError(null),
    dismissShortcutWarning: () => setShortcutWarning(null),
    quit,
  };

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useApp(): AppContextValue {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error("useApp must be used inside AppProvider");
  }
  return context;
}

export type { RecordingState };
