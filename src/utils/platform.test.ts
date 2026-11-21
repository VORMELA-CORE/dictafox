import { describe, expect, it } from "vitest";
import { platformInfo } from "./platform";

describe("platformInfo", () => {
  it("prefers the OS reported by the backend", () => {
    const info = platformInfo({ os: "macos", sessionType: "macos" }, "X11; Linux");
    expect(info.isMac).toBe(true);
    expect(info.pasteKeys).toBe("Cmd+V");
    expect(info.defaultShortcut).toBe("Alt+Space");
  });

  it("detects Wayland only on Linux", () => {
    expect(platformInfo({ os: "linux", sessionType: "wayland" }).wayland).toBe(true);
    expect(platformInfo({ os: "linux", sessionType: "x11" }).wayland).toBe(false);
    expect(platformInfo({ os: "windows", sessionType: "wayland" }).wayland).toBe(false);
  });

  it("falls back to the user agent before capabilities load", () => {
    expect(platformInfo(null, "Mozilla/5.0 (Windows NT 10.0; Win64; x64)").os).toBe("windows");
    expect(platformInfo(null, "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)").os).toBe("macos");
    expect(platformInfo(null, "Mozilla/5.0 (X11; Linux x86_64)").os).toBe("linux");
  });
});
