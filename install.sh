#!/bin/sh
# ---------------------------------------------------------------------------
# Relay — POSIX shell installer
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/ffgenius/relay/master/install.sh | sh
#   wget -qO- https://raw.githubusercontent.com/ffgenius/relay/master/install.sh | sh
#
#   # Install a specific version:
#   curl -fsSL ... | sh -s -- --version 0.0.6
#
# What it does:
#   1. Detects your OS (Linux / macOS) and CPU architecture.
#   2. Downloads the matching prebuilt binary from GitHub Releases.
#   3. Installs it to ~/.relay/bin/relay  (no sudo required).
#   4. Runs `relay init` so your shell (bash / zsh / fish) picks up
#      ~/.relay/bin on PATH automatically.
#
# No root. No package manager. No surprises.
# ---------------------------------------------------------------------------
set -u

# ── Configuration ──────────────────────────────────────────────────────────

REPO="ffgenius/relay"
INSTALL_DIR="${HOME}/.relay/bin"
BIN_NAME="relay"

# ---- pretty-print helpers ------------------------------------------------
BOLD=""; DIM=""; GREEN=""; CYAN=""; RESET=""
if [ -t 2 ] && [ -z "${NO_COLOR:-}" ]; then
  BOLD="$(printf '\033[1m')"
  DIM="$(printf '\033[2m')"
  GREEN="$(printf '\033[32m')"
  CYAN="$(printf '\033[36m')"
  RESET="$(printf '\033[0m')"
fi

info()  { printf '%s\n' "${CYAN}>${RESET} $*" >&2; }
ok()    { printf '%s\n' "${GREEN}✓${RESET} $*" >&2; }
err()   { printf '%s\n' "${BOLD}error:${RESET} $*" >&2; exit 1; }

# ---- helpers -------------------------------------------------------------

detect_platform() {
  case "$(uname -s)" in
    Linux)  echo "linux" ;;
    Darwin) echo "darwin" ;;
    *)      err "unsupported OS: $(uname -s). Relay currently supports Linux and macOS." ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)  echo "x64" ;;
    aarch64|arm64) echo "arm64" ;;
    *)             err "unsupported architecture: $(uname -m). Relay currently supports x86_64 and arm64." ;;
  esac
}

# Download a URL to stdout. Prefer curl, fall back to wget.
download() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" || err "download failed (curl): $1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1" || err "download failed (wget): $1"
  else
    err "neither curl nor wget is available — please install one of them and retry."
  fi
}

# ---- main ----------------------------------------------------------------

main() {
  # Parse --version
  REQUESTED_VERSION=""
  NO_INIT=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --version) REQUESTED_VERSION="$2"; shift 2 ;;
      --no-init) NO_INIT=1; shift ;;
      --help|-h)
        printf '%s\n' "Usage: $0 [--version <ver>] [--no-init]"
        printf '%s\n' ""
        printf '%s\n' "Options:"
        printf '%s\n' "  --version <ver>  Install a specific version (e.g. 0.0.6)"
        printf '%s\n' "  --no-init        Skip 'relay init' (don't touch shell profiles)"
        exit 0
        ;;
      *) err "unknown option: $1 (use --help for usage)" ;;
    esac
  done

  PLATFORM="$(detect_platform)"
  ARCH="$(detect_arch)"

  info "detected: ${PLATFORM}-${ARCH}"

  # Resolve the version tag.
  if [ -n "${REQUESTED_VERSION}" ]; then
    TAG="v${REQUESTED_VERSION#v}"
    VERSION="${REQUESTED_VERSION#v}"
    info "installing requested version ${VERSION}"
  else
    info "fetching latest release from GitHub…"
    TAG="$(download "https://api.github.com/repos/${REPO}/releases/latest" \
      | grep '"tag_name":' \
      | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
    if [ -z "${TAG}" ]; then
      err "could not determine latest release tag. Try --version <ver> to pin a specific version."
    fi
    VERSION="${TAG#v}"
    info "latest release: ${TAG} (version ${VERSION})"
  fi

  # Build the download URL.
  ARCHIVE="relay-${VERSION}-${PLATFORM}-${ARCH}.tar.gz"
  URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"

  # Create install directory.
  mkdir -p "${INSTALL_DIR}"

  # Download and extract.
  info "downloading ${ARCHIVE}…"
  TMPDIR="$(mktemp -d)"
  trap 'rm -rf "${TMPDIR}"' EXIT

  download "${URL}" > "${TMPDIR}/${ARCHIVE}"

  info "extracting…"
  tar -xzf "${TMPDIR}/${ARCHIVE}" -C "${TMPDIR}"
  # The archive wraps the binary in a directory, e.g. relay-0.0.6-linux-x64/relay
  cp "${TMPDIR}/relay-${VERSION}-${PLATFORM}-${ARCH}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
  chmod +x "${INSTALL_DIR}/${BIN_NAME}"

  ok "installed relay ${VERSION} → ${INSTALL_DIR}/${BIN_NAME}"

  # Run relay init for shell integration.
  if [ -z "${NO_INIT}" ]; then
    info "running 'relay init' to configure shell integration…"
    "${INSTALL_DIR}/${BIN_NAME}" init
  else
    info "skipped 'relay init' (--no-init). Run '${INSTALL_DIR}/${BIN_NAME} init' manually."
  fi

  # Print next steps.
  echo ""
  printf '%s\n' "${BOLD}Next steps:${RESET}"
  printf '%s\n' "  1. Open a ${BOLD}new terminal${RESET} (or run 'exec \$SHELL -l') for PATH changes to take effect."
  printf '%s\n' "  2. Try it: ${CYAN}relay add g git${RESET}"
  printf '%s\n' "  3. Then:  ${CYAN}g status${RESET}"
  echo ""
}

main "$@"
