#!/bin/bash
set -e

if [ "$MSYSTEM" != "CLANG64" ]; then
    echo "Run this script from the MSYS2 CLANG64 environment." >&2
    exit 1
fi

echo "=== Updating package database and installing dependencies ==="
pacman -S --noconfirm --needed \
    mingw-w64-clang-x86_64-toolchain \
    mingw-w64-clang-x86_64-gtk4 \
    mingw-w64-clang-x86_64-libadwaita \
    mingw-w64-clang-x86_64-unzip \
    mingw-w64-clang-x86_64-vulkan-loader \
    mingw-w64-clang-x86_64-librsvg \
    mingw-w64-clang-x86_64-adwaita-icon-theme \
    mingw-w64-clang-x86_64-hicolor-icon-theme \
    git \
    zip


# The official gnullvm target uses the same UCRT/LLVM ABI as CLANG64.
if ! command -v rustup &> /dev/null; then
    echo "Install Rust with rustup before building." >&2
    exit 1
fi
rustup target add x86_64-pc-windows-gnullvm

echo "=== Building Tadpole in Release mode ==="
# Ensure compile env variables are set
export PKG_CONFIG_PATH="/clang64/lib/pkgconfig"
export PKG_CONFIG_ALLOW_CROSS=1
export CC_x86_64_pc_windows_gnullvm=clang CXX_x86_64_pc_windows_gnullvm=clang++
export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER=clang
cargo build --release --locked --target x86_64-pc-windows-gnullvm

echo "=== Creating distribution package ==="
DIST_DIR="dist/tadpole-windows"
rm -rf dist
mkdir -p "$DIST_DIR"

# Copy binary
cp target/x86_64-pc-windows-gnullvm/release/tadpole.exe "$DIST_DIR/"

# Copy logo PNG files
cp tadpolelogo.png "$DIST_DIR/"
cp tadpolelogonobg.png "$DIST_DIR/"

# Copy all required DLL dependencies from MSYS2 CLANG64
echo "=== Resolving and copying DLL dependencies ==="
dependencies=$(ldd target/x86_64-pc-windows-gnullvm/release/tadpole.exe | grep /clang64/bin | awk '{print $3}')
for dll in $dependencies; do
    cp "$dll" "$DIST_DIR/"
done

# Copy Vulkan loader (loaded dynamically by GTK4)
echo "=== Copying Vulkan loader ==="
if [ -f "/clang64/bin/vulkan-1.dll" ]; then
    cp "/clang64/bin/vulkan-1.dll" "$DIST_DIR/"
elif [ -f "/clang64/bin/libvulkan-1.dll" ]; then
    cp "/clang64/bin/libvulkan-1.dll" "$DIST_DIR/vulkan-1.dll"
fi


# Copy GSettings schema and compile it
echo "=== Copying GSettings schemas ==="
SCHEMA_DIR="$DIST_DIR/share/glib-2.0/schemas"
mkdir -p "$SCHEMA_DIR"
cp /clang64/share/glib-2.0/schemas/org.gtk.Settings.FileChooser.gschema.xml "$SCHEMA_DIR/" 2>/dev/null || true
cp /clang64/share/glib-2.0/schemas/gschemas.compiled "$SCHEMA_DIR/" 2>/dev/null || true

# Copy icon themes (Adwaita and hicolor)
echo "=== Copying icon themes ==="
mkdir -p "$DIST_DIR/share/icons/Adwaita"
cp /clang64/share/icons/Adwaita/index.theme "$DIST_DIR/share/icons/Adwaita/"
cp -r /clang64/share/icons/Adwaita/scalable "$DIST_DIR/share/icons/Adwaita/"
mkdir -p "$DIST_DIR/share/icons/hicolor"
cp /clang64/share/icons/hicolor/index.theme "$DIST_DIR/share/icons/hicolor/"
cp -r /clang64/share/icons/hicolor/scalable "$DIST_DIR/share/icons/hicolor/"

# Copy gdk-pixbuf SVG loaders
echo "=== Copying gdk-pixbuf loaders ==="
mkdir -p "$DIST_DIR/lib/gdk-pixbuf-2.0/2.10.0/loaders"
cp /clang64/lib/gdk-pixbuf-2.0/2.10.0/loaders/*.dll "$DIST_DIR/lib/gdk-pixbuf-2.0/2.10.0/loaders/"
gdk-pixbuf-query-loaders | sed 's|[^"]*/lib/gdk-pixbuf-2.0/2.10.0/||g' > "$DIST_DIR/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"

# Copy dynamic dependency DLLs for loaders (like librsvg-2-2.dll etc.)
echo "=== Resolving and copying loader DLL dependencies ==="
loader_deps=$(ldd /clang64/lib/gdk-pixbuf-2.0/2.10.0/loaders/*.dll | grep /clang64/bin | awk '{print $3}')
for dll in $loader_deps; do
    cp -n "$dll" "$DIST_DIR/"
done

# Write GTK settings for default icon theme
echo "=== Creating GTK settings ==="
mkdir -p "$DIST_DIR/etc/gtk-4.0"
cat << 'EOF' > "$DIST_DIR/etc/gtk-4.0/settings.ini"
[Settings]
gtk-icon-theme-name=Adwaita
EOF

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
