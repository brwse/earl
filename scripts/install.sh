#!/usr/bin/env bash
set -euo pipefail

REPO="${EARL_INSTALL_REPO:-mathematic-inc/earl}"
VERSION="latest"
INSTALL_DIR=""
BIN_DIR=""

usage() {
  cat <<USAGE
Usage: install.sh [options]

Options:
  --version <version>      Version to install (for example 0.1.0 or v0.1.0). Default: latest
  --install-dir <path>     Prefix for installation. Binary is installed into <path>/bin unless --bin-dir is set
  --bin-dir <path>         Directly install the binary into this directory
  -h, --help               Show this help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --bin-dir)
      BIN_DIR="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if command -v curl >/dev/null 2>&1; then
  downloader() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
  downloader() { wget -qO "$2" "$1"; }
else
  echo "curl or wget is required" >&2
  exit 1
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    OS_PART="apple-darwin"
    ;;
  Linux)
    OS_PART="unknown-linux-gnu"
    ;;
  *)
    echo "unsupported OS: $OS" >&2
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64)
    ARCH_PART="x86_64"
    ;;
  aarch64|arm64)
    ARCH_PART="aarch64"
    ;;
  *)
    echo "unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

TARGET="${ARCH_PART}-${OS_PART}"

if [[ "$VERSION" == "latest" ]]; then
  API_URL="https://api.github.com/repos/${REPO}/releases?per_page=100"
  VERSION="$(downloader "$API_URL" /dev/stdout | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\([0-9][^"]*\)".*/\1/p' | head -n1)"
  if [[ -z "$VERSION" ]]; then
    echo "failed to resolve latest version from ${API_URL}" >&2
    exit 1
  fi
fi

VERSION="${VERSION#v}"
TAG="v${VERSION}"
ARCHIVE="earl-${VERSION}-${TARGET}.tar.gz"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

ARCHIVE_URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"
CHECKSUM_URL="https://github.com/${REPO}/releases/download/${TAG}/SHA256SUMS"

echo "Downloading $ARCHIVE_URL"
downloader "$ARCHIVE_URL" "$TMP_DIR/$ARCHIVE"
downloader "$CHECKSUM_URL" "$TMP_DIR/SHA256SUMS"

EXPECTED_HASH="$(awk -v name="$ARCHIVE" '$2 == name { print $1 }' "$TMP_DIR/SHA256SUMS")"
if [[ -z "$EXPECTED_HASH" ]]; then
  echo "checksum entry for $ARCHIVE not found" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL_HASH="$(sha256sum "$TMP_DIR/$ARCHIVE" | awk '{ print $1 }')"
else
  ACTUAL_HASH="$(shasum -a 256 "$TMP_DIR/$ARCHIVE" | awk '{ print $1 }')"
fi

if [[ "$EXPECTED_HASH" != "$ACTUAL_HASH" ]]; then
  echo "checksum verification failed for $ARCHIVE" >&2
  exit 1
fi

tar -C "$TMP_DIR" -xzf "$TMP_DIR/$ARCHIVE"

if [[ -n "$BIN_DIR" ]]; then
  DEST_DIR="$BIN_DIR"
elif [[ -n "$INSTALL_DIR" ]]; then
  DEST_DIR="$INSTALL_DIR/bin"
else
  if [[ "$(id -u)" -eq 0 ]]; then
    DEST_DIR="/usr/local/bin"
  else
    DEST_DIR="$HOME/.local/bin"
  fi
fi

mkdir -p "$DEST_DIR"
install -m 0755 "$TMP_DIR/earl" "$DEST_DIR/earl"

echo "Installed earl ${VERSION} to ${DEST_DIR}/earl"
if [[ ":$PATH:" != *":$DEST_DIR:"* ]]; then
  echo "Add ${DEST_DIR} to your PATH to run 'earl' from any shell."
fi
