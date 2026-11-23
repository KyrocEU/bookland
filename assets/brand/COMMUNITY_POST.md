# Bookland community feedback draft

I’m building **Bookland**, an open-source macOS island that sits around the camera notch. Hover over it and a Liquid Glass panel opens for music, useful Mac tools, and live coding-agent activity. The app is written mostly in Rust, with AppKit for native glass and a Core Animation layer for the closed activity ring. The interface is in English and supports right-to-left music titles.

The current build has a working hover island; system media playback, seeking, and skip controls; artwork and optional synced lyrics; CPU, RAM, GPU, battery, and disk widgets; a floating file Pocket; clipboard history; downloads; Calendar; notes; a prompt builder and prompter; and a Codex activity view. For Codex turns started through Bookland, approvals and questions open directly in the island and wait for my decision. Bookland uses an original logo and is independent of NotchNook and OpenAI.

**Before I release it, what would make this genuinely useful on your Mac?** I’m especially interested in which features you would use every day, what should stay out of a small island, and what needs to feel faster or simpler.

Some parts are built but still need a proper demo or wider testing: Apple Music and Spotify playback, AirPlay switching, shuffle and synced lyric timing; Calendar permissions and event creation; the camera Mirror, connected AirPods batteries, voice-following Prompter, and Apple Notes export; plus the new native volume card, the full video-download flow, and real file-drag gestures.

Other features are still on the roadmap:

- Music: a true Playing Next list and audio spectrum.
- Agents: approvals from Codex turns already running in the desktop app, richer Claude Code controls, and more complete agent activity cards.
- Communication: Messages and WhatsApp conversations, replies, attachments, and voice notes.
- Mac controls: a brightness HUD, lock-screen activities, AirPods activity in the closed island, and synthetic notches on external displays.
- Workflows: video previews, multiple concurrent transfers, and Pipelines.
- Polish: matching the reference hover motion and glass appearance across different wallpapers, displays, and macOS configurations while keeping CPU and GPU use low.

I haven’t published this draft or the latest local changes yet. If you could pick **three priorities** from that list, which would they be? Any missing feature or design criticism is welcome.
