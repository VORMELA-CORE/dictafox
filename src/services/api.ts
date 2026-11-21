import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AudioDevice,
  Bootstrap,
  Capabilities,
  DictationResult,
  DictionaryEntry,
  DictionaryInput,
  EngineStatus,
  HistoryEntry,
  InsertionOutcome,
  MicTestResult,
  MicTestState,
  ModelInfo,
  Profile,
  RecordingSnapshot,
  RefineOutcome,
  SaveSettingsOutcome,
  Settings,
  Verification,
} from "../types";

export function toAppError(error: unknown): AppError {
  if (typeof error === "object" && error !== null && "message" in error) {
    const candidate = error as Partial<AppError>;
    if (typeof candidate.message === "string") {
      return {
        code: candidate.code ?? "unknown_error",
        message: candidate.message,
        hint: candidate.hint ?? null,
        detail: candidate.detail ?? null,
      };
    }
  }
  if (typeof error === "string") {
    return { code: "unknown_error", message: error };
  }
  return {
    code: "unknown_error",
    message: "DictaFox could not complete that action.",
    hint: "Try again. If the problem continues, open Settings → Developer → Diagnostics.",
  };
}

export const api = {
  bootstrap: () => invoke<Bootstrap>("get_bootstrap"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<SaveSettingsOutcome>("save_settings", { settings }),
  setProfile: (profile: Profile) => invoke<Settings>("set_profile", { profile }),
  capabilities: () => invoke<Capabilities>("get_capabilities"),
  recordingState: () => invoke<RecordingSnapshot>("get_recording_state"),
  engineStatus: () => invoke<EngineStatus>("get_engine_status"),
  lastResult: () => invoke<DictationResult | null>("get_last_result"),
  startRecording: (origin: string) =>
    invoke<RecordingSnapshot>("start_recording", { origin }),
  stopRecording: () => invoke<DictationResult>("stop_recording"),
  cancelRecording: () => invoke<boolean>("cancel_recording"),
  insertText: (text: string) => invoke<InsertionOutcome>("insert_text", { text }),
  copyText: (text: string) => invoke<string>("copy_text", { text }),
  refinePreview: (text: string) =>
    invoke<RefineOutcome>("refine_preview", { text }),
  listHistory: (search: string | null, limit = 100) =>
    invoke<HistoryEntry[]>("list_history", { search, limit }),
  deleteHistory: (id: number) => invoke<boolean>("delete_history", { id }),
  clearHistory: () => invoke<number>("clear_history"),
  listDictionary: () => invoke<DictionaryEntry[]>("list_dictionary"),
  upsertDictionary: (entry: DictionaryInput) =>
    invoke<DictionaryEntry>("upsert_dictionary", { entry }),
  deleteDictionary: (id: number) =>
    invoke<boolean>("delete_dictionary", { id }),
  listDevices: () => invoke<AudioDevice[]>("list_devices"),
  listModels: () => invoke<ModelInfo[]>("list_models"),
  downloadModel: (modelId: string) =>
    invoke<void>("download_model", { modelId }),
  cancelModelDownload: (modelId: string) =>
    invoke<boolean>("cancel_model_download", { modelId }),
  deleteModel: (modelId: string) => invoke<void>("delete_model", { modelId }),
  verifyModel: (modelId: string) =>
    invoke<Verification>("verify_model", { modelId }),
  selectModel: (modelId: string) =>
    invoke<Settings>("select_model", { modelId }),
  startMicTest: (device: string | null) =>
    invoke<MicTestState>("start_mic_test", { device }),
  micTestState: () => invoke<MicTestState>("mic_test_state"),
  stopMicTest: () => invoke<MicTestResult>("stop_mic_test"),
  validateShortcut: (shortcut: string) =>
    invoke<string>("validate_shortcut", { shortcut }),
  setupDesktopShortcut: (shortcut: string | null) =>
    invoke<Capabilities>("setup_desktop_shortcut", { shortcut }),
  diagnostics: () => invoke<string>("get_diagnostics"),
  openDataDir: () => invoke<void>("open_data_dir"),
  openModelsDir: () => invoke<void>("open_models_dir"),
  quit: () => invoke<void>("quit_app"),
};
