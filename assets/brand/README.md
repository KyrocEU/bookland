# Bookland identity

Bookland's mark is an open book with a bright spine and a small diamond at its top. The book forms the app icon and the first item of the island menu. These assets are original to Bookland; the OpenAI mark used for Codex activity is separate.

| Role | Color |
| --- | --- |
| Ink | `#050A13` |
| Glass blue | `#5B95E9` |
| Ice | `#D7F3FF` |
| Text | `#F4F8FF` |

`bookland-icon.svg` is the editable base symbol and `bookland-icon-premium.png` is the refined app icon used by the signed build and island interface. `bookland-mark.svg` is the transparent flat mark for other small uses. `bookland-social-avatar.png` is a larger circular profile variant with transparent corners. `bookland-reddit-banner.jpg` is a 1000×300 Reddit header under 500 KB. `bookland-banner.svg` composes the README hero from the premium icon and `bookland-glass-backdrop.png`; `bookland-social-preview.svg` crops that composition to GitHub's 2:1 preview. The corresponding PNGs are `bookland-launch-hero.png` (1672×941) and `social-preview.png` (1280×640). `Bookland.icns` and the remaining PNG versions are generated deliverables.

The premium icon was generated with the built-in image generation tool from the original Bookland vector icon. The icon prompt preserved the open-book shape, diamond, page colors, navy tile, and transparent corners while adding soft depth and highlights. The new glass backdrop was generated with that tool using the premium icon and previous hero as palette and atmosphere references. The exact logo, typography, feature names, and illustrative island layout are assembled in SVG so they stay consistent and editable. This is marketing artwork, not a screenshot of the live app.

See [BRAND_GUIDE.md](BRAND_GUIDE.md) for logo usage and launch copy, [COMMUNITY_POST.md](COMMUNITY_POST.md) for a pre-release feedback message, and [PROMPTS.md](PROMPTS.md) for the exact image generation prompts.

To regenerate the app icon bundle and vector-derived raster files on macOS, install `rsvg-convert` and run `sh assets/brand/build_icons.sh` from the repository root. The script uses the checked-in premium PNG as its icon source and macOS `sips` and `iconutil` for the icon set. The ordinary Bookland build uses the checked-in `.icns` and does not require `rsvg-convert`.

To regenerate the GitHub images from their checked-in SVG compositions, run `rsvg-convert -o assets/brand/bookland-launch-hero.png assets/brand/bookland-banner.svg`, then `rsvg-convert -o assets/brand/social-preview.png assets/brand/bookland-social-preview.svg` from the repository root. Both SVGs resolve the checked-in PNGs relative to this folder. Upload `social-preview.png` in GitHub repository Settings → General → Social preview; committing that file alone does not change GitHub's cached social image.
