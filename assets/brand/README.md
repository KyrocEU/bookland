# Bookland identity

Bookland's mark is an open book with a bright spine and a small diamond at its top. The book forms the app icon and the first item of the island menu. These assets are original to Bookland; the OpenAI mark used for Codex activity is separate.

| Role | Color |
| --- | --- |
| Ink | `#050A13` |
| Glass blue | `#5B95E9` |
| Ice | `#D7F3FF` |
| Text | `#F4F8FF` |

`bookland-icon.svg` is the editable base symbol and `bookland-icon-premium.png` is the refined app icon used by the signed build and island interface. `bookland-mark.svg` is the transparent flat mark for other small uses. `bookland-banner.svg` and `bookland-launch-hero.png` are the GitHub and campaign banners. `Bookland.icns`, the remaining PNG versions, and `social-preview.png` are generated deliverables.

The premium icon and launch hero were generated with the built-in image generation tool from the original Bookland vector icon. The icon prompt preserved the open-book shape, diamond, page colors, navy tile, and transparent corners while adding soft depth and highlights. The hero prompt carried that icon into a dark macOS launch image with the exact wordmark, tagline, and feature labels. The original vectors remain in this folder for reproducible small-scale interface graphics.

See [BRAND_GUIDE.md](BRAND_GUIDE.md) for logo usage and launch copy, [COMMUNITY_POST.md](COMMUNITY_POST.md) for a pre-release feedback message, and [PROMPTS.md](PROMPTS.md) for the exact image generation prompts.

To regenerate the app icon bundle and vector-derived raster files on macOS, install `rsvg-convert` and run `sh assets/brand/build_icons.sh` from the repository root. The script uses the checked-in premium PNG as its icon source and macOS `sips` and `iconutil` for the icon set. The ordinary Bookland build uses the checked-in `.icns` and does not require `rsvg-convert`.
