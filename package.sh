#!/bin/sh
set -eu
cd "$(dirname "$0")"
cargo build --release
mkdir -p Bookland.app/Contents/MacOS
mkdir -p Bookland.app/Contents/Resources
cp Info.plist Bookland.app/Contents/Info.plist
cp "${CARGO_TARGET_DIR:-target}/release/bookland" Bookland.app/Contents/MacOS/bookland
if [ ! -f vendor/yt-dlp_macos ]; then
    mkdir -p vendor
    curl --fail --location --retry 3 \
        --output vendor/yt-dlp_macos \
        'https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp_macos'
    chmod +x vendor/yt-dlp_macos
fi
printf '%s  %s\n' '0f922c15ccc0b83eefa1c8bc9104e434318c4bf16e408e1ee93b7745a6efe0aa' 'vendor/yt-dlp_macos' | shasum -a 256 -c -
cp vendor/yt-dlp_macos Bookland.app/Contents/Resources/yt-dlp_macos
cp THIRD_PARTY.md Bookland.app/Contents/Resources/THIRD_PARTY.md
codesign --force --sign - --timestamp=none Bookland.app/Contents/Resources/yt-dlp_macos
codesign --force --deep --sign - --timestamp=none Bookland.app
