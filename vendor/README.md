# Video downloader dependency

`yt-dlp_macos` is the official universal macOS standalone release of yt-dlp 2026.10.26, downloaded from:

https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp_macos

SHA-256: `0f922c15ccc0b83eefa1c8bc9104e434318c4bf16e408e1ee93b7745a6efe0aa`

The hash matched the `SHA2-256SUMS` file from the same official release. yt-dlp is distributed under the Unlicense; see the upstream repository for its license and notices. `package.sh` verifies this exact binary before bundling and signing it. Bookland invokes the binary only after the user clicks Download for a recognized YouTube video URL.
