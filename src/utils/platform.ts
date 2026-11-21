import type { Capabilities } from "../types";

export type OsKind = "linux" | "windows" | "macos";

export interface PlatformInfo {
  os: OsKind;
  label: string;
  isLinux: boolean;
  isMac: boolean;
  isWindows: boolean;
  /** Linux Wayland session, where global key grabs and paste are restricted. */
  wayland: boolean;
  /** Keys that paste in the focused app. */
  pasteKeys: string;
  /** Name of the Super/Meta modifier on this OS. */
  superKey: string;
  /** Default dictation shortcut for this OS. */
  defaultShortcut: string;
}

function guessOs(userAgent: string): OsKind {
  if (/Mac OS X|Macintosh/i.test(userAgent)) return "macos";
  if (/Windows/i.test(userAgent)) return "windows";
  return "linux";
}

function normaliseOs(value: string | undefined): OsKind | null {
  if (value === "linux" || value === "windows" || value === "macos") return value;
  return null;
}

/** Describe the OS DictaFox runs on. Before capabilities load, the webview's
 * user agent is a reliable enough guess. */
export function platformInfo(
  capabilities?: Pick<Capabilities, "os" | "sessionType"> | null,
  userAgent: string = typeof navigator === "undefined" ? "" : navigator.userAgent,
): PlatformInfo {
  const os = normaliseOs(capabilities?.os) ?? guessOs(userAgent);
  const isMac = os === "macos";
  const isWindows = os === "windows";
  const isLinux = os === "linux";
  return {
    os,
    label: isMac ? "macOS" : isWindows ? "Windows" : "Linux",
    isLinux,
    isMac,
    isWindows,
    wayland: isLinux && capabilities?.sessionType?.toLowerCase() === "wayland",
    pasteKeys: isMac ? "Cmd+V" : "Ctrl+V",
    superKey: isMac ? "Cmd" : isWindows ? "Win" : "Super",
    defaultShortcut: isMac ? "Alt+Space" : "Ctrl+Space",
  };
}
