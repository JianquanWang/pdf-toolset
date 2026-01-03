#!/bin/bash

# PDF Toolset .deb Packaging Script
# This script builds the release binary and packages it into a .deb file for Ubuntu.

set -e

# Configuration
PACKAGE_NAME="pdf-tools"
VERSION="1.0.0"
ARCH="amd64"
MAINTAINER="Your Name <your.email@example.com>"
DESCRIPTION="PDF Toolset - A desktop app for PDF operations built with Rust and GTK4"

# Build the release binary
echo "Building release binary..."
cargo build --release

# Create package directory structure
PACKAGE_DIR="${PACKAGE_NAME}_${VERSION}_${ARCH}"
echo "Creating package structure in $PACKAGE_DIR..."

mkdir -p "$PACKAGE_DIR/DEBIAN"
mkdir -p "$PACKAGE_DIR/usr/bin"
mkdir -p "$PACKAGE_DIR/usr/share/applications"
mkdir -p "$PACKAGE_DIR/usr/share/icons/hicolor/256x256/apps"

# Copy binary
cp "target/release/$PACKAGE_NAME" "$PACKAGE_DIR/usr/bin/"

# Create control file
cat > "$PACKAGE_DIR/DEBIAN/control" << EOF
Package: $PACKAGE_NAME
Version: $VERSION
Architecture: $ARCH
Maintainer: $MAINTAINER
Description: $DESCRIPTION
Depends: libgtk-4-1, poppler-utils, imagemagick
EOF

# Create desktop file
cat > "$PACKAGE_DIR/usr/share/applications/$PACKAGE_NAME.desktop" << EOF
[Desktop Entry]
Name=PDF Toolset
Comment=PDF manipulation tool
Exec=$PACKAGE_NAME
Icon=$PACKAGE_NAME
Terminal=false
Type=Application
Categories=Utility;
EOF

# Create a simple icon
echo "Generating icon..."
convert -size 256x256 xc:#4A90E2 -gravity center -pointsize 72 -fill white -annotate +0+0 "PDF" "$PACKAGE_DIR/usr/share/icons/hicolor/256x256/apps/$PACKAGE_NAME.png"

# Build the .deb package
echo "Building .deb package..."
dpkg-deb --build "$PACKAGE_DIR"

echo "Package created: ${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"

# Optional: Install locally
read -p "Do you want to install the package locally? (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    sudo dpkg -i "${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
    sudo apt install -f
    echo "Package installed. You can now launch PDF Toolset from Show Apps."
fi

# Clean up
rm -rf "$PACKAGE_DIR"