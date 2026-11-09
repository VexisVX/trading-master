#!/usr/bin/env bash
set -euo pipefail

# Build distributable packages and report sizes.
# Usage:
#   ./build.sh              # build all bundles
#   ./build.sh --clean      # cargo clean first

CLEAN=0
[[ "${1:-}" == "--clean" ]] && CLEAN=1

if [ -f "src-tauri/Cargo.toml" ]; then
  PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then
  PROJECT_DIR="$(pwd)/trading-manager"
else
  echo "ERROR: cannot find project (need src-tauri/Cargo.toml)."
  exit 1
fi
cd "${PROJECT_DIR}"

echo "==> Project: ${PROJECT_DIR}"

if [ "${CLEAN}" == "1" ]; then
  echo "==> Cleaning build artifacts"
  (cd src-tauri && cargo clean)
  rm -rf dist
fi

echo "==> Building release bundles (this may take several minutes)"
npm run tauri build

echo ""
echo "==> Bundles:"
for dir in deb rpm appimage; do
  if ls src-tauri/target/release/bundle/${dir}/* >/dev/null 2>&1; then
    ls -lh src-tauri/target/release/bundle/${dir}/*
  else
    echo "  (no ${dir} artifacts)"
  fi
done

echo ""
echo "==> Binary size:"
if [ -f src-tauri/target/release/trading-manager ]; then
  ls -lh src-tauri/target/release/trading-manager
else
  echo "  (no binary)"
fi

echo ""
echo "==> Release folder usage:"
du -sh src-tauri/target/release 2>/dev/null || true

echo ""
echo "==> Install:"
echo "  sudo pacman -U src-tauri/target/release/bundle/deb/*.deb"
echo "  chmod +x src-tauri/target/release/bundle/appimage/*.AppImage"
echo "  ./src-tauri/target/release/bundle/appimage/*.AppImage"

echo ""
echo "==> User data (not bundled):"
echo "  ~/.local/share/com.trading.manager/app.db"
echo "  ~/.local/share/com.trading.manager/backups/"
