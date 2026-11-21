import { useEffect } from "react";
import { Sidebar } from "./components/layout";
import { ErrorBanner, FoxMark, Toasts } from "./components/ui";
import { useApp } from "./hooks/useApp";
import { About } from "./pages/About";
import { Dashboard } from "./pages/Dashboard";
import { Dictionary } from "./pages/Dictionary";
import { History } from "./pages/History";
import { Models } from "./pages/Models";
import { Onboarding } from "./pages/Onboarding";
import { SettingsPage } from "./pages/Settings";

function resolveTheme(theme: "dark" | "light" | "system"): "dark" | "light" {
  if (theme !== "system") return theme;
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function App() {
  const {
    loading,
    fatal,
    bootstrap,
    settings,
    route,
    toasts,
    dismissToast,
    reload,
    error,
    dismissError,
  } = useApp();

  useEffect(() => {
    if (!settings) return;
    const apply = () => {
      document.documentElement.dataset.theme = resolveTheme(settings.theme);
    };
    apply();
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [settings]);

  useEffect(() => {
    document.documentElement.classList.toggle(
      "reduce-motion",
      settings?.reduceMotion ?? false,
    );
  }, [settings?.reduceMotion]);

  if (loading) {
    return (
      <div className="boot-screen">
        <FoxMark size={64} />
        <div className="boot-copy">
          <h1>DictaFox</h1>
          <p>Loading your local dictation workspace…</p>
        </div>
      </div>
    );
  }

  if (fatal || !bootstrap || !settings) {
    return (
      <div className="boot-screen">
        <FoxMark size={56} />
        <div className="boot-copy">
          <h1>DictaFox could not start</h1>
          <p>
            {fatal?.message ??
              "The native backend did not respond. Run DictaFox as a desktop app, not in a browser tab."}
          </p>
          {fatal?.hint ? <p className="muted">{fatal.hint}</p> : null}
          <button type="button" className="btn btn-primary" onClick={reload}>
            Try again
          </button>
        </div>
      </div>
    );
  }

  if (!settings.onboardingComplete) {
    return (
      <>
        <Onboarding />
        <Toasts toasts={toasts} onDismiss={dismissToast} />
      </>
    );
  }

  return (
    <div className="shell">
      <Sidebar />
      <main className="main" id="main-content">
        {error ? (
          <div className="global-error">
            <ErrorBanner error={error} onDismiss={dismissError} />
          </div>
        ) : null}
        {route === "dashboard" ? <Dashboard /> : null}
        {route === "history" ? <History /> : null}
        {route === "dictionary" ? <Dictionary /> : null}
        {route === "models" ? <Models /> : null}
        {route === "settings" ? <SettingsPage /> : null}
        {route === "about" ? <About /> : null}
      </main>
      <Toasts toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
