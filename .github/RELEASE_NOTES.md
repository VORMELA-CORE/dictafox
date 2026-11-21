DictaFox by **Vormela Core** — Speak Naturally. Type Everywhere.

Local, private voice dictation: press a shortcut, speak, and the text is typed into the app you were using. Speech recognition runs on your computer with whisper.cpp.

## Install

| Platform | Easiest | File |
| --- | --- | --- |
| **Linux** (Ubuntu, Debian, Mint, Pop!_OS) | `curl -fsSL https://raw.githubusercontent.com/VORMELA-CORE/dictafox/main/scripts/install.sh \| bash` | `DictaFox_*_amd64.deb` |
| **Linux** (Fedora, openSUSE) | same command | `DictaFox-*.x86_64.rpm` |
| **Linux** (any distro, portable) | `install.sh --appimage` | `DictaFox_*_amd64.AppImage` |
| **Windows 10/11** | `irm https://raw.githubusercontent.com/VORMELA-CORE/dictafox/main/scripts/install.ps1 \| iex` | `DictaFox_*_x64-setup.exe` (or `.msi`) |
| **macOS** Apple Silicon | `curl -fsSL https://raw.githubusercontent.com/VORMELA-CORE/dictafox/main/scripts/install.sh \| bash` | `DictaFox_*_aarch64.dmg` |
| **macOS** Intel | same command | `DictaFox_*_x64.dmg` |

`SHA256SUMS.txt` lists checksums for every file.

**macOS:** the app is not notarized yet. The install script handles this. If you open the `.dmg` by hand, right-click DictaFox → **Open** the first time. Allow **Microphone** access, and **Accessibility** access for automatic paste.

**Windows:** SmartScreen may warn about an unrecognised publisher. Choose **More info → Run anyway**.

On first launch, DictaFox walks you through choosing a microphone, downloading a speech model (Tiny is 78 MB), and setting a shortcut.
