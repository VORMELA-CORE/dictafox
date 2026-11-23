# Changelog

All notable changes to DictaFox are listed here. DictaFox is a product of [Vormela Core](https://vormelacore.com).

## Unreleased

### Changed
- New app icon: the Vormela Core mark on night, as set out in the brand guide. Every platform icon is regenerated from `assets/brand/dictafox-app-icon-1024.png`.
- Visual identity follows the Vormela Core brand: the Malachite colour palette in both themes, and the Manrope typeface, bundled with the app (no network font download).
- "by Vormela Core" appears in the window title, the tray tooltip, the sidebar, the start-up screen, the setup screen, the About page and the README.
- The About page links to vormelacore.com and to the source repository.

### Fixed
- `install.sh --uninstall` now removes the `.deb` and `.rpm` packages. It looked for the wrong package name, so the package was never removed.

## 0.1.0 (2026-11-21)

First public release for Linux (`.deb`, `.rpm`, AppImage), Windows (`.msi`, setup `.exe`) and macOS (Apple Silicon and Intel `.dmg`).

- Hold or toggle a shortcut, speak, and the text is typed into the app you were using.
- Local speech recognition with whisper.cpp. No account, subscription or cloud service.
- Local history and a personal dictionary. Optional text refinement, off by default, can send transcript text (never audio) to a provider you configure.
- Command-line control: `dictafox --toggle`, `--start`, `--stop`, `--cancel` and `--show`.
