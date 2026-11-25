# Bookland 0.1.1 preview

Bookland is a free, MIT-licensed macOS camera-edge island. This preview keeps the same Bookland identity and moves the public build forward from 0.1.0.

## Changes

- Improved browser media controls, including 15-second seeking from the compact Music controls and stable seek position during media refreshes. Failed seeks restore the shown position.
- Added a single-instance guard so launching a second Bookland copy does not leave two islands on the screen.
- Reduced idle redraws and kept the Liquid Glass reveal on its native 60 Hz animation path. The closed activity ring uses Core Animation.
- Refined the Settings sidebar and its icons, Calendar date markers, and the display of the selected calendar when access is unavailable.
- Improved the local Codex activity view and connected the selected turn to the in-island chat. Decisions remain limited to Codex turns connected through Bookland.

## Install

Requires an Apple Silicon Mac running macOS 26 or later. Download the ZIP and SHA-256 file attached to this release, verify the checksum, extract the ZIP, and move `Bookland.app` to Applications or the Desktop. This build is locally signed with a stable Bookland update identity, but it is not Apple-notarized. macOS may show **Open Anyway** on first launch. Calendar, microphone, Bluetooth, and system audio features request their own permissions when used.

## Preview limits

Bookland does not receive approval requests from turns already running in the Codex desktop app. Apple's private MediaRemote framework can change between macOS versions. AirPlay route changes, live AirPods connection cues, voice following, and some other permission-dependent flows have not been verified end to end on every Mac. This is not a complete recreation of every NotchNook feature. Please report the Mac model, macOS version, media player, and exact steps for any issue in [GitHub Issues](https://github.com/KyrocEU/bookland/issues).
