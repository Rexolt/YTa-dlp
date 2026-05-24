#!/usr/bin/env bash

# Exit immediately if any command fails
set -e

# Colored output formatting
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}====================================================${NC}"
echo -e "${BLUE}          YTa-dlp Linux Production Builder          ${NC}"
echo -e "${BLUE}====================================================${NC}"

# 1. System dependency helper reminder (Ubuntu/Debian)
echo -e "\n${CYAN}[1/5] Checking development tools...${NC}"
if [ -f /etc/debian_version ]; then
    echo -e "${YELLOW}Note: If compilation fails, make sure you have installed the required system headers:${NC}"
    echo -e "${YELLOW}sudo apt update && sudo apt install -y build-essential curl wget file libssl-dev pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsqlite3-dev libayatana-appindicator3-dev librsvg2-dev${NC}"
fi

# 2. Install Svelte frontend dependencies
echo -e "\n${CYAN}[2/5] Installing package dependencies via pnpm...${NC}"
pnpm install

# 3. Ensure sidecar binaries are fetched
echo -e "\n${CYAN}[3/5] Fetching sidecar binaries (yt-dlp, ffmpeg, ffprobe)...${NC}"
pnpm run binaries:fetch

# 4. Compile Svelte & Tauri Application
echo -e "\n${CYAN}[4/5] Building Svelte frontend & Tauri Rust bundle...${NC}"
pnpm tauri build

# 5. Locate and present the build packages
echo -e "\n${CYAN}[5/5] Locating output packages...${NC}"
DEB_DIR="src-tauri/target/release/bundle/deb"
APPIMAGE_DIR="src-tauri/target/release/bundle/appimage"

FOUND_ANY=false

echo -e "${GREEN}Build Completed Successfully!${NC}"
echo -e "\n${BLUE}Resulting Linux Packages:${NC}"

if [ -d "$DEB_DIR" ]; then
    DEB_FILE=$(find "$DEB_DIR" -name "*.deb" | head -n 1)
    if [ -n "$DEB_FILE" ]; then
        echo -e "${GREEN}✓ Debian/Ubuntu package (.deb):${NC}"
        echo -e "  $(realpath "$DEB_FILE")"
        FOUND_ANY=true
    fi
fi

if [ -d "$APPIMAGE_DIR" ]; then
    APPIMAGE_FILE=$(find "$APPIMAGE_DIR" -name "*.AppImage" | head -n 1)
    if [ -n "$APPIMAGE_FILE" ]; then
        echo -e "${GREEN}✓ Portable executable (.AppImage) - runs on any Linux distribution:${NC}"
        echo -e "  $(realpath "$APPIMAGE_FILE")"
        FOUND_ANY=true
    fi
fi

if [ "$FOUND_ANY" = false ]; then
    echo -e "${YELLOW}Could not automatically locate the packages, but check your 'src-tauri/target/release/bundle/' directory.${NC}"
fi

echo -e "\n${BLUE}====================================================${NC}"
