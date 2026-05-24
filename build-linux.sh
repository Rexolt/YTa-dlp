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

# Read and increment local build number
BUILD_NUM_FILE=".build_number"
if [ -f "$BUILD_NUM_FILE" ]; then
    BUILD_NUM=$(cat "$BUILD_NUM_FILE")
    BUILD_NUM=$((BUILD_NUM + 1))
else
    BUILD_NUM=1
fi
echo "$BUILD_NUM" > "$BUILD_NUM_FILE"

# Parse application version from package.json
VERSION=$(node -p "require('./package.json').version")

echo -e "${BLUE}====================================================${NC}"
echo -e "${BLUE}          YTa-dlp Linux Production Builder          ${NC}"
echo -e "${BLUE}  Version: ${VERSION}  |  Build Number: ${BUILD_NUM}${NC}"
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
# Some bundling steps like AppImage require FUSE (libfuse2) on the host to execute linuxdeploy.
# If they fail, we log a warning but continue so that the built DEB and Pacman packages are preserved.
pnpm tauri build --bundles deb,appimage || echo -e "${YELLOW}⚠ AppImage packaging failed (usually because 'libfuse2' is missing on host). Continuing with DEB and Pacman packaging...${NC}"

# 4.5. Build Arch Linux pacman package (.pkg.tar.zst) and Fedora RPM package (.rpm) from the built .deb package
DEB_DIR="src-tauri/target/release/bundle/deb"
ARCH_DIR="src-tauri/target/release/bundle/pacman"
RPM_DIR="src-tauri/target/release/bundle/rpm"

# Clean up existing RPM and Pacman directories to prevent stale or conflicted packages
rm -rf "$ARCH_DIR"
rm -rf "$RPM_DIR"

DEB_FILE=$(find "$DEB_DIR" -name "*.deb" | head -n 1 || true)
if [ -n "$DEB_FILE" ]; then
    echo -e "\n${CYAN}Building Arch Linux Pacman package (.pkg.tar.zst) from .deb...${NC}"
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
            
            # Remove redundant bundled binaries to prevent file conflicts with system packages
            rm -f pkg/usr/bin/ffmpeg
            rm -f pkg/usr/bin/ffprobe
            rm -f pkg/usr/bin/yt-dlp
            
            # Write Arch Linux .PKGINFO metadata
            cat <<EOF > pkg/.PKGINFO
pkgname = yta-dlp
pkgver = ${VERSION}-${BUILD_NUM}
pkgdesc = premium downloader · powered by yt-dlp
url = https://github.com/Rexolt/YTa-dlp
arch = x86_64
license = MIT
depend = gtk3
depend = webkit2gtk-4.1
depend = sqlite
depend = ffmpeg
depend = yt-dlp
EOF
            
            # Compress to Arch Linux standard package format (.pkg.tar.zst) and copy to .pacman
            (
                cd pkg
                tar -c --zstd -f ../yta-dlp-${VERSION}-${BUILD_NUM}-x86_64.pkg.tar.zst .PKGINFO usr/
                cp ../yta-dlp-${VERSION}-${BUILD_NUM}-x86_64.pkg.tar.zst ../yta-dlp-${VERSION}-${BUILD_NUM}-x86_64.pacman
            )
            
            mv yta-dlp-${VERSION}-${BUILD_NUM}-x86_64.pkg.tar.zst "$PROJECT_ROOT/$ARCH_DIR/"
            mv yta-dlp-${VERSION}-${BUILD_NUM}-x86_64.pacman "$PROJECT_ROOT/$ARCH_DIR/"
            echo -e "${GREEN}✓ Arch Linux Pacman package (.pkg.tar.zst & .pacman) created!${NC}"
            
            # Build Fedora RPM package if rpmbuild is available
            if command -v rpmbuild >/dev/null 2>&1; then
                echo -e "\n${CYAN}Building Fedora RPM package (.rpm) from .deb...${NC}"
                mkdir -p "$PROJECT_ROOT/$RPM_DIR"
                
                RPM_BUILD_DIR=$(mktemp -d)
                mkdir -p "$RPM_BUILD_DIR"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
                
                # Copy the extracted files (without sidecars) to SOURCES/pkg
                cp -r pkg "$RPM_BUILD_DIR/SOURCES/"
                
                # Write RPM spec file
                cat <<EOF > "$RPM_BUILD_DIR/SPECS/yta-dlp.spec"
Name:           yta-dlp
Version:        ${VERSION}
Release:        ${BUILD_NUM}%{?dist}
Summary:        premium downloader · powered by yt-dlp
License:        MIT
URL:            https://github.com/Rexolt/YTa-dlp
Requires:       gtk3
Requires:       webkit2gtk4.1
Requires:       sqlite
Requires:       /usr/bin/ffmpeg
Requires:       /usr/bin/ffprobe
Requires:       /usr/bin/yt-dlp

%description
premium downloader · powered by yt-dlp

%install
mkdir -p %{buildroot}
cp -r %{_sourcedir}/pkg/* %{buildroot}/

%files
/usr/bin/yta-dlp
/usr/share/applications/YTa-dlp.desktop
/usr/share/icons/hicolor/*/apps/yta-dlp.png
EOF

                # Run rpmbuild
                rpmbuild --define "_topdir $RPM_BUILD_DIR" -bb "$RPM_BUILD_DIR/SPECS/yta-dlp.spec"
                
                # Copy the generated RPM to target directory
                GENERATED_RPM=$(find "$RPM_BUILD_DIR/RPMS" -name "*.rpm" | head -n 1 || true)
                if [ -n "$GENERATED_RPM" ]; then
                    cp "$GENERATED_RPM" "$PROJECT_ROOT/$RPM_DIR/yta-dlp-${VERSION}-${BUILD_NUM}.x86_64.rpm"
                    echo -e "${GREEN}✓ Fedora RPM package (.rpm) created!${NC}"
                else
                    echo -e "${RED}✗ Failed to locate generated RPM package${NC}"
                fi
                
                rm -rf "$RPM_BUILD_DIR"
            else
                echo -e "\n${YELLOW}⚠ 'rpmbuild' not found. Skipping Fedora RPM package creation. Install 'rpm-build' to compile RPMs locally.${NC}"
            fi
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
    PACMAN_EXT_FILE=$(find "$ARCH_DIR" -name "*.pacman" | head -n 1 || true)
    if [ -n "$PACMAN_EXT_FILE" ]; then
        echo -e "${GREEN}✓ Arch Linux direct package (.pacman):${NC}"
        echo -e "  $(realpath "$PACMAN_EXT_FILE")"
    fi
fi

if [ "$FOUND_ANY" = false ]; then
    echo -e "${YELLOW}Could not automatically locate the packages, but check your 'src-tauri/target/release/bundle/' directory.${NC}"
fi

echo -e "\n${BLUE}====================================================${NC}"
