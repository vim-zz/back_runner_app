#!/bin/bash

# Configuration
APP_NAME="BackRunner"
IDENTIFIER="com.1000Ants.BackRunner"
RELEASE_BUILD="target/release/BackRunner"

# Create app bundle structure
APP_BUNDLE="$APP_NAME.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

# Create directories
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# Copy binary
cp "$RELEASE_BUILD" "$MACOS_DIR/$APP_NAME"

# Copy Info.plist
cp "Info.plist" "$CONTENTS_DIR/"

# Create icns file if you have one
# cp "Icon.icns" "$RESOURCES_DIR/"

# Set permissions
chmod +x "$MACOS_DIR/$APP_NAME"
