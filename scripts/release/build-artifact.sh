#!/usr/bin/env bash
set -euo pipefail

TARGET=""
VERSION=""
BUILD_TOOL="cargo"
OUTPUT_DIR="dist"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="$2"
      shift 2
      ;;
    --version)
      VERSION="$2"
      shift 2
      ;;
    --build-tool)
      BUILD_TOOL="$2"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$TARGET" || -z "$VERSION" ]]; then
  echo "usage: build-artifact.sh --target <target> --version <version> [--build-tool <cargo|cross|xwin>] [--output-dir <dir>]" >&2
  exit 1
fi

rustup target add "$TARGET"

FEATURES_FLAG=""
if [[ "$TARGET" == *"windows"* ]]; then
  FEATURES_FLAG="--no-default-features --features http,graphql,grpc,sql"
elif [[ "$TARGET" == *"musl"* || "$TARGET" == "aarch64-unknown-linux-gnu" ]]; then
  FEATURES_FLAG="--no-default-features --features http,graphql,grpc,bash,sql"
fi

case "$BUILD_TOOL" in
  cargo)
    cargo build --locked --release --target "$TARGET" $FEATURES_FLAG
    ;;
  cross)
    cross build --locked --release --target "$TARGET" $FEATURES_FLAG
    ;;
  xwin)
    cargo xwin build --locked --release --target "$TARGET" $FEATURES_FLAG
    ;;
  *)
    echo "unsupported build tool: $BUILD_TOOL" >&2
    exit 1
    ;;
esac

BINARY_NAME="earl"
if [[ "$TARGET" == *"windows"* ]]; then
  BINARY_NAME="earl.exe"
fi

BINARY_PATH="target/$TARGET/release/$BINARY_NAME"
if [[ ! -f "$BINARY_PATH" ]]; then
  echo "expected binary not found: $BINARY_PATH" >&2
  exit 1
fi

HOST_TRIPLE="$(rustc -vV | awk '/host:/ { print $2 }')"
if [[ "$TARGET" == "$HOST_TRIPLE" ]]; then
  "$BINARY_PATH" --version >/dev/null
elif [[ "$TARGET" == "x86_64-unknown-linux-musl" && "$HOST_TRIPLE" == "x86_64-unknown-linux-gnu" ]]; then
  "$BINARY_PATH" --version >/dev/null
fi

mkdir -p "$OUTPUT_DIR"

if [[ "$TARGET" == *"windows"* ]]; then
  ARCHIVE="earl-${VERSION}-${TARGET}.zip"
  (
    cd "target/$TARGET/release"
    zip -q "$OLDPWD/$OUTPUT_DIR/$ARCHIVE" "$BINARY_NAME"
  )
else
  ARCHIVE="earl-${VERSION}-${TARGET}.tar.gz"
  tar -C "target/$TARGET/release" -czf "$OUTPUT_DIR/$ARCHIVE" "$BINARY_NAME"
fi

echo "created $OUTPUT_DIR/$ARCHIVE"
