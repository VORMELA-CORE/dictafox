# DictaFox installer for Windows — by Vormela Core.
#
#   irm https://raw.githubusercontent.com/VORMELA-CORE/dictafox/main/scripts/install.ps1 | iex
#
# Installs the latest release for the current user (no administrator rights
# needed) and starts DictaFox. Set $env:DICTAFOX_VERSION = "v0.1.0" to pin a
# release, or $env:GITHUB_TOKEN if the repository is private.

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$repo = if ($env:DICTAFOX_REPO) { $env:DICTAFOX_REPO } else { "VORMELA-CORE/dictafox" }
$version = if ($env:DICTAFOX_VERSION) { $env:DICTAFOX_VERSION } else { "latest" }
$api = if ($version -eq "latest") {
    "https://api.github.com/repos/$repo/releases/latest"
} else {
    "https://api.github.com/repos/$repo/releases/tags/$version"
}

$headers = @{ "User-Agent" = "dictafox-installer"; "Accept" = "application/vnd.github+json" }
if ($env:GITHUB_TOKEN) { $headers["Authorization"] = "Bearer $($env:GITHUB_TOKEN)" }

if (-not [Environment]::Is64BitOperatingSystem) {
    throw "DictaFox requires 64-bit Windows 10 or 11."
}

Write-Host "==> Looking up DictaFox $version" -ForegroundColor Green
$release = Invoke-RestMethod -Uri $api -Headers $headers
$asset = $release.assets | Where-Object { $_.name -like "*x64-setup.exe" } | Select-Object -First 1
if (-not $asset) { throw "No Windows installer was found in release $($release.tag_name)." }

$installer = Join-Path $env:TEMP $asset.name
Write-Host "==> Downloading $($asset.name)" -ForegroundColor Green
$downloadHeaders = $headers.Clone()
$downloadHeaders["Accept"] = "application/octet-stream"
Invoke-WebRequest -Uri $asset.url -Headers $downloadHeaders -OutFile $installer

Write-Host "==> Installing for $env:USERNAME" -ForegroundColor Green
$process = Start-Process -FilePath $installer -ArgumentList "/S" -Wait -PassThru
Remove-Item $installer -ErrorAction SilentlyContinue
if ($process.ExitCode -ne 0) { throw "The installer exited with code $($process.ExitCode)." }

$exe = Join-Path $env:LOCALAPPDATA "DictaFox\dictafox.exe"
if (Test-Path $exe) {
    Write-Host "==> DictaFox $($release.tag_name) is installed. Starting it now." -ForegroundColor Green
    Start-Process $exe
} else {
    Write-Host "==> DictaFox is installed. Open it from the Start menu." -ForegroundColor Green
}
