#!/bin/sh
set -eu
cd "$(dirname "$0")"
cargo build --release
cmake -S vendor/mediaremote-adapter -B vendor/mediaremote-adapter/build -DCMAKE_BUILD_TYPE=Release
cmake --build vendor/mediaremote-adapter/build --target MediaRemoteAdapter -j 4
mkdir -p Bookland.app/Contents/MacOS
mkdir -p Bookland.app/Contents/Resources
mkdir -p Bookland.app/Contents/Frameworks
cp Info.plist Bookland.app/Contents/Info.plist
cp "${CARGO_TARGET_DIR:-target}/release/bookland" Bookland.app/Contents/MacOS/bookland
cp assets/brand/Bookland.icns Bookland.app/Contents/Resources/Bookland.icns
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
cp vendor/mediaremote-adapter/bin/mediaremote-adapter.pl Bookland.app/Contents/Resources/mediaremote-adapter.pl
cp vendor/mediaremote-adapter/LICENSE Bookland.app/Contents/Resources/MediaRemoteAdapter-LICENSE
rm -rf Bookland.app/Contents/Frameworks/MediaRemoteAdapter.framework
cp -R vendor/mediaremote-adapter/build/MediaRemoteAdapter.framework Bookland.app/Contents/Frameworks/
codesign --force --sign - --timestamp=none Bookland.app/Contents/Resources/yt-dlp_macos
codesign --force --deep --sign - --timestamp=none Bookland.app
