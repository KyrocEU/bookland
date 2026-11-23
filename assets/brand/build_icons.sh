#!/bin/sh
set -eu
cd "$(dirname "$0")"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/Bookland.iconset"
rsvg-convert -w 1024 -h 1024 -o bookland-icon.png bookland-icon.svg
sips -s format png -z 1024 1024 bookland-icon-premium.png \
    --out "$work/master.png" >/dev/null

for size in 16 32 128 256 512; do
    sips -s format png -z "$size" "$size" "$work/master.png" \
        --out "$work/Bookland.iconset/icon_${size}x${size}.png" >/dev/null
    retina=$((size * 2))
    sips -s format png -z "$retina" "$retina" "$work/master.png" \
        --out "$work/Bookland.iconset/icon_${size}x${size}@2x.png" >/dev/null
done

iconutil -c icns -o Bookland.icns "$work/Bookland.iconset"
rsvg-convert -w 256 -h 256 -o bookland-mark.png bookland-mark.svg
rsvg-convert -w 1280 -h 640 -o social-preview.png bookland-banner.svg
