#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${FEEDIE_VERSION:-0.0.0-dev}"
ARCH="${FEEDIE_APPIMAGE_ARCH:-x86_64}"

APPDIR="${ROOT}/dist/AppDir"
DESKTOP_FILE="${ROOT}/installer/linux/feedie.desktop"
ICON_SOURCE="${ROOT}/assets/Feedie_icon_256.png"
APPIMAGE_NAME="Feedie-linux-${ARCH}-${VERSION}.AppImage"

LINUXDEPLOY="${LINUXDEPLOY:-}"
APPIMAGETOOL="${APPIMAGETOOL:-}"

if [[ -z "${LINUXDEPLOY}" ]]; then
  if command -v linuxdeploy >/dev/null 2>&1; then
    LINUXDEPLOY="linuxdeploy"
  elif command -v linuxdeploy.AppImage >/dev/null 2>&1; then
    LINUXDEPLOY="linuxdeploy.AppImage"
  elif [[ -x "${ROOT}/tools/linuxdeploy.AppImage" ]]; then
    LINUXDEPLOY="${ROOT}/tools/linuxdeploy.AppImage"
  elif [[ -x "${ROOT}/tools/linuxdeploy" ]]; then
    LINUXDEPLOY="${ROOT}/tools/linuxdeploy"
  else
    echo "linuxdeploy not found. Set LINUXDEPLOY or add it to PATH." >&2
    exit 1
  fi
fi

if [[ -z "${APPIMAGETOOL}" ]]; then
  if command -v appimagetool >/dev/null 2>&1; then
    APPIMAGETOOL="appimagetool"
  elif command -v appimagetool.AppImage >/dev/null 2>&1; then
    APPIMAGETOOL="appimagetool.AppImage"
  elif [[ -x "${ROOT}/tools/appimagetool.AppImage" ]]; then
    APPIMAGETOOL="${ROOT}/tools/appimagetool.AppImage"
  elif [[ -x "${ROOT}/tools/appimagetool" ]]; then
    APPIMAGETOOL="${ROOT}/tools/appimagetool"
  fi
fi

if [[ -z "${APPIMAGETOOL}" ]]; then
  echo "appimagetool not found. Set APPIMAGETOOL or add it to PATH." >&2
  exit 1
fi
export APPIMAGETOOL

rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/feedie/models"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/256x256/apps"

cp "${ROOT}/target/release/Feedie" "${APPDIR}/usr/bin/Feedie"
cp -a "${ROOT}/models/." "${APPDIR}/usr/share/feedie/models/"
cp "${DESKTOP_FILE}" "${APPDIR}/usr/share/applications/"
cp "${ICON_SOURCE}" "${APPDIR}/usr/share/icons/hicolor/256x256/apps/feedie.png"

"${LINUXDEPLOY}" \
  --appdir "${APPDIR}" \
  --desktop-file "${APPDIR}/usr/share/applications/feedie.desktop" \
  --icon-file "${APPDIR}/usr/share/icons/hicolor/256x256/apps/feedie.png"

# Use the system libxkbcommon to avoid mismatches with host X11 compose data.
if [[ -d "${APPDIR}/usr/lib" ]]; then
  find "${APPDIR}/usr/lib" -type f -name 'libxkbcommon*.so*' -delete
fi
if [[ -d "${APPDIR}/usr/lib64" ]]; then
  find "${APPDIR}/usr/lib64" -type f -name 'libxkbcommon*.so*' -delete
fi

export ARCH
export VERSION
pushd "${ROOT}" >/dev/null
"${APPIMAGETOOL}" "${APPDIR}"
popd >/dev/null

OUTPUT="$(ls -1 "${ROOT}"/*.AppImage 2>/dev/null | head -n 1 || true)"
if [[ -z "${OUTPUT}" ]]; then
  echo "AppImage build failed: no output file found." >&2
  exit 1
fi

mkdir -p "${ROOT}/dist"
mv "${OUTPUT}" "${ROOT}/dist/${APPIMAGE_NAME}"
echo "AppImage written to ${ROOT}/dist/${APPIMAGE_NAME}"
