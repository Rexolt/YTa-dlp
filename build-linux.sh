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

PROJECT_ROOT=$(pwd)

echo -e "${BLUE}====================================================${NC}"
echo -e "${BLUE}          YTa-dlp Linux Production Builder          ${NC}"
echo -e "${BLUE}====================================================${NC}"

# 1. System dependency helper reminder (Ubuntu/Debian)
echo -e "\n${CYAN}[1/5] Checking development tools...${NC}"
if [ -f /etc/debian_version ]; then
    echo -e "${YELLOW}Note: If compilation fails, make sure you have installed the required system headers:${NC}"
    echo -e "${YELLOW}sudo apt update && sudo apt install -y build-essential curl wget file libssl-dev pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libsqlite3-dev libayatana-appindicator3-dev librsvg2-dev rpm zstd${NC}"
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

# 4.5. Build Arch Linux pacman package (.pkg.tar.zst) from the built .deb package
DEB_DIR="src-tauri/target/release/bundle/deb"
ARCH_DIR="src-tauri/target/release/bundle/pacman"

DEB_FILE=$(find "$DEB_DIR" -name "*.deb" | head -n 1 || true)
if [ -n "$DEB_FILE" ]; then
    echo -e "\n${CYAN}Building Arch Linux Pacman package (.pkg.tar.zst) from .deb...${NC}"
    rm -rf "$ARCH_DIR"
    mkdir -p "$ARCH_DIR"
    
    TEMP_PKG=$(mktemp -d)
    cp "$DEB_FILE" "$TEMP_PKG/"
    
    (
        cd "$TEMP_PKG"
        # Extract the deb package control and data archives
        ar x *.deb
        
        DATA_TAR=$(find . -name "data.tar.*" | head -n 1)
        if [ -n "$DATA_TAR" ]; then
            mkdir -p pkg
            tar -xf "$DATA_TAR" -C pkg/
            
            # Write Arch Linux .PKGINFO metadata
            cat <<EOF > pkg/.PKGINFO
pkgname = yta-dlp
pkgver = 0.1.0
pkgrel = 1
pkgdesc = premium downloader · powered by yt-dlp
url = https://github.com/Rexolt/YTa-dlp
arch = x86_64
license = MIT
depend = gtk3
depend = webkit2gtk-4.1
depend = libsqlite3
EOF
            
            # Compress to Arch Linux standard package format (.pkg.tar.zst)
            (
                cd pkg
                tar -c --zstd -f ../yta-dlp-0.1.0-1-x86_64.pkg.tar.zst .PKGINFO usr/
            )
            
            mv yta-dlp-0.1.0-1-x86_64.pkg.tar.zst "$PROJECT_ROOT/$ARCH_DIR/"
            echo -e "${GREEN}✓ Arch Linux Pacman package (.pkg.tar.zst) created!${NC}"
        else
            echo -e "${RED}✗ Failed to parse deb package structure${NC}"
        fi
    )
    
    rm -rf "$TEMP_PKG"
fi

# 5. Locate and present the build packages
echo -e "\n${CYAN}[5/5] Locating output packages...${NC}"
APPIMAGE_DIR="src-tauri/target/release/bundle/appimage"
RPM_DIR="src-tauri/target/release/bundle/rpm"

FOUND_ANY=false

echo -e "${GREEN}Build Completed Successfully!${NC}"
echo -e "\n${BLUE}Resulting Linux Packages:${NC}"

if [ -f "$DEB_FILE" ]; then
    echo -e "${GREEN}✓ Debian/Ubuntu package (.deb):${NC}"
    echo -e "  $(realpath "$DEB_FILE")"
    FOUND_ANY=true
fi

if [ -d "$APPIMAGE_DIR" ]; then
    APPIMAGE_FILE=$(find "$APPIMAGE_DIR" -name "*.AppImage" | head -n 1 || true)
    if [ -n "$APPIMAGE_FILE" ]; then
        echo -e "${GREEN}✓ Portable executable (.AppImage) - runs on any Linux distribution:${NC}"
        echo -e "  $(realpath "$APPIMAGE_FILE")"
        FOUND_ANY=true
    fi
fi

if [ -d "$RPM_DIR" ]; then
    RPM_FILE=$(find "$RPM_DIR" -name "*.rpm" | head -n 1 || true)
    if [ -n "$RPM_FILE" ]; then
        echo -e "${GREEN}✓ Fedora/RHEL/openSUSE package (.rpm):${NC}"
        echo -e "  $(realpath "$RPM_FILE")"
        FOUND_ANY=true
    fi
fi

if [ -d "$ARCH_DIR" ]; then
    PACMAN_FILE=$(find "$ARCH_DIR" -name "*.pkg.tar.zst" | head -n 1 || true)
    if [ -n "$PACMAN_FILE" ]; then
        echo -e "${GREEN}✓ Arch Linux Pacman package (.pkg.tar.zst):${NC}"
        echo -e "  $(realpath "$PACMAN_FILE")"
        FOUND_ANY=true
    fi
fi

if [ "$FOUND_ANY" = false ]; then
    echo -e "${YELLOW}Could not automatically locate the packages, but check your 'src-tauri/target/release/bundle/' directory.${NC}"
fi

echo -e "\n${BLUE}====================================================${NC}"
