# ---------------------------------------------------------------------------
# Relay — PowerShell installer (Windows)
#
# Usage:
#   irm https://raw.githubusercontent.com/ffgenius/relay/master/install.ps1 | iex
#
#   # Install a specific version:
#   irm ... | iex -args "-Version 0.0.6"
#
# What it does:
#   1. Detects your CPU architecture.
#   2. Downloads the matching prebuilt binary from GitHub Releases.
#   3. Installs it to ~/.relay/bin/relay.exe
#   4. Runs `relay init` to add ~/.relay/bin to your user PATH (via the
#      registry — no admin required).
# ---------------------------------------------------------------------------
param(
    [string]$Version = "",
    [switch]$NoInit = $false
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"  # speeds up Invoke-WebRequest

# ── Configuration ──────────────────────────────────────────────────────────

$Repo    = "ffgenius/relay"
$BinDir  = "$env:USERPROFILE\.relay\bin"
$BinName = "relay.exe"

# ---- helpers -------------------------------------------------------------

function info { Write-Host "> $args" -ForegroundColor Cyan }
function ok   { Write-Host "✓ $args" -ForegroundColor Green }
function err  { Write-Host "error: $args" -ForegroundColor Red; exit 1 }

function Detect-Arch {
    switch -Wildcard ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64" { return "x64" }
        "ARM64" { return "arm64" }
        default { err "unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
    }
}

# ---- main ----------------------------------------------------------------

function Main {
    $Arch = Detect-Arch
    info "detected: win32-$Arch"

    # Resolve version.
    if ($Version) {
        $ver = $Version -replace '^v', ''
        $Tag = "v$ver"
        info "installing requested version $ver"
    } else {
        info "fetching latest release from GitHub…"
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        $Tag    = $release.tag_name
        $ver    = $Tag -replace '^v', ''
        if (-not $Tag) {
            err "could not determine latest release tag. Try -Version 0.0.6 to pin a specific version."
        }
        info "latest release: $Tag (version $ver)"
    }

    # Download URL.
    $Archive = "relay-$ver-win32-$Arch.zip"
    $Url     = "https://github.com/$Repo/releases/download/$Tag/$Archive"

    # Create install directory.
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

    # Download and extract.
    info "downloading $Archive…"
    $TmpDir = Join-Path $env:TEMP "relay-install-$(Get-Random)"
    New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null
    try {
        $ZipPath = Join-Path $TmpDir $Archive
        Invoke-WebRequest -Uri $Url -OutFile $ZipPath

        info "extracting…"
        Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

        # The archive wraps the binary in relay-<ver>-win32-<arch>/relay.exe
        $Extracted = Join-Path $TmpDir "relay-$ver-win32-$Arch" $BinName
        Copy-Item -Path $Extracted -Destination (Join-Path $BinDir $BinName) -Force

        ok "installed relay $ver → $BinDir\$BinName"
    } finally {
        Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
    }

    # Run relay init for PATH setup.
    if (-not $NoInit) {
        info "running 'relay init' to configure shell integration…"
        & (Join-Path $BinDir $BinName) init
    } else {
        info "skipped 'relay init' (-NoInit). Run '$(Join-Path $BinDir $BinName) init' manually."
    }

    # Print next steps.
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor White
    Write-Host "  1. Open a new terminal for PATH changes to take effect."
    Write-Host "  2. Try it: relay add g git" -ForegroundColor Cyan
    Write-Host "  3. Then:  g status" -ForegroundColor Cyan
    Write-Host ""
}

Main
