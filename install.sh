#!/usr/bin/env bash
# ==============================================================================
# K0maru-Agent-Memory: Standalone Installer Script
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/K0maru/k0maru-agent-memory/main/install.sh | bash
#
# Environment Overrides:
#   K0MARU_VERSION    Specify version tag (e.g. v0.6.0). Default: latest.
#   K0MARU_INSTALL_DIR Specify installation directory. Default: /usr/local/bin or ~/.local/bin.
# ==============================================================================

set -euo pipefail

REPO="K0maru/k0maru-agent-memory"
GITHUB_URL="https://github.com/${REPO}"
GITHUB_API="https://api.github.com/repos/${REPO}"

# ANSI Colors
if [ -t 1 ]; then
    BOLD="$(printf '\033[1m')"
    GREEN="$(printf '\033[32m')"
    BLUE="$(printf '\033[34m')"
    YELLOW="$(printf '\033[33m')"
    RED="$(printf '\033[31m')"
    RESET="$(printf '\033[0m')"
else
    BOLD=""
    GREEN=""
    BLUE=""
    YELLOW=""
    RED=""
    RESET=""
fi

log_info() {
    printf "%b[info]%b %b\n" "${BLUE}" "${RESET}" "$1"
}

log_success() {
    printf "%b[ok]%b %b%b%b\n" "${GREEN}" "${RESET}" "${BOLD}" "$1" "${RESET}"
}

log_warn() {
    printf "%b[warn]%b %b\n" "${YELLOW}" "${RESET}" "$1"
}

log_error() {
    printf "%b[error]%b %b\n" "${RED}" "${RESET}" "$1" >&2
}

# 1. Detect Operating System and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        OS_TYPE="apple-darwin"
        ;;
    Linux)
        OS_TYPE="unknown-linux-musl"
        ;;
    *)
        log_error "Unsupported operating system: $OS. K0maru supports macOS and Linux via this installer."
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH_TYPE="x86_64"
        ;;
    arm64|aarch64)
        ARCH_TYPE="aarch64"
        ;;
    *)
        log_error "Unsupported machine architecture: $ARCH"
        exit 1
        ;;
esac

TARGET="${ARCH_TYPE}-${OS_TYPE}"
log_info "Detected platform: ${BOLD}${TARGET}${RESET}"

# 2. Resolve Version Tag
if [ -n "${K0MARU_VERSION:-}" ]; then
    VERSION="$K0MARU_VERSION"
    log_info "Using specified version: ${BOLD}${VERSION}${RESET}"
else
    log_info "Resolving latest release tag from GitHub..."
    LATEST_JSON=$(curl -sSL -H "Accept: application/vnd.github+json" "${GITHUB_API}/releases/latest" || true)
    VERSION=$(printf "%s" "$LATEST_JSON" | grep -o '"tag_name": *"[^"]*"' | head -n 1 | cut -d'"' -f4 || true)
    if [ -z "$VERSION" ]; then
        log_warn "Could not fetch latest release from GitHub API (rate-limit or network). Falling back to v0.6.0."
        VERSION="v0.6.0"
    fi
    log_info "Latest release version: ${BOLD}${VERSION}${RESET}"
fi

# 3. Resolve Download URL
ASSET_NAME="k0maru-${TARGET}.tar.gz"
DOWNLOAD_URL="${GITHUB_URL}/releases/download/${VERSION}/${ASSET_NAME}"
CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

# 4. Download and Extract to Temporary Directory
TMP_DIR="$(mktemp -d -t k0maru-install-XXXXXX)"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

log_info "Downloading ${ASSET_NAME} from ${DOWNLOAD_URL}..."
if ! curl -fL --progress-bar -o "${TMP_DIR}/${ASSET_NAME}" "$DOWNLOAD_URL"; then
    log_error "Failed to download release binary for ${TARGET} (${VERSION})."
    log_error "Please check available assets at: ${GITHUB_URL}/releases/tag/${VERSION}"
    exit 1
fi

# Attempt checksum verification if sha256 file is present
if curl -sLf -o "${TMP_DIR}/${ASSET_NAME}.sha256" "$CHECKSUM_URL" 2>/dev/null; then
    log_info "Verifying SHA-256 checksum..."
    EXPECTED_SHA=$(cut -d' ' -f1 "${TMP_DIR}/${ASSET_NAME}.sha256" | tr -d '\r\n')
    if command -v shasum >/dev/null 2>&1; then
        ACTUAL_SHA=$(shasum -a 256 "${TMP_DIR}/${ASSET_NAME}" | cut -d' ' -f1)
    elif command -v sha256sum >/dev/null 2>&1; then
        ACTUAL_SHA=$(sha256sum "${TMP_DIR}/${ASSET_NAME}" | cut -d' ' -f1)
    else
        ACTUAL_SHA=""
    fi

    if [ -n "$ACTUAL_SHA" ]; then
        if [ "$EXPECTED_SHA" != "$ACTUAL_SHA" ]; then
            log_error "Checksum verification failed! Expected: $EXPECTED_SHA, Got: $ACTUAL_SHA"
            exit 1
        fi
        log_success "Checksum verified: ${ACTUAL_SHA:0:16}..."
    fi
fi

log_info "Extracting archive..."
tar -xzf "${TMP_DIR}/${ASSET_NAME}" -C "$TMP_DIR"

if [ ! -f "${TMP_DIR}/k0maru" ]; then
    log_error "Binary 'k0maru' not found in downloaded archive."
    exit 1
fi

chmod +x "${TMP_DIR}/k0maru"

# 5. Determine Destination Directory
if [ -n "${K0MARU_INSTALL_DIR:-}" ]; then
    DEST_DIR="$K0MARU_INSTALL_DIR"
elif [ -w "/usr/local/bin" ]; then
    DEST_DIR="/usr/local/bin"
elif command -v sudo >/dev/null 2>&1 && [ -t 0 ]; then
    DEST_DIR="/usr/local/bin"
    USE_SUDO="true"
else
    DEST_DIR="${HOME}/.local/bin"
fi

mkdir -p "$DEST_DIR" 2>/dev/null || true

log_info "Installing k0maru to ${DEST_DIR}..."
if [ "${USE_SUDO:-false}" = "true" ] && [ ! -w "$DEST_DIR" ]; then
    sudo cp "${TMP_DIR}/k0maru" "${DEST_DIR}/k0maru"
    sudo chmod 755 "${DEST_DIR}/k0maru"
else
    cp "${TMP_DIR}/k0maru" "${DEST_DIR}/k0maru"
    chmod 755 "${DEST_DIR}/k0maru"
fi

# 6. Verify Installation
INSTALLED_BIN="${DEST_DIR}/k0maru"
if [ -x "$INSTALLED_BIN" ]; then
    VERSION_OUTPUT="$("$INSTALLED_BIN" --version 2>&1 || true)"
    log_success "Successfully installed: ${VERSION_OUTPUT}"
else
    log_error "Installation failed: ${INSTALLED_BIN} is not executable."
    exit 1
fi

# Check if destination directory is in PATH
case ":${PATH}:" in
    *":${DEST_DIR}:"*)
        ;;
    *)
        log_warn "${DEST_DIR} is not currently in your \$PATH."
        log_warn "Add the following to your ~/.zshrc or ~/.bashrc:"
        printf "    export PATH=\"%s:\$PATH\"\n\n" "$DEST_DIR"
        ;;
esac

printf "\n%b\n" "${BOLD}🎉 K0maru-Agent-Memory is ready!${RESET}"
printf "To get started:\n"
printf "  ${GREEN}k0maru doctor${RESET}             # Run health diagnostics\n"
printf "  ${GREEN}k0maru install --target all${RESET} # One-click configure Claude / Cursor / Gemini\n"
printf "  ${GREEN}k0maru --help${RESET}               # View all CLI commands\n\n"
