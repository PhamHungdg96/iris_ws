#!/bin/bash
# IRIS - macOS build helper
# Copies the Rust bridge dylib into the Flutter macOS app bundle.

set -e

PROJECT_DIR="${SRCROOT}/.."
BRIDGE_LIB="${PROJECT_DIR}/native/libiris_bridge.dylib"
FRAMEWORKS_DIR="${BUILT_PRODUCTS_DIR}/${FRAMEWORKS_FOLDER_PATH}"

if [ -f "${BRIDGE_LIB}" ]; then
    echo "Copying iris_bridge dylib to ${FRAMEWORKS_DIR}"
    cp "${BRIDGE_LIB}" "${FRAMEWORKS_DIR}/"
    # Update install name for code signing
    install_name_tool -id "@rpath/libiris_bridge.dylib" "${FRAMEWORKS_DIR}/libiris_bridge.dylib" 2>/dev/null || true
else
    echo "WARNING: iris_bridge dylib not found at ${BRIDGE_LIB}"
    echo "Run ./build.sh first to compile the Rust library."
fi
