# Bookland 0.1.2 preview

This preview brings the public download up to the locally tested Bookland build.

- Browser media controls in the Nook now seek 15 seconds backward or forward instead of sending previous or next track commands. The Chrome YouTube check confirmed the video position moved from 0:15 to 0:30 and back, and Play/Pause changed its state.
- The native Core Animation island morph now has enough sampled spring frames to settle at the slowest animation speed without a visible final-size jump.

Bookland remains a free, open-source preview for Apple Silicon Macs running macOS 26 or later. It is locally signed and is not Apple-notarized. macOS may require **System Settings → Privacy & Security → Open Anyway** after the first launch attempt. The ZIP includes `Bookland.app`; the companion SHA-256 file lets you verify the download.

The [README](README.md) documents installation, permissions, the current feature set, and known limits. In particular, Codex approvals appear in the island only for turns started through Bookland, and browser media support can vary with macOS and browser updates.
