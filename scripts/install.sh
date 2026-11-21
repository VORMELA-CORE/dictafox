#!/usr/bin/env bash
# DictaFox installer for Linux and macOS — by Vormela Core.
#
#   curl -fsSL https://raw.githubusercontent.com/VORMELA-CORE/dictafox/main/scripts/install.sh | bash
#
# Linux:  Debian/Ubuntu/Mint/Pop!_OS   -> .deb via apt
#         Fedora/RHEL/openSUSE          -> .rpm via dnf/zypper
#         anything else (or --appimage) -> AppImage in ~/.local with a menu entry
# macOS:  .dmg for Apple Silicon or Intel -> /Applications (or ~/Applications)
#
# Options:
#   --appimage        Linux: install the portable AppImage even if a package fits
#   --version vX.Y.Z  Install a specific release instead of the latest
#   --uninstall       Remove DictaFox (your data and models are kept)
#   -h, --help        Show this help
#
# Environment: DICTAFOX_REPO (default VORMELA-CORE/dictafox), GITHUB_TOKEN
# (only needed if the repository is private and the GitHub CLI is not logged in).

set -euo pipefail

REPO="${DICTAFOX_REPO:-VORMELA-CORE/dictafox}"
VERSION="latest"
FORCE_APPIMAGE=0
UNINSTALL=0
PKG_NAME="dicta-fox"

APPIMAGE_DIR="$HOME/.local/share/dictafox-app"
APPIMAGE_PATH="$APPIMAGE_DIR/DictaFox.AppImage"
BIN_LINK="$HOME/.local/bin/dictafox"
DESKTOP_FILE="$HOME/.local/share/applications/dictafox.desktop"
ICON_FILE="$HOME/.local/share/icons/hicolor/256x256/apps/dictafox.png"

say() { printf '\033[1;32m==>\033[0m %s\n' "$*" >&2; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
has() { command -v "$1" >/dev/null 2>&1; }

usage() {
  cat <<'EOF'
DictaFox installer (Linux and macOS)

Usage: install.sh [--appimage] [--version vX.Y.Z] [--uninstall]

  --appimage        Linux: install the portable AppImage even if a package fits
  --version vX.Y.Z  Install a specific release instead of the latest
  --uninstall       Remove DictaFox (settings, history and models are kept)
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --appimage) FORCE_APPIMAGE=1 ;;
    --uninstall) UNINSTALL=1 ;;
    --version) shift; VERSION="${1:-}"; [ -n "$VERSION" ] || die "--version needs a value such as v0.1.0" ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
  shift
done

SUDO=""
if [ "$(id -u)" -ne 0 ] && has sudo; then SUDO="sudo"; fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Download the release asset whose name matches a shell glob into $TMP and
# print its path. Uses the GitHub CLI when it is logged in (works for private
# repositories), otherwise the GitHub REST API.
fetch_asset() {
  local pattern="$1"
  if has gh && gh auth status >/dev/null 2>&1; then
    local tag_args=()
    if [ "$VERSION" != "latest" ]; then tag_args=("$VERSION"); fi
    if gh release download ${tag_args[@]+"${tag_args[@]}"} -R "$REPO" -p "$pattern" -D "$TMP" --clobber >/dev/null 2>&1; then
      find "$TMP" -maxdepth 1 -type f -name "$pattern" | head -n1
      return 0
    fi
  fi
  has curl || die "curl is required"
  local api="https://api.github.com/repos/$REPO/releases/latest"
  if [ "$VERSION" != "latest" ]; then api="https://api.github.com/repos/$REPO/releases/tags/$VERSION"; fi
  local auth=()
  if [ -n "${GITHUB_TOKEN:-}" ]; then auth=(-H "Authorization: Bearer $GITHUB_TOKEN"); fi
  local json
  json="$(curl -fsSL ${auth[@]+"${auth[@]}"} -H "Accept: application/vnd.github+json" "$api")" \
    || die "could not read the $VERSION release of $REPO. If the repository is private, run 'gh auth login' first."
  local line name url
  while IFS= read -r line; do
    url="${line%% *}"; name="${line#* }"
    # shellcheck disable=SC2053
    if [[ "$name" == $pattern ]]; then
      say "Downloading $name"
      curl -fL --progress-bar ${auth[@]+"${auth[@]}"} -H "Accept: application/octet-stream" -o "$TMP/$name" "$url" \
        || die "download of $name failed"
      echo "$TMP/$name"
      return 0
    fi
  done < <(printf '%s' "$json" | tr ',' '\n' | awk -F'"' '
      /"url": *"https:\/\/api\.github\.com\/repos\/.*\/releases\/assets\// { url=$4 }
      /"name":/ { if (url != "") { print url " " $4; url="" } }')
  die "no release file matching $pattern was found in $REPO ($VERSION)"
}

linux_arch_ok() {
  case "$(uname -m)" in
    x86_64|amd64) ;;
    *) die "DictaFox currently ships Linux builds for x86_64 only (this machine is $(uname -m))." ;;
  esac
}

install_appimage() {
  say "Installing the DictaFox AppImage for $(id -un)"
  local file
  file="$(fetch_asset '*.AppImage')"
  mkdir -p "$APPIMAGE_DIR" "$(dirname "$BIN_LINK")" "$(dirname "$DESKTOP_FILE")" "$(dirname "$ICON_FILE")"
  install -m 755 "$file" "$APPIMAGE_PATH"

  # AppImages need FUSE 2; newer distros ship only FUSE 3. Extract-and-run
  # works everywhere, so use it when libfuse.so.2 is missing.
  local runner="exec \"$APPIMAGE_PATH\" \"\$@\""
  if ! (ldconfig -p 2>/dev/null | grep -q 'libfuse.so.2'); then
    runner="APPIMAGE_EXTRACT_AND_RUN=1 exec \"$APPIMAGE_PATH\" \"\$@\""
    warn "libfuse2 not found; DictaFox will run in extract-and-run mode (slightly slower start)."
  fi
  printf '#!/bin/sh\n%s\n' "$runner" > "$BIN_LINK"
  chmod 755 "$BIN_LINK"

  (cd "$TMP" && APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGE_PATH" --appimage-extract '*.png' >/dev/null 2>&1 || true)
  local icon
  icon="$(find "$TMP/squashfs-root" -name '*.png' -path '*256x256*' 2>/dev/null | head -n1 || true)"
  if [ -z "$icon" ]; then icon="$(find "$TMP/squashfs-root" -name '*.png' 2>/dev/null | head -n1 || true)"; fi
  if [ -n "$icon" ]; then cp "$icon" "$ICON_FILE"; fi

  cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=DictaFox
GenericName=Voice Dictation
Comment=Speak Naturally. Type Everywhere.
Exec=$BIN_LINK
Icon=dictafox
Terminal=false
Categories=Utility;Accessibility;
Keywords=dictation;speech;voice;transcription;whisper;
StartupWMClass=dictafox
EOF
  if has update-desktop-database; then update-desktop-database "$(dirname "$DESKTOP_FILE")" >/dev/null 2>&1 || true; fi
  case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) warn "$HOME/.local/bin is not on your PATH; add it to use the 'dictafox' command." ;;
  esac
}

install_linux() {
  linux_arch_ok
  local file
  if [ "$FORCE_APPIMAGE" -eq 0 ] && has apt-get && has dpkg; then
    say "Installing the DictaFox .deb package (apt)"
    file="$(fetch_asset '*_amd64.deb')"
    $SUDO apt-get install -y "$file"
  elif [ "$FORCE_APPIMAGE" -eq 0 ] && has dnf; then
    say "Installing the DictaFox .rpm package (dnf)"
    file="$(fetch_asset '*.x86_64.rpm')"
    $SUDO dnf install -y "$file"
  elif [ "$FORCE_APPIMAGE" -eq 0 ] && has zypper; then
    say "Installing the DictaFox .rpm package (zypper)"
    file="$(fetch_asset '*.x86_64.rpm')"
    $SUDO zypper --non-interactive install --allow-unsigned-rpm "$file"
  else
    install_appimage
  fi

  # Audio capture and clipboard helpers: packages pull these in as
  # recommendations; the AppImage relies on the desktop already having them.
  local missing=()
  has pactl || missing+=("pulseaudio-utils (microphone capture)")
  if [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
    has wl-copy || missing+=("wl-clipboard (clipboard on Wayland)")
  else
    has xclip || missing+=("xclip (clipboard on X11)")
  fi
  if [ "${#missing[@]}" -gt 0 ]; then
    warn "DictaFox also needs: ${missing[*]}. Install them with your package manager."
  fi
}

install_macos() {
  local pattern
  case "$(uname -m)" in
    arm64) pattern='*_aarch64.dmg' ;;
    x86_64) pattern='*_x64.dmg' ;;
    *) die "unsupported Mac architecture $(uname -m)" ;;
  esac
  say "Downloading DictaFox for macOS ($(uname -m))"
  local dmg mount dest
  dmg="$(fetch_asset "$pattern")"
  mount="$TMP/mount"
  mkdir -p "$mount"
  hdiutil attach -nobrowse -quiet -mountpoint "$mount" "$dmg"
  dest="/Applications"
  [ -w "$dest" ] || dest="$HOME/Applications"
  mkdir -p "$dest"
  rm -rf "$dest/DictaFox.app"
  cp -R "$mount/DictaFox.app" "$dest/"
  hdiutil detach -quiet "$mount" || true
  # The app is not notarized by Apple yet; clear the download quarantine flag
  # so Gatekeeper opens it without the "unidentified developer" block.
  xattr -dr com.apple.quarantine "$dest/DictaFox.app" 2>/dev/null || true
  say "Installed to $dest/DictaFox.app"
  echo "    On first use, allow Microphone access, and Accessibility access for automatic" >&2
  echo "    paste (System Settings → Privacy & Security)." >&2
  open "$dest/DictaFox.app" || true
}

uninstall() {
  case "$(uname -s)" in
    Darwin)
      rm -rf "/Applications/DictaFox.app" "$HOME/Applications/DictaFox.app"
      ;;
    Linux)
      if has dpkg && dpkg -s "$PKG_NAME" >/dev/null 2>&1; then
        $SUDO apt-get remove -y "$PKG_NAME"
      elif has rpm && rpm -q "$PKG_NAME" >/dev/null 2>&1; then
        if has dnf; then $SUDO dnf remove -y "$PKG_NAME"; else $SUDO zypper --non-interactive remove "$PKG_NAME"; fi
      fi
      rm -f "$BIN_LINK" "$DESKTOP_FILE" "$ICON_FILE"
      rm -rf "$APPIMAGE_DIR"
      ;;
  esac
  say "DictaFox was removed. Settings, history and models remain in your user data folder."
}

main() {
  if [ "$UNINSTALL" -eq 1 ]; then uninstall; exit 0; fi
  case "$(uname -s)" in
    Linux) install_linux ;;
    Darwin) install_macos ;;
    *) die "this installer supports Linux and macOS. On Windows run in PowerShell: irm https://raw.githubusercontent.com/$REPO/main/scripts/install.ps1 | iex" ;;
  esac
  say "DictaFox is installed. Open it from your applications menu, or run: dictafox"
}

main
