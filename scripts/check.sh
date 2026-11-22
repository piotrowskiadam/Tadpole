#!/usr/bin/env bash
set -euo pipefail

echo "==> Checking Rust formatting"
cargo fmt --all --check

echo "==> Running core tests (no GTK)"
cargo test --locked --no-default-features

echo "==> Running strict core Clippy"
cargo clippy --locked --no-default-features --all-targets -- -D warnings

echo "==> Checking full desktop targets"
cargo check --locked --all-targets

echo "==> Checking version alignment across package metadata"
cargo_ver=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
iss_ver=$(sed -n 's/^AppVersion=\(.*\)/\1/p' setup.iss | head -n 1 | tr -d '\r')
snap_ver=$(sed -n 's/^version: '\''\([^'\'']*\)'\''/\1/p' snap/snapcraft.yaml | head -n 1)
metainfo_ver=$(sed -n 's/.*<release version="\([^"]*\)".*/\1/p' flatpak/com.tadpole.seo.metainfo.xml | head -n 1)

for pair in "setup.iss:$iss_ver" "snap/snapcraft.yaml:$snap_ver" "flatpak/com.tadpole.seo.metainfo.xml:$metainfo_ver"; do
    file="${pair%%:*}"
    ver="${pair#*:}"
    if [[ "$ver" != "$cargo_ver" ]]; then
        echo "ERROR: Version mismatch — Cargo.toml is '$cargo_ver' but $file is '$ver'" >&2
        exit 1
    fi
done
echo "Versions aligned at $cargo_ver"

if command -v appstreamcli >/dev/null 2>&1; then
    echo "==> Validating AppStream metadata"
    appstreamcli validate --no-net flatpak/com.tadpole.seo.metainfo.xml
fi

if command -v desktop-file-validate >/dev/null 2>&1; then
    echo "==> Validating desktop entry"
    desktop-file-validate snap/gui/com.tadpole.seo.desktop
fi

echo "==> All checks passed"
