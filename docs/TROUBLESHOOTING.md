# Troubleshooting the Bookland preview

These steps apply to the current [0.1.4 preview](https://github.com/KyrocEU/bookland/releases/tag/v0.1.4)
on an Apple Silicon Mac running macOS 26 or later.

## Bookland does not appear in the Dock

Bookland is a menu-bar-style utility. It opens as a slim island at the top of
the selected display, beside the camera cutout. Move the pointer near the top
center edge. You can change the target display in **Settings → Appearance**.

## macOS blocks the app on first launch

The preview is locally signed but not Apple-notarized. After trying to open it
from Applications, use **System Settings → Privacy & Security → Open Anyway**.
The [release page](https://github.com/KyrocEU/bookland/releases/tag/v0.1.4)
provides the ZIP and SHA-256 checksum. Do not approve an app downloaded from
an unrelated source.

## The island stays expanded

Select the current workspace pill to collapse it immediately. Check
**Settings → Behavior → Close after the pointer leaves** and its delay slider;
the default is 550 ms. Right-clicking the closed strip and choosing **Open
Nook** intentionally keeps that workspace open.

## Music controls do not act on a player

Start a song or video in the player first. If nothing is queued, the Play
button can open the selected player but cannot select music for you. Bookland
uses macOS media-session support, which can differ between players, browsers,
and macOS releases. If the problem repeats, report the player, browser if any,
the action you pressed, and what changed on screen. See [Media and privacy](../README.md#media-and-privacy)
for how the integration works.

## Google Calendar is empty

Bookland reads calendars through macOS Calendar. Confirm that the Google account
is present in **System Settings → Internet Accounts**, that **Calendars** is on,
and that its events appear in the Calendar app. Then open Bookland's Calendar
workspace, choose **Connect Calendar**, grant the macOS permission, and select
the account under **Calendar Source**. [The full setup](../README.md#google-calendar-on-your-mac)
explains the connection.

## An older copy seems to open

Bookland allows one normal instance per Mac user. Quit the running copy before
opening a build from another folder, then use the copy in Applications. If the
issue persists, include the app version shown in Bookland and the path you
launched in a [bug report](https://github.com/KyrocEU/bookland/issues/new?template=bug-report.yml).
