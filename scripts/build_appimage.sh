#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${FEEDIE_VERSION:-0.0.0-dev}"

APPDIR="${ROOT}/dist/AppDir"
DESKTOP_FILE="${ROOT}/installer/linux/feedie.desktop"
ICON_SOURCE="${ROOT}/assets/Feedie_icon.png"
APPIMAGE_NAME="Feedie-linux-x86_64-${VERSION}.AppImage"

rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/feedie/models"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/256x256/apps"

cp "${ROOT}/target/release/app_gui" "${APPDIR}/usr/bin/Feedie"
cp -a "${ROOT}/models/." "${APPDIR}/usr/share/feedie/models/"
cp "${DESKTOP_FILE}" "${APPDIR}/usr/share/applications/"
cp "${ICON_SOURCE}" "${APPDIR}/usr/share/icons/hicolor/256x256/apps/feedie.png"

linuxdeploy \
  --appdir "${APPDIR}" \
  --desktop-file "${APPDIR}/usr/share/applications/feedie.desktop" \
  --icon-file "${APPDIR}/usr/share/icons/hicolor/256x256/apps/feedie.png" \
  --output appimage

OUTPUT="$(ls -1 "${ROOT}"/*.AppImage 2>/dev/null | head -n 1 || true)"
if [[ -z "${OUTPUT}" ]]; then
  echo "AppImage build failed: no output file found." >&2
  exit 1
fi

mkdir -p "${ROOT}/dist"
mv "${OUTPUT}" "${ROOT}/dist/${APPIMAGE_NAME}"
echo "AppImage written to ${ROOT}/dist/${APPIMAGE_NAME}"
