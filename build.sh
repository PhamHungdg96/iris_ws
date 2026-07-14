#!/bin/bash
# IRIS Project - Build Script
# Compiles the Rust native library for all target platforms
# and prepares Flutter FFI integration.

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$PROJECT_ROOT/iris_core"

echo "=== IRIS Build Script ==="
echo "Project root: $PROJECT_ROOT"

# ── Detect host platform ──
OS="$(uname -s)"
case "$OS" in
    Linux*)     HOST_OS="linux" ;;
    Darwin*)    HOST_OS="macos" ;;
    MINGW*|MSYS*|CYGWIN*) HOST_OS="windows" ;;
    *)          echo "Unknown OS: $OS"; exit 1 ;;
esac

echo "Host platform: $HOST_OS"

# ── Build Rust core ──
echo ""
echo "--- Building iris_core (Rust) ---"

cd "$RUST_DIR"

# Build for host platform
echo "Building for $HOST_OS..."
cargo build --release

# Determine library extension
LIB_EXT=""
case "$HOST_OS" in
    linux)   LIB_EXT=".so" ;;
    macos)   LIB_EXT=".dylib" ;;
    windows) LIB_EXT=".dll" ;;
esac

LIB_NAME="libiris_core$LIB_EXT"
echo "Built: target/release/$LIB_NAME"

# ── Copy to Flutter native directory ──
FLUTTER_NATIVE="$PROJECT_ROOT/iris_flutter/native"
mkdir -p "$FLUTTER_NATIVE"
cp "target/release/$LIB_NAME" "$FLUTTER_NATIVE/"

echo "Copied to: $FLUTTER_NATIVE/$LIB_NAME"

# ── Build for Android (if NDK is available) ──
build_android() {
    echo ""
    echo "--- Building for Android ---"

    if [ -z "${ANDROID_NDK_HOME:-}" ]; then
        echo "ANDROID_NDK_HOME not set, skipping Android build."
        echo "Set ANDROID_NDK_HOME and re-run for Android support."
        return
    fi

    # Android targets
    TARGETS=(
        "aarch64-linux-android"
        "armv7-linux-androideabi"
        "x86_64-linux-android"
        "i686-linux-android"
    )

    for target in "${TARGETS[@]}"; do
        echo "  Building for $target..."
        rustup target add "$target" 2>/dev/null || true
        cargo build --release --target "$target"
    done

    # Copy to Flutter android JNI libs
    ANDROID_LIBS="$PROJECT_ROOT/iris_flutter/android/app/src/main/jniLibs"
    declare -A TARGET_MAP=(
        ["aarch64-linux-android"]="arm64-v8a"
        ["armv7-linux-androideabi"]="armeabi-v7a"
        ["x86_64-linux-android"]="x86_64"
        ["i686-linux-android"]="x86"
    )

    for target in "${!TARGET_MAP[@]}"; do
        abi="${TARGET_MAP[$target]}"
        mkdir -p "$ANDROID_LIBS/$abi"
        cp "target/$target/release/libiris_core.so" "$ANDROID_LIBS/$abi/"
        echo "  Copied $target -> $ANDROID_LIBS/$abi/"
    done
}

# Uncomment to enable Android builds in CI
# build_android

echo ""
echo "=== Build complete ==="
echo ""
echo "Next steps:"
echo "  1. cd iris_flutter"
echo "  2. flutter pub get"
echo "  3. flutter run -d windows   # or macos, android"
