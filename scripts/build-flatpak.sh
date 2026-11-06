#!/usr/bin/env bash
# AI-assisted upstream package. Do not reuse as a Flathub submission manifest.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
output=${1:-"$root/dist/tadpole.flatpak"}
mkdir -p "$(dirname "$output")"
output=$(realpath -m "$output")
work=$(mktemp -d "${TMPDIR:-/tmp}/tadpole-flatpak.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
mkdir -p "$work/source/.cargo"
if git -C "$root" rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    (cd "$root" && git ls-files -z --cached --others --exclude-standard --deduplicate | tar --null -T - -cf -) | tar -C "$work/source" -xf -
else
    tar -C "$root" --exclude=./.git --exclude=./target --exclude=./dist --exclude=./.flatpak-builder --exclude=./vendor --exclude=./.env --exclude='./.env.*' --exclude='*.seocrawl' -cf - . | tar -C "$work/source" -xf -
fi
cargo vendor --locked --manifest-path "$work/source/Cargo.toml" "$work/source/vendor" > /dev/null
cat > "$work/source/.cargo/config.toml" <<'CONFIG'
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "/run/build/tadpole/vendor"
CONFIG
flatpak-builder --user --force-clean --disable-rofiles-fuse --state-dir="$work/state" --install-deps-from=flathub --repo="$work/repo" "$work/build" "$work/source/flatpak/com.tadpole.seo.yml"
flatpak build-bundle "$work/repo" "$output" com.tadpole.seo
printf 'Flatpak bundle: %s\n' "$output"
