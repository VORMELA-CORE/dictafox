<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="DictaFox app icon">
</p>

<h1 align="center">DictaFox</h1>
<p align="center"><strong>Speak Naturally. Type Everywhere.</strong></p>

<p align="center">
  <a href="https://vormelacore.com">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="src/assets/brand/vormela-core-lockup-dark.svg">
      <img src="src/assets/brand/vormela-core-lockup-light.svg" height="34" alt="A product of Vormela Core">
    </picture>
  </a>
</p>

DictaFox is a private voice-dictation app for **Linux**, **Windows** and **macOS**. Press a shortcut and speak. DictaFox types the text into the app you were using. Speech recognition runs on your own computer with [whisper.cpp](https://github.com/ggerganov/whisper.cpp). You do not need an account, a subscription or a cloud service.

## Install

### Linux (first-class platform)

```bash
curl -fsSL https://raw.githubusercontent.com/VORMELA-CORE/dictafox/main/scripts/install.sh | bash
```

The script picks the correct package for your system:

| Distribution | Package |
| --- | --- |
| Ubuntu, Debian, Linux Mint, Pop!_OS, elementary | `.deb` (through `apt`, which also installs the dependencies) |
| Fedora, RHEL, openSUSE | `.rpm` (through `dnf` or `zypper`) |
| Arch, NixOS, any other distribution | portable AppImage in `~/.local`, with a menu entry |

Use `--appimage` to get the AppImage on every distribution. Use `--uninstall` to remove DictaFox. You can also download a file from the [Releases](../../releases) page and install it by hand:

```bash
sudo apt install ./DictaFox_*_amd64.deb      # Debian / Ubuntu
sudo dnf install ./DictaFox-*.x86_64.rpm     # Fedora
chmod +x DictaFox_*_amd64.AppImage && ./DictaFox_*_amd64.AppImage
```

DictaFox records the microphone with PipeWire or PulseAudio (`pactl`, `parecord`). It uses `wl-clipboard` (Wayland) or `xclip` (X11) for the clipboard. For automatic paste it uses `wtype` or `ydotool` (Wayland) or XTest (X11). The `.deb` and `.rpm` packages recommend these tools. **Settings → Diagnostics** shows what is missing.

### Windows 10 / 11

In PowerShell:

```powershell
irm https://raw.githubusercontent.com/VORMELA-CORE/dictafox/main/scripts/install.ps1 | iex
```

This installs DictaFox for the current user and does not need administrator rights. You can also run `DictaFox_*_x64-setup.exe` or the `.msi` from the Releases page. If SmartScreen shows a warning, select **More info → Run anyway**.

### macOS 11+ (Apple Silicon and Intel)

```bash
curl -fsSL https://raw.githubusercontent.com/VORMELA-CORE/dictafox/main/scripts/install.sh | bash
```

You can also open the `.dmg` for your Mac and drag DictaFox to Applications. The app is not notarized yet, so right-click it and select **Open** the first time. Then allow **Microphone** access, and **Accessibility** access so that DictaFox can press Cmd+V for you.

## Use

1. On first launch, DictaFox guides you through the setup: microphone, speech model (Tiny is 78 MB, Base is more accurate) and shortcut.
2. Hold the shortcut, speak, and release. The default shortcut is **Ctrl+Space**, or **Option+Space** on macOS. In toggle mode, you press the shortcut to start and press it again to stop.
3. DictaFox copies the text and pastes it into the app that had focus. If the system does not allow automatic paste, the text stays on the clipboard.

When you close the window, DictaFox continues to run in the system tray. To stop it, select **Quit** in the tray menu.

### Command line

```text
dictafox --toggle    start dictation, or stop and transcribe
dictafox --start     start recording
dictafox --stop      stop and transcribe
dictafox --cancel    discard the current recording
dictafox --show      open the dashboard
```

On **Linux Wayland**, applications cannot grab global keys. Bind `dictafox --toggle` to a custom shortcut in your desktop settings.

### Where your data goes

Your audio and transcripts stay on your computer. The only optional network use is **cloud text refinement**, which is off by default. If you turn it on in **Settings → Refinement** and add a key, DictaFox sends the transcript text (never the audio) to the OpenAI-compatible provider you configure. Dictation keeps working without it.

All your data stays on your computer:

| OS | Data folder |
| --- | --- |
| Linux | `~/.local/share/dictafox` |
| Windows | `%APPDATA%\DictaFox` |
| macOS | `~/Library/Application Support/DictaFox` |

## Platform support

| | Linux X11 | Linux Wayland | Windows | macOS |
| --- | --- | --- | --- | --- |
| Global shortcut | ✅ | via `dictafox --toggle` binding | ✅ | ✅ |
| Hold-to-talk release | ✅ | press again to stop | ✅ | ✅ |
| Automatic paste | ✅ XTest | ✅ with `wtype` / `ydotool`, otherwise clipboard | ✅ | ✅ after Accessibility permission |
| Audio capture | PipeWire / PulseAudio | PipeWire / PulseAudio | WASAPI | CoreAudio |

## Build from source

Requirements:

- Rust (stable)
- Node.js 20 or later
- CMake and Clang (to compile whisper.cpp)
- Linux only: the WebKitGTK development packages. On Ubuntu, install them with this command:

  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev cmake clang
  ```

Then run these commands:

```bash
npm ci
npm run tauri:dev      # development build with hot reload
npm run tauri:build    # installers for the current OS, in target/release/bundle/
cargo test --workspace # Rust tests
npm test               # frontend tests
```

## Releasing

Change the version in `package.json`, `src-tauri/tauri.conf.json` and both `Cargo.toml` files, and move the **Unreleased** entries in `CHANGELOG.md` under the new version. Then push a tag:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

The **Release** workflow builds the `.deb`, `.rpm`, AppImage, `.msi`, `-setup.exe` and the Apple Silicon and Intel `.dmg` files. It then publishes them with checksums to a GitHub Release. Release builds compile whisper.cpp for a portable CPU baseline (`scripts/ci/portable-cpu.cmake`), so the app runs on any x86_64 CPU with AVX2 and on any Apple Silicon Mac.

## Project layout

```text
crates/dictafox-core   engine: audio capture, whisper.cpp, refinement, clipboard/paste, storage
src-tauri              desktop shell: windows, tray, global shortcut, CLI control
src                    React + TypeScript interface
assets/brand           source artwork for the app icon
scripts                installers and CI helpers
```

## Brand and icons

DictaFox follows the Vormela Core brand system: the Malachite palette and the Manrope typeface. The app icon is the Vormela Core mark on night. To regenerate every platform icon after changing the artwork:

```bash
npx tauri icon assets/brand/dictafox-app-icon-1024.png -o src-tauri/icons
rm -rf src-tauri/icons/android src-tauri/icons/ios
```

## Changelog

Release history is in [CHANGELOG.md](CHANGELOG.md).

## License

The DictaFox source code is released under the [MIT License](LICENSE), © 2026 Vormela Core.

The Vormela Core and DictaFox logos, the app icon and the brand lockups are not covered by the MIT license. Please use them only to identify the official product or the company, and do not use them in a way that suggests endorsement of a fork.

## About Vormela Core

[Vormela Core](https://vormelacore.com) builds product-led software: intelligent platforms and connected tools. DictaFox is one of them.
