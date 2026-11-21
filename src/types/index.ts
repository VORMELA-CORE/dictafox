export type ShortcutMode = "pushToTalk" | "toggle";
export type Theme = "dark" | "light" | "system";
export type InsertionMode = "auto" | "clipboardOnly";
export type Profile =
  | "general"
  | "coding"
  | "email"
  | "messaging"
  | "academic"
  | "professional";
export type RefinementMode =
  | "profile"
  | "raw"
  | "clean"
  | "professional"
  | "casual"
  | "email"
  | "technical"
  | "developer";
export type RecordingState = "idle" | "recording" | "transcribing";

export interface ProviderSettings {
  enabled: boolean;
  baseUrl: string;
  model: string;
  apiKey: string | null;
  timeoutSeconds: number;
}

export interface InsertionSettings {
  mode: InsertionMode;
  restoreClipboard: boolean;
  pasteDelayMs: number;
}

export interface Settings {
  onboardingComplete: boolean;
  shortcut: string;
  shortcutMode: ShortcutMode;
  microphone: string | null;
  maxRecordingSeconds: number;
  vadEnabled: boolean;
  inputGain: number;
  model: string;
  customModelPath: string | null;
  language: string;
  threads: number;
  keepModelLoaded: boolean;
  profile: Profile;
  refinement: RefinementMode;
  dictionaryEnabled: boolean;
  provider: ProviderSettings;
  persistHistory: boolean;
  saveAudio: boolean;
  theme: Theme;
  reduceMotion: boolean;
  insertion: InsertionSettings;
  developerMode: boolean;
}

export interface AppError {
  code: string;
  message: string;
  hint?: string | null;
  detail?: string | null;
}

export interface AudioDevice {
  id: string;
  name: string;
  isMonitor: boolean;
  isDefault: boolean;
}

export interface DependencyStatus {
  name: string;
  installed: boolean;
  required: boolean;
  purpose: string;
  installHint: string;
}

export interface Limitation {
  code: string;
  title: string;
  description: string;
  hint: string;
}

export interface Capabilities {
  version: string;
  os: "linux" | "windows" | "macos";
  osLabel: string;
  sessionType: string;
  desktop: string;
  displayServer: string;
  audioBackend: string | null;
  microphoneCount: number;
  defaultMicrophone: string | null;
  pasteMethod: string;
  globalShortcutSupported: boolean;
  /** GNOME Wayland: DictaFox can add its shortcut to GNOME settings. */
  desktopShortcutAvailable: boolean;
  desktopShortcutInstalled: boolean;
  engine: string;
  dataDir: string;
  modelsDir: string;
  logFile: string;
  historyCount: number;
  activeModel: string;
  modelInstalled: boolean;
  diskFreeGb: number | null;
  dependencies: DependencyStatus[];
  limitations: Limitation[];
}

export interface ModelInfo {
  id: string;
  label: string;
  path: string;
  sizeBytes: number;
  multilingual: boolean;
  note: string;
  recommendedRamGb: number;
  installed: boolean;
  active: boolean;
  filePresent: boolean;
}

export interface HistoryEntry {
  id: number;
  text: string;
  rawText: string | null;
  profile: Profile;
  refinement: RefinementMode;
  model: string;
  language: string;
  durationMs: number;
  createdAt: number;
}

export interface DictionaryEntry {
  id: number;
  spoken: string;
  written: string;
  category: string | null;
  caseSensitive: boolean;
  enabled: boolean;
}

export interface DictionaryInput {
  id: number | null;
  spoken: string;
  written: string;
  category: string | null;
  caseSensitive: boolean;
  enabled: boolean;
}

export interface RecordingSnapshot {
  state: RecordingState;
  level: number;
  durationMs: number;
  maxSeconds: number;
  origin: string | null;
  microphone: string | null;
}

export interface InsertionOutcome {
  method: string;
  clipboard: boolean;
  pasted: boolean;
  warning: string | null;
}

export interface DictationResult {
  text: string;
  rawText: string;
  mode: RefinementMode;
  profile: Profile;
  model: string;
  language: string;
  durationMs: number;
  audioMs: number;
  transcriptionMs: number;
  inserted: boolean;
  insertion: InsertionOutcome | null;
  warning: string | null;
  historyId: number | null;
  createdAt: number;
  savedAudio: string | null;
}

export interface EngineStatus {
  engine: string;
  loadedModel: string | null;
  keepModelLoaded: boolean;
  threads: number;
}

export interface Bootstrap {
  settings: Settings;
  capabilities: Capabilities;
  devices: AudioDevice[];
  audioError: AppError | null;
  models: ModelInfo[];
  dictionary: DictionaryEntry[];
  history: HistoryEntry[];
  lastResult: DictationResult | null;
  engine: EngineStatus;
  recording: RecordingSnapshot;
}

export interface DownloadProgress {
  modelId: string;
  downloaded: number;
  total: number;
  phase: "downloading" | "verifying" | "done";
}

export interface Verification {
  valid: boolean;
  sizeOk: boolean;
  hashOk: boolean;
  headerOk: boolean;
  message: string;
}

export interface MicTestState {
  active: boolean;
  level: number;
  peak: number;
  durationMs: number;
}

export interface MicTestResult {
  success: boolean;
  durationMs: number;
  peak: number;
  message: string;
}

export interface RefineOutcome {
  text: string;
  mode: RefinementMode;
  provider: string | null;
  warning: string | null;
}

export interface SaveSettingsOutcome {
  settings: Settings;
  shortcutWarning: string | null;
}

export interface ToastMessage {
  id: number;
  kind: "info" | "success" | "warning" | "error";
  message: string;
}
