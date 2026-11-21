import type { Profile, RefinementMode } from "../types";

export const PROFILE_LABELS: Record<Profile, string> = {
  general: "General",
  coding: "Coding",
  email: "Email",
  messaging: "Messaging",
  academic: "Academic",
  professional: "Professional",
};

export const PROFILE_DESCRIPTIONS: Record<Profile, string> = {
  general: "Balanced cleanup for everyday dictation.",
  coding: "Preserves identifiers, paths and commands.",
  email: "Cleans prose and keeps paragraphs tidy.",
  messaging: "Light cleanup that keeps a conversational tone.",
  academic: "Careful punctuation for notes and research.",
  professional: "A more formal tone for workplace writing.",
};

export const REFINEMENT_LABELS: Record<RefinementMode, string> = {
  profile: "Follow profile",
  raw: "Raw transcript",
  clean: "Clean",
  professional: "Professional",
  casual: "Casual",
  email: "Email",
  technical: "Technical",
  developer: "Developer",
};

export const REFINEMENT_DESCRIPTIONS: Record<RefinementMode, string> = {
  profile: "Use the cleanup that matches the active profile.",
  raw: "Whisper output with no cleanup at all.",
  clean: "Remove filler words and fix punctuation.",
  professional: "Formal wording without changing the facts.",
  casual: "Keep contractions and a relaxed tone.",
  email: "Paragraph-friendly formatting for messages.",
  technical: "Preserve technical terms and code identifiers.",
  developer: "Minimal changes; never adds punctuation to commands.",
};

export const LANGUAGES: { value: string; label: string }[] = [
  { value: "auto", label: "Detect automatically" },
  { value: "en", label: "English" },
  { value: "ur", label: "Urdu" },
  { value: "hi", label: "Hindi" },
  { value: "ar", label: "Arabic" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "pt", label: "Portuguese" },
  { value: "it", label: "Italian" },
  { value: "nl", label: "Dutch" },
  { value: "tr", label: "Turkish" },
  { value: "zh", label: "Chinese" },
  { value: "ja", label: "Japanese" },
  { value: "ko", label: "Korean" },
  { value: "ru", label: "Russian" },
];

export function languageLabel(value: string): string {
  return LANGUAGES.find((language) => language.value === value)?.label ?? value;
}

export function formatDuration(ms: number): string {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 MB";
  const mb = bytes / 1_048_576;
  if (mb >= 1024) return `${(mb / 1024).toFixed(2)} GB`;
  if (mb >= 100) return `${Math.round(mb)} MB`;
  return `${mb.toFixed(1)} MB`;
}

export function formatRelativeTime(timestamp: number, now = Date.now()): string {
  if (!timestamp) return "never";
  const diff = Math.max(0, now - timestamp);
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hr ago`;
  const days = Math.floor(hours / 24);
  if (days === 1) return "yesterday";
  if (days < 7) return `${days} days ago`;
  return new Date(timestamp).toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

export function formatClock(timestamp: number): string {
  if (!timestamp) return "";
  return new Date(timestamp).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function truncate(text: string, max = 160): string {
  const clean = text.replace(/\s+/g, " ").trim();
  if (clean.length <= max) return clean;
  return `${clean.slice(0, max - 1).trimEnd()}…`;
}

export function percent(downloaded: number, total: number): number {
  if (!total || total <= 0) return 0;
  return Math.min(100, Math.max(0, Math.round((downloaded / total) * 100)));
}
