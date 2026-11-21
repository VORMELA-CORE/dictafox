import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatDuration,
  formatRelativeTime,
  languageLabel,
  percent,
  truncate,
} from "./format";

describe("formatDuration", () => {
  it("formats seconds and minutes", () => {
    expect(formatDuration(0)).toBe("0:00");
    expect(formatDuration(8_400)).toBe("0:08");
    expect(formatDuration(65_000)).toBe("1:05");
    expect(formatDuration(723_000)).toBe("12:03");
  });

  it("never renders negative time", () => {
    expect(formatDuration(-10)).toBe("0:00");
  });
});

describe("formatBytes", () => {
  it("uses MB and GB sensibly", () => {
    expect(formatBytes(77_704_715)).toBe("74.1 MB");
    expect(formatBytes(1_624_555_275)).toBe("1.51 GB");
    expect(formatBytes(0)).toBe("0 MB");
  });
});

describe("formatRelativeTime", () => {
  const now = 1_800_000_000_000;
  it("describes recent timestamps", () => {
    expect(formatRelativeTime(now - 1_000, now)).toBe("just now");
    expect(formatRelativeTime(now - 120_000, now)).toBe("2 min ago");
    expect(formatRelativeTime(now - 7_200_000, now)).toBe("2 hr ago");
    expect(formatRelativeTime(now - 86_400_000, now)).toBe("yesterday");
  });

  it("handles missing timestamps", () => {
    expect(formatRelativeTime(0, now)).toBe("never");
  });
});

describe("misc helpers", () => {
  it("truncates with an ellipsis", () => {
    expect(truncate("hello world", 8)).toBe("hello w…");
    expect(truncate("short", 20)).toBe("short");
  });

  it("calculates download percentages safely", () => {
    expect(percent(50, 200)).toBe(25);
    expect(percent(0, 0)).toBe(0);
    expect(percent(300, 200)).toBe(100);
  });

  it("maps language codes to labels", () => {
    expect(languageLabel("ur")).toBe("Urdu");
    expect(languageLabel("xx")).toBe("xx");
  });
});
