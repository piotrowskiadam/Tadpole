#!/bin/bash
set -e

echo "=== Building Tadpole in Release mode ==="
cargo build --release

echo "=== Structuring AppDir ==="
APPDIR="Tadpole.AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/pixmaps"

# Copy binary
cp target/release/tadpole "$APPDIR/usr/bin/tadpole"

# Copy desktop file and icons
cp snap/gui/com.tadpole.seo.desktop "$APPDIR/com.tadpole.seo.desktop"
cp snap/gui/com.tadpole.seo.desktop "$APPDIR/usr/share/applications/com.tadpole.seo.desktop"
cp tadpolelogonobg.png "$APPDIR/com.tadpole.seo.png"
cp tadpolelogonobg.png "$APPDIR/.DirIcon"
cp tadpolelogonobg.png "$APPDIR/usr/share/pixmaps/com.tadpole.seo.png"

# Create AppRun script
cat << 'EOF' > "$APPDIR/AppRun"
#!/bin/sh
SELF=$(readlink -f "$0")
HERE=$(dirname "$SELF")
export PATH="${HERE}/usr/bin:${PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS}"
exec "${HERE}/usr/bin/tadpole" "$@"
EOF
chmod +x "$APPDIR/AppRun"

# Download appimagetool if not found
if ! command -v appimagetool &> /dev/null; then
    echo "=== Downloading appimagetool ==="
    curl -Lo appimagetool https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x appimagetool
    APPIMAGETOOL="./appimagetool"
else
    APPIMAGETOOL="appimagetool"
fi

# Run appimagetool (using ARCH=x86_64 environment variable)
echo "=== Generating AppImage ==="
export ARCH=x86_64

# Workaround for running AppImage inside Docker/GitHub Actions without FUSE
if [ "$GITHUB_ACTIONS" = "true" ]; then
    $APPIMAGETOOL --appimage-extract-and-run "$APPDIR" Tadpole-x86_64.AppImage
else
    $APPIMAGETOOL "$APPDIR" Tadpole-x86_64.AppImage
fi

echo "=== AppImage generation complete: Tadpole-x86_64.AppImage ==="
