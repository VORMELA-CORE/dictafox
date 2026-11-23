import { AppIcon, ProductCredit, VormelaLink } from "../components/brand";
import { Badge, Button, LimitationsList } from "../components/ui";
import { useApp } from "../hooks/useApp";
import { platformInfo } from "../utils/platform";
import { ClipboardCopy, FolderOpen, ShieldCheck } from "lucide-react";

export function About() {
  const { capabilities, settings, copyText, openDataDir, quit } = useApp();
  const platform = platformInfo(capabilities);

  return (
    <div className="page about">
      <header className="about-hero">
        <AppIcon size={64} label="DictaFox" />
        <div>
          <h1>DictaFox</h1>
          <p className="tagline">Speak Naturally. Type Everywhere.</p>
          <ProductCredit />
          <div className="chip-row">
            <Badge tone="accent">MIT licensed</Badge>
            <Badge>Version {capabilities?.version ?? "0.1.0"}</Badge>
            <Badge>Local first</Badge>
          </div>
        </div>
      </header>

      <p className="about-intro">
        DictaFox is a lightweight dictation utility by Vormela Core for Linux,
        Windows and macOS. Press a shortcut,
        speak, and polished text is inserted into the application you were using.
        Speech recognition runs on your computer with whisper.cpp. There is no
        account, no subscription and no mandatory cloud service.
      </p>

      <div className="about-grid">
        <section className="card">
          <h2>
            <ShieldCheck size={16} aria-hidden="true" /> Privacy by design
          </h2>
          <ul className="check-list">
            <li>Audio is captured only while a dictation session is active.</li>
            <li>
              Recordings and transcripts never leave the device unless you
              explicitly enable cloud text refinement and press record.
            </li>
            <li>No analytics, telemetry, advertising identifiers or accounts.</li>
            <li>
              History can be disabled, audio saving is off by default, and all
              local data can be deleted.
            </li>
            <li>
              Optional provider API keys stay in the local database with file
              permissions restricted to your user.{" "}
              <code>DICTAFOX_OPENAI_API_KEY</code> is supported if you prefer not
              to store a key.
            </li>
          </ul>
        </section>

        <section className="card">
          <h2>Built with</h2>
          <dl className="about-facts">
            <div>
              <dt>Desktop shell</dt>
              <dd>Tauri 2 · Rust</dd>
            </div>
            <div>
              <dt>Interface</dt>
              <dd>React · TypeScript · Vite</dd>
            </div>
            <div>
              <dt>Speech engine</dt>
              <dd>whisper.cpp, compiled into the app</dd>
            </div>
            <div>
              <dt>Audio</dt>
              <dd>
                {platform.isLinux
                  ? "PipeWire / PulseAudio"
                  : platform.isMac
                    ? "CoreAudio"
                    : "WASAPI"}{" "}
                capture at 16 kHz mono
              </dd>
            </div>
            <div>
              <dt>Storage</dt>
              <dd>SQLite in {capabilities?.dataDir ?? "the user data directory"}</dd>
            </div>
            <div>
              <dt>Platform</dt>
              <dd>
                {capabilities
                  ? platform.isLinux
                    ? `${capabilities.desktop} · ${capabilities.displayServer}`
                    : capabilities.osLabel
                  : platform.label}
              </dd>
            </div>
          </dl>
        </section>

        <section className="card">
          <h2>Using DictaFox</h2>
          <ul className="check-list">
            <li>
              <kbd>{settings?.shortcut ?? platform.defaultShortcut}</kbd>{" "}
              {settings?.shortcutMode === "toggle"
                ? "starts and stops dictation."
                : `holds to record; release to transcribe${platform.isLinux ? " (X11)" : ""}.`}
            </li>
            <li>
              The transcript is copied first, then pasted into the previously
              focused window when your desktop allows it.
            </li>
            <li>
              Closing this window hides DictaFox; quit from the tray icon or the
              button below.
            </li>
            {platform.isLinux ? (
              <li>
                Wayland compositors often block global key grabs and simulated
                paste. Bind <code>dictafox --toggle</code> to a desktop shortcut as
                a reliable fallback.
              </li>
            ) : null}
            {platform.isMac ? (
              <li>
                Automatic paste needs Accessibility access: System Settings →
                Privacy &amp; Security → Accessibility.
              </li>
            ) : null}
          </ul>
        </section>

        <section className="card">
          <h2>CLI triggers</h2>
          <dl className="about-facts mono-facts">
            <div>
              <dt>dictafox --toggle</dt>
              <dd>Start dictation, or stop and transcribe.</dd>
            </div>
            <div>
              <dt>dictafox --start</dt>
              <dd>Begin recording.</dd>
            </div>
            <div>
              <dt>dictafox --stop</dt>
              <dd>Stop and transcribe.</dd>
            </div>
            <div>
              <dt>dictafox --cancel</dt>
              <dd>Discard the current recording.</dd>
            </div>
            <div>
              <dt>dictafox --show</dt>
              <dd>Open this dashboard.</dd>
            </div>
          </dl>
        </section>
      </div>

      {capabilities?.limitations.length ? (
        <section className="card">
          <h2>Detected limitations in this session</h2>
          <LimitationsList limitations={capabilities.limitations} />
        </section>
      ) : null}

      <section className="card about-footer-card">
        <div>
          <h2>License and attribution</h2>
          <p className="muted">
            © 2026 Vormela Core. DictaFox is a product of{" "}
            <VormelaLink link="website">Vormela Core</VormelaLink>, released under the
            MIT license. Speech recognition is powered by
            whisper.cpp (MIT). The interface uses React, Tauri and Lucide icons,
            each under its own open-source license. No performance figures are
            claimed without measurement. Source code is on{" "}
            <VormelaLink link="repository">GitHub</VormelaLink>.
          </p>
        </div>
        <div className="about-actions">
          <Button
            variant="ghost"
            icon={<ClipboardCopy size={15} />}
            onClick={() =>
              void copyText(
                `DictaFox ${capabilities?.version ?? ""} (${platform.label})\nSession: ${capabilities?.sessionType ?? ""}\nEngine: ${capabilities?.engine ?? ""}`,
              )
            }
          >
            Copy build info
          </Button>
          <Button
            variant="ghost"
            icon={<FolderOpen size={15} />}
            onClick={() => void openDataDir()}
          >
            Data folder
          </Button>
          <Button variant="danger" onClick={() => void quit()}>
            Quit DictaFox
          </Button>
        </div>
      </section>
    </div>
  );
}
