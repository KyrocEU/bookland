![Bookland, a living island for your Mac](assets/brand/bookland-launch-hero.png)

# Bookland

[Website](https://bookland-mac-island.moshe99871.chatgpt.site) · [Source repository](https://github.com/KyrocEU/bookland)

**A macOS hover island for media, everyday tools, and Codex.** Bookland lives at the top edge of the screen as a small black notch. Hover to open an adaptive Liquid Glass panel that closes after the pointer leaves, or click to keep it open until you press Escape. Choose a workspace in the floating pill. The interface is in English and can display right-to-left music metadata, including Hebrew. Its premium open-book icon appears in the app and island menu, and on the idle strip.

Bookland is free and open source, with no Bookland account, trial, or subscription. It is an independent project inspired by the hover island in NotchNook. It is an early source release, not a copy of NotchNook's code or a claim that every original feature is complete. Bookland is not affiliated with NotchNook or OpenAI.

## What the island does

| Area | What is available |
| --- | --- |
| Hover shell | Closed top-edge strip, hover and click opening, close on pointer leave, workspace pill, touchpad scroll gestures, optional haptic feedback, configurable appearance and timing. |
| Nook | Media controls and artwork, selectable CPU/RAM/GPU/Battery/Disk Pulse cards, Pomodoro and stopwatch, To-dos, and optional widgets for notes, downloads, clipboard, weather, terminal, and camera mirror. |
| Music | Current macOS media session, including supported browser playback; Apple Music or Spotify can be selected as sources. There are playback controls, a seek bar, optional synced lyrics, an on-demand Apple Music playlist preview under Playing Next, Apple Music AirPlay output controls, system volume, and AirPods connection and battery readings when macOS reports them. A brief closed-island cue appears after a detected AirPods connection change. Song and artist labels use bidirectional text ordering. |
| Calendar | A month grid with event dots and day filtering, plus upcoming events from macOS Calendar, including synced Google calendars. Hebrew titles retain right-to-left reading order beside English dates. Select a calendar to create events in that account after granting Bookland Calendar access. |
| Tray and Pocket | Recent files, a floating drag-and-drop Pocket, independent file copies, ZIP archives, image conversion to PNG, JPEG, HEIC, TIFF, or lossless WebP, audio conversion to AIFF, ALAC, or FLAC using macOS tools, document conversion to DOCX or HTML using macOS textutil, AirDrop handoff, and user-initiated YouTube video downloads through yt-dlp. WebP can also be used as an image input. |
| Clipboard | Searchable text, link, color, image, and Finder-file cards with favorites and privacy controls. Optional local persistence excludes image payloads. |
| Agents | Recent local Codex, Claude Code, and optional external activity feed; task progress, usage information when available, and a compact decision view for Bookland-connected Codex turns. |
| Prompts and Prompter | An editable spreadsheet-chat prompt builder and a script prompter with scroll or voice-following modes. Both can be added to the island menu. |
| Live Activities | Timer completion cue, file conversion progress and a six-second result cue in the closed island, and an optional native Liquid Glass volume card. Core Audio change events trigger the volume card, and Core Animation animates its fill and fade without idle polling. |

The default Glass Nook is a single wide row. Music, Calendar, Tray, and the optional workspaces open as compact top-edge panels. The Agents hover panel uses a compact Progress/Usage layout. Its activity bubble opens the Codex app for ordinary desktop tasks; if a Bookland-connected turn asks for a decision, the bubble opens that request inside the same island. On Macs with a camera notch, the closed strip measures the cutout and leaves space for its status symbols on either side. Appearance settings can target a connected display; a display without a camera cutout gets a rounded synthetic island at its top edge. Bookland currently shows one island on the selected display, rather than an island on every display at once.

Settings → General → Onboarding tour opens a seven-page walkthrough of the island and its main workspaces. Each page can open the corresponding workspace directly; the final page links to Appearance settings.

### Google Calendar on your Mac

Bookland uses macOS Calendar's account connection. In the Calendar app, choose **Calendar → Add Account → Google** and finish Google's sign-in. If the account is already in **System Settings → Internet Accounts**, check that **Calendars** is on and complete any Google reauthentication request. Then open Bookland's Calendar workspace, choose **Connect Calendar** to grant macOS Calendar access, and select the Google calendar under **Calendar Source**. Bookland shows events from that selection and saves new events to the selected writable calendar. It searches for events on a background queue while the Calendar workspace is open. The Google Calendar connection in Codex or ChatGPT is separate and does not grant Bookland access. Bookland does not store a Google password or OAuth token.

The closed activity ring and the larger Agents Usage arc run in a macOS Core Animation layer so they move between Bookland's low-frequency idle redraws. Active file conversions, video downloads, and watched-folder downloads can appear in the closed island; opening their status goes to Tray. File conversion results appear briefly and stop drawing when dismissed or expired. Their native spin stops when hidden or when macOS Reduce Motion is enabled. The opening and closing outline uses sampled spring paths in a native Core Animation shape layer. A hover event selects the open target before the first spring step. The host display link requests 30 fps while only that outline moves and 60 fps during the glass and content reveal, then pauses when the shell settles. The shell morphs inside a stable host window and native glass is revealed as the panel expands. The closed shell has PNG-shaped rounded shoulders and a solid black center aligned with the physical camera cutout. The transparent host window adds no rectangular tint or outline.

### Codex inside the island

Bookland reads local Codex session activity for the hover panel. An optional in-island composer, reached through the prompt builder, can connect to a selected **desktop Codex thread** through the installed Codex app-server. It starts read-only. “Allow edits” enables workspace writes for that Bookland-connected turn; command, file, and additional-permission approvals, plus Codex questions, open a decision panel in the island as soon as they arrive. Only your explicit choice grants a requested permission, for that turn. Bookland saves the thread's prior approval and sandbox settings before changing them, then restores them when the turn ends or when recovery runs after an interrupted launch. A separate private Codex chat can be started from Bookland. The Codex app must be installed and signed in for these integrations. Bookland cannot yet receive or answer approval requests raised by turns already running in the Codex desktop app.

Bookland also accepts an optional local JSON feed from other tools. See [AGENT_FEED.md](AGENT_FEED.md) for the format. It shows brief activity labels and selected visible messages; it does not upload the session log to Bookland servers.

### Media and privacy

System media reads use Apple's private MediaRemote framework, and system playback controls use a bundled MediaRemote Adapter helper launched on demand. That path can change with macOS releases. Apple Music and Spotify actions use local automation. Lyrics lookup sends the playing song title and artist to LRCLIB only when synced lyrics are enabled. A video URL is sent to yt-dlp only after the user clicks Download. Codex uses the installed local Codex app and its configured service. Bookland has no separate cloud account or telemetry service.

Playing Next reads up to three tracks following the current song in an Apple Music playlist, only when opened and when shuffle is off. Apple Music does not expose manually queued songs through its scripting dictionary, so this view identifies itself as a playlist preview. Queue previews for other players are not available yet.

Camera, microphone/speech, Calendar, and global shortcuts are opt-in. AirPods status uses a periodic local Bluetooth report. Music settings has an opt-in Bluetooth button for immediate AirPods connection alerts; when access is unavailable, Bookland checks every 30 seconds. Clipboard history is kept in memory by default. If local clipboard persistence is enabled, text and Finder-file references are saved under `~/Library/Application Support/Bookland/`; image payloads stay in memory. Items that macOS or supported password managers mark as confidential or temporary are skipped. Untagged sensitive text cannot always be recognized, so the clipboard privacy controls should be used with care.

## Build and run

**Requirements:** macOS 26 or later, Xcode Command Line Tools, CMake, a current Rust toolchain, and an internet connection for the first build and the optional yt-dlp binary download. The project uses Rust, egui/eframe, wgpu/Metal, and small AppKit/Objective-C bridges.

```sh
git clone https://github.com/KyrocEU/bookland.git
cd bookland
cargo test
sh package.sh
open Bookland.app
```

`package.sh` downloads the official yt-dlp macOS executable only if it is missing, verifies its SHA-256 hash, builds the Release app and bundled MediaRemote Adapter, and applies a local ad-hoc signature. This source build is **not notarized**. Rebuilding changes that ad-hoc signature, so macOS may ask you to grant Bookland Calendar access again after a rebuild. If macOS blocks it on first launch, inspect the standard macOS Open Anyway flow for locally built apps. The app is a menu-bar-style utility (`LSUIElement`) and draws its window at the screen's top edge rather than opening a normal Dock window.

To keep Bookland running after login, enable **Settings → General → Launch at login**. To tune hover behavior, use **Settings → Behavior → Close after the pointer leaves** and its delay slider. The default delay is 550 ms after leaving the island. Right-click the closed strip for Open Nook, Pocket, workspace editing, and Settings.

## Source map

- `src/main.rs`: island state, views, settings, rendering, and event loop.
- `src/glass.m`, `src/pocket_drag.m`, `src/island_gesture.rs`: native glass host, pointer tracking, and gestures.
- `src/agents.rs`, `src/codex_desktop.rs`, `src/codex_bridge.rs`: local activity and Codex connections.
- `src/now_playing.jxa`, `src/media_remote_adapter.rs`, `src/music_airplay.rs`, `src/lyrics.rs`, `src/airpods.rs`: media and accessories.
- `src/calendar.m`, `src/clipboard.m`, `src/mirror.m`, `src/prompter_speech.m`: macOS integrations.
- `src/pocket.rs`, `src/video.rs`, `src/downloads.rs`: file and download tools.
- `Info.plist`, `build.rs`, `package.sh`: app metadata, native compilation, and packaging.

See [THIRD_PARTY.md](THIRD_PARTY.md), [vendor/README.md](vendor/README.md), and [assets/README.md](assets/README.md) for dependency and asset provenance. The OpenAI mark in `assets/` identifies Codex activity and is not covered by the code's MIT license.

The editable Bookland logo, app icon, banner, and GitHub social preview are in [assets/brand](assets/brand/README.md). The icon is included in `Bookland.app` when packaged.

## Current limits

This is a working preview with gaps. AirPlay route changes, real AirPods battery and connection cues, physical trackpad haptics, live microphone-following, the new volume card's on-screen appearance, and some permission-dependent flows have not been verified end to end on every Mac. Pocket can create ZIPs and convert common image, audio, and document types, but the original's installable pipelines are not implemented. DOCX and HTML conversion uses macOS textutil; complex layouts and linked resources may vary. WebP output uses lossless encoding and does not preserve source metadata. FLAC output uses macOS afconvert with a small frame size so short clips work. Brightness HUD replacement is not implemented. The hover island and major workspaces have been exercised in the signed app, but macOS 26 compatibility and use of private media APIs can vary by system version. Open issues or contribute test results with the exact macOS version and hardware model.

## License

The original Bookland source is MIT licensed; see [LICENSE](LICENSE). Third-party code, the yt-dlp executable, and the OpenAI mark retain their own terms as described in the notices above.
