#!/bin/bash
set -e

echo "=== Updating package database and installing dependencies ==="
pacman -S --noconfirm --needed \
    mingw-w64-x86_64-toolchain \
    mingw-w64-x86_64-gtk4 \
    mingw-w64-x86_64-libadwaita \
    mingw-w64-x86_64-pkg-config \
    mingw-w64-x86_64-unzip \
    mingw-w64-x86_64-vulkan-loader \
    git \
    zip


# Check if rustup / cargo is installed, if not install it
if ! command -v cargo &> /dev/null; then
    echo "Rust/Cargo not found. Installing via pacman..."
    pacman -S --noconfirm --needed mingw-w64-x86_64-rust
fi

echo "=== Building Tadpole in Release mode ==="
# Ensure compile env variables are set
export PKG_CONFIG_PATH="/mingw64/lib/pkgconfig"
cargo build --release

echo "=== Creating distribution package ==="
DIST_DIR="dist/tadpole-windows"
rm -rf dist
mkdir -p "$DIST_DIR"

# Copy binary
cp target/release/tadpole.exe "$DIST_DIR/"

# Copy logo PNG files
cp tadpolelogo.png "$DIST_DIR/"
cp tadpolelogonobg.png "$DIST_DIR/"

# Copy all required DLL dependencies from MSYS2 MinGW64
echo "=== Resolving and copying DLL dependencies ==="
dependencies=$(ldd target/release/tadpole.exe | grep /mingw64/bin | awk '{print $3}')
for dll in $dependencies; do
    cp "$dll" "$DIST_DIR/"
done

# Copy Vulkan loader (loaded dynamically by GTK4)
echo "=== Copying Vulkan loader ==="
if [ -f "/mingw64/bin/vulkan-1.dll" ]; then
    cp "/mingw64/bin/vulkan-1.dll" "$DIST_DIR/"
elif [ -f "/mingw64/bin/libvulkan-1.dll" ]; then
    cp "/mingw64/bin/libvulkan-1.dll" "$DIST_DIR/vulkan-1.dll"
fi


# Copy GSettings schema and compile it
echo "=== Copying GSettings schemas ==="
SCHEMA_DIR="$DIST_DIR/share/glib-2.0/schemas"
mkdir -p "$SCHEMA_DIR"
cp /mingw64/share/glib-2.0/schemas/org.gtk.Settings.FileChooser.gschema.xml "$SCHEMA_DIR/" 2>/dev/null || true
cp /mingw64/share/glib-2.0/schemas/gschemas.compiled "$SCHEMA_DIR/" 2>/dev/null || true

# Zip the release
echo "=== Packaging into tadpole-windows.zip ==="
cd dist
zip -r tadpole-windows.zip tadpole-windows/
cd ..

# Run Inno Setup Compiler if available on system
if command -v iscc &> /dev/null; then
    echo "=== Building Windows Installer (TadpoleSetup.exe) ==="
    iscc setup.iss
elif [ -f "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" ]; then
    echo "=== Building Windows Installer (TadpoleSetup.exe) ==="
    "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" setup.iss
else
    echo "Inno Setup Compiler (ISCC) not found, skipping installer generation."
fi

echo "=== Build and Packaging complete: dist/tadpole-windows.zip ==="
