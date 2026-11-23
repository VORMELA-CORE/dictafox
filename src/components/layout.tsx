import {
  BookOpen,
  History,
  Info,
  LayoutDashboard,
  Mic,
  Settings,
  SlidersHorizontal,
} from "lucide-react";
import type { Route } from "../hooks/useApp";
import { platformInfo } from "../utils/platform";
import { useApp } from "../hooks/useApp";
import { PROFILE_LABELS } from "../utils/format";
import { AppIcon } from "./brand";

const NAV_ITEMS: { route: Route; label: string; icon: typeof Mic }[] = [
  { route: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { route: "history", label: "History", icon: History },
  { route: "dictionary", label: "Dictionary", icon: BookOpen },
  { route: "models", label: "Models", icon: SlidersHorizontal },
  { route: "settings", label: "Settings", icon: Settings },
  { route: "about", label: "About", icon: Info },
];

export function Sidebar() {
  const { route, navigate, recording, settings, result } = useApp();
  const stateLabel =
    recording.state === "recording"
      ? "Recording"
      : recording.state === "transcribing"
        ? "Transcribing"
        : "Ready";

  return (
    <aside className="sidebar">
      <button
        type="button"
        className="brand"
        onClick={() => navigate("dashboard")}
        aria-label="DictaFox dashboard"
      >
        <AppIcon size={34} />
        <span className="brand-text">
          <span className="brand-name">DictaFox</span>
          <span className="brand-tag">by Vormela Core</span>
        </span>
      </button>

      <nav className="nav" aria-label="Main navigation">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          const active = route === item.route;
          return (
            <button
              key={item.route}
              type="button"
              className={`nav-item${active ? " is-active" : ""}`}
              onClick={() => navigate(item.route)}
              aria-current={active ? "page" : undefined}
            >
              <Icon size={17} strokeWidth={1.8} aria-hidden="true" />
              <span>{item.label}</span>
              {item.route === "history" && result ? (
                <span className="nav-dot" aria-hidden="true" />
              ) : null}
            </button>
          );
        })}
      </nav>

      <div className="sidebar-footer">
        <div className={`live-status is-${recording.state}`}>
          <span className="live-dot" aria-hidden="true" />
          <div>
            <span className="live-label">{stateLabel}</span>
            <span className="live-detail">
              {settings ? PROFILE_LABELS[settings.profile] : "Local"} ·{" "}
              {settings?.shortcut ?? platformInfo().defaultShortcut}
            </span>
          </div>
        </div>
        <p className="sidebar-note">
          <Mic size={13} aria-hidden="true" /> Audio stays on this device.
        </p>
        <p className="sidebar-credit">© 2026 Vormela Core</p>
      </div>
    </aside>
  );
}
