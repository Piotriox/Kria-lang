#!/usr/bin/env bash
# Kria build & install (Linux/macOS)
#
# Run from anywhere:
#   ./release/build.sh              # build only
#   ./release/build.sh install      # build + install to ~/.kria/bin
#   ./release/build.sh package      # build + tar.gz in release/
#   ./release/build.sh clean        # cargo clean + remove release artifacts
#   ./release/build.sh help

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

PROJECT_NAME="kria"
BINARY_NAME="kria"
INSTALL_DIR="${HOME}/.kria/bin"
# Always use the project target dir so install paths are predictable
export CARGO_TARGET_DIR="${PROJECT_ROOT}/target"
BINARY_PATH="${CARGO_TARGET_DIR}/release/${BINARY_NAME}"

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
esac

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"

get_version() {
    grep 'version = "' "${PROJECT_ROOT}/Cargo.toml" | head -1 | sed 's/.*version = "\([^"]*\)".*/\1/'
}

usage() {
    echo "Kria build script (project root: ${PROJECT_ROOT})"
    echo ""
    echo "Usage:"
    echo "  ${SCRIPT_DIR}/build.sh              Build release binary"
    echo "  ${SCRIPT_DIR}/build.sh install      Build and install to ${INSTALL_DIR}"
    echo "  ${SCRIPT_DIR}/build.sh package      Build and create tar.gz in release/"
    echo "  ${SCRIPT_DIR}/build.sh clean        Clean build artifacts"
    echo "  ${SCRIPT_DIR}/build.sh help         Show this help"
}

do_build() {
    if ! command -v cargo >/dev/null 2>&1; then
        echo "[-] Rust/cargo not found. Install from https://rustup.rs/" >&2
        exit 1
    fi

    echo "[*] Building ${PROJECT_NAME} (release)..."
    echo "[*] Target directory: ${CARGO_TARGET_DIR}"
    cargo build --release

    if [ ! -f "${BINARY_PATH}" ]; then
        echo "[-] Binary not found at ${BINARY_PATH}" >&2
        echo "[-] Try: cd ${PROJECT_ROOT} && cargo build --release" >&2
        exit 1
    fi

    echo "[+] Build successful: ${BINARY_PATH}"
}

case "${1:-}" in
    help|--help|-h)
        usage
        exit 0
        ;;
    clean)
        echo "[*] Cleaning..."
        cargo clean
        # Remove packaged artifacts only; keep build.sh and installer scripts
        find "${SCRIPT_DIR}" -mindepth 1 -maxdepth 1 \
            \( -name '*.tar.gz' -o -name '*.zip' \) -delete 2>/dev/null || true
        echo "[+] Clean done."
        exit 0
        ;;
esac

do_build

case "${1:-}" in
    install)
        echo "[*] Installing to ${INSTALL_DIR}..."
        mkdir -p "${INSTALL_DIR}"
        cp "${BINARY_PATH}" "${INSTALL_DIR}/${BINARY_NAME}"
        chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
        echo "[+] Installed: ${INSTALL_DIR}/${BINARY_NAME}"

        case ":${PATH}:" in
            *":${INSTALL_DIR}:"*) ;;
            *)
                echo ""
                echo "[!] ${INSTALL_DIR} is not on your PATH. Add:"
                echo "    export PATH=\"\${PATH}:${INSTALL_DIR}\""
                echo "    # persist (bash): echo 'export PATH=\"\${PATH}:${INSTALL_DIR}\"' >> ~/.bashrc"
                ;;
        esac

        echo ""
        echo "[*] Verifying..."
        if "${INSTALL_DIR}/${BINARY_NAME}" --help >/dev/null 2>&1; then
            echo "[+] \`kria --help\` works."
        else
            echo "[-] Installed binary failed --help check." >&2
            exit 1
        fi
        echo ""
        echo "[+] Run: kria          (REPL)"
        echo "[+]      kria file.krx"
        ;;

    package)
        VERSION="$(get_version)"
        PKG_NAME="${PROJECT_NAME}-${VERSION}-${OS}-${ARCH}.tar.gz"
        PKG_PATH="${SCRIPT_DIR}/${PKG_NAME}"

        mkdir -p "${SCRIPT_DIR}"
        echo "[*] Packaging ${PKG_PATH}..."

        STAGING="$(mktemp -d)"
        trap 'rm -rf "${STAGING}"' EXIT
        cp "${BINARY_PATH}" "${STAGING}/${BINARY_NAME}"
        cp README.md LICENSE "${STAGING}/"
        cp test.krx "${STAGING}/" 2>/dev/null || true

        tar -czf "${PKG_PATH}" -C "${STAGING}" .
        echo "[+] Package created: ${PKG_PATH}"
        ;;

    "")
        echo ""
        echo "[*] Binary ready. To install system-wide:"
        echo "    ${SCRIPT_DIR}/build.sh install"
        echo ""
        echo "[*] Or run without installing:"
        echo "    ${BINARY_PATH} test.krx"
        ;;

    *)
        echo "[-] Unknown command: ${1}" >&2
        usage >&2
        exit 1
        ;;
esac

echo "[+] Done."
