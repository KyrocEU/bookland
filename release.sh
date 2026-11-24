#!/bin/sh
# Build and verify a local release candidate. Publishing is a separate step.
set -eu
cd "$(dirname "$0")"

version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' Info.plist)
minimum=$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' Info.plist)
bundle_id=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' Info.plist)
case "$version" in
    ''|*[!0-9A-Za-z.-]*) echo 'Invalid app version' >&2; exit 1 ;;
esac
case "$minimum" in
    ''|*[!0-9.]*) echo 'Invalid minimum macOS version' >&2; exit 1 ;;
esac
test "$bundle_id" = 'local.bookland.app' || {
    echo 'Unexpected bundle identifier' >&2; exit 1;
}

cargo test --locked
sh package.sh
codesign --verify --deep --strict Bookland.app

release_dir="$PWD/release"
mkdir -p "$release_dir"
archive="Bookland-$version-macOS-$minimum.zip"
checksum="Bookland-$version-SHA256.txt"
candidate=$(mktemp -d "$release_dir/.candidate.XXXXXX")
trap 'rm -rf "$candidate"' EXIT HUP INT TERM

ditto -c -k --keepParent Bookland.app "$candidate/$archive"
ditto -x -k "$candidate/$archive" "$candidate/extracted"
codesign --verify --deep --strict "$candidate/extracted/Bookland.app"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' \
    "$candidate/extracted/Bookland.app/Contents/Info.plist")" = "$version"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' \
    "$candidate/extracted/Bookland.app/Contents/Info.plist")" = "$bundle_id"

(cd "$candidate" && shasum -a 256 "$archive" > "$checksum")
mv "$candidate/$archive" "$release_dir/$archive"
mv "$candidate/$checksum" "$release_dir/$checksum"
echo "Verified local candidate: $release_dir/$archive"
echo "Checksum: $release_dir/$checksum"
