#!/usr/bin/env bash
# Convert an explicit Tadpole media-mode frame directory; never records a desktop.
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo 'Usage: scripts/media-export.sh FRAME_DIRECTORY OUTPUT.gif|OUTPUT.mp4' >&2
  exit 2
fi
command -v ffmpeg >/dev/null || { echo 'Install FFmpeg first.' >&2; exit 1; }
frames=$(realpath -- "$1")
output=$(realpath -m -- "$2")
[[ -f "$frames/frame-0000.png" ]] || { echo 'No recording frames found.' >&2; exit 1; }
case "$output" in
  *.gif)
    ffmpeg -nostdin -n -framerate 2 -i "$frames/frame-%04d.png" -filter_complex '[0:v]fps=2,scale=1200:-1:force_original_aspect_ratio=decrease:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=sierra2_4a' -loop 0 "$output"
    ;;
  *.mp4)
    ffmpeg -nostdin -n -framerate 2 -i "$frames/frame-%04d.png" -vf 'pad=ceil(iw/2)*2:ceil(ih/2)*2' -c:v libx264 -pix_fmt yuv420p -movflags +faststart "$output"
    ;;
  *) echo 'Choose an output ending in .gif or .mp4.' >&2; exit 2 ;;
esac
