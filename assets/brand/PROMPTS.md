# Image generation prompts

The built-in image generation tool produced the premium icon and the backdrop plate. Both are project-bound and saved in this folder. The vector mark, wordmarks, GitHub hero, and social-preview composition were authored directly as SVG.

## `bookland-icon-premium.png`

Input: `bookland-icon.png`, an original raster export of `bookland-icon.svg`. Transparent background requested.

> Use case: logo-brand. Asset type: production macOS app icon refinement. Edit the supplied Bookland app icon, preserving its recognizable open-book silhouette, central spine, small diamond above the spine, navy rounded-square tile, ice-white left page and soft blue right page. Make it feel more premium and native to modern macOS: subtle depth beneath the book, restrained soft contact shadow, delicate specular highlights along the upper page edges, polished layered glass-like finish, very clean geometry, high legibility at small Dock sizes. Keep the icon centered with generous navy breathing room. Keep the rounded-square silhouette and transparent corners. No letters, no extra symbols, no watermark, no outer rectangular background or white border. The page shape and palette must remain faithful to the reference; refine lighting and finish only.

## Previous `bookland-launch-hero.png`

Input: `bookland-icon-premium.png`. Opaque background requested.

> Use case: ads-marketing. Asset type: Bookland product launch hero and social cover, wide landscape 16:9. Input image is the approved Bookland icon reference: preserve the open-book symbol, tiny diamond, ice-white and sky-blue page split, and deep navy identity. Create a sophisticated minimal macOS product campaign image. Left third: the recognizable Bookland icon appears as a beautifully lit physical glass-and-enamel app tile with a subtle soft shadow. Right two-thirds: precise, spacious premium typography with the exact wordmark 'Bookland' and exact tagline 'A living island for your Mac.' Underneath, a restrained hairline and three small descriptors: 'Liquid Glass   ·   Media   ·   Codex'. Background: deep near-black navy, softly layered translucent glass arcs and faint blue light falloff, premium Apple-like studio lighting, generous negative space. Avoid fake interface screenshots, Mac hardware, other logos, extra text, watermarks, neon effects, heavy glow, or ornate decoration. Ensure the wordmark and tagline are spelled exactly as given.

The original generated hero has been replaced by the editable SVG composition in `bookland-banner.svg`. The prompt above is retained as a historical record.

## `bookland-glass-backdrop.png`

Inputs: the previous `bookland-launch-hero.png` as a style reference and `bookland-icon-premium.png` as a palette/material reference. Opaque background requested. The tool returned a 1672×941 plate. Exact branding and the illustrative island were added later in SVG.

> Use case: ads-marketing. Asset type: wide background plate for the Bookland GitHub README hero and repository social preview. Create a premium, original macOS-inspired product background in Bookland's exact palette: ink #050A13, glass blue #5B95E9, ice #D7F3FF. A dark near-black canvas with a subtle luminous blue aurora and deep translucent layered glass ribbons flowing behind a clean central composition. Polished, restrained Apple keynote art direction: physical depth, soft internal reflection, delicate edge highlights, controlled shadows, fine subtle grain, elegant spatial lighting. Keep the central lower half readable and uncluttered for a later composited UI and logo; atmospheric curved light toward left and right edges. WIDE LANDSCAPE 16:9, high resolution. No text, no letters, no logo, no app icon, no phone or laptop, no UI, no people, no watermark. The attached existing Bookland hero is a style reference only; the attached premium icon is a palette/material reference only, and should not appear in the output.
