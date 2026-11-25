# Bookland CreatorOps render worker

This is the source for the Bookland short-form video render worker. It uses Remotion and three recorded Mac captures: the real island opening, a YouTube video seeking back 15 seconds from 00:45 to 00:30, and the island collapsing. `BooklandMusicReal` renders a 22-second, 1080 × 1920 MP4. The logo and CTA can change through render props, but the recorded product footage is fixed. A separate `CreatorVertical` composition is a generic template and does not show other Bookland features.

The worker accepts authenticated render jobs, returns their status, and serves completed MP4s over HTTPS with byte-range support. A repeated `jobId` with the same input returns the same render; a changed input with the same ID is rejected. It queues one render at a time, limits input and stored output size, and does not silently delete finished media.

## Run locally

Use Node.js 20.19 or newer. From this directory:

```sh
npm ci
cp .env.example .env
# Set a random WORKER_TOKEN of at least 32 characters in .env
npm run lint
npm test
npm run build
npm run worker
```

`.env` is ignored by Git. Never put a token in a workflow export or issue. Submit jobs with `Authorization: Bearer <WORKER_TOKEN>`. The render API is `POST /renders`, `GET /renders/:id`, and `GET /media/:id.mp4`. The worker accepts only the `CreatorVertical` and `BooklandMusicReal` composition IDs; the latter requires `inputProps.feature: "music"`. See `worker/server.test.mjs` for contract examples.

The Dockerfile copies `public/` into the image, so the recorded Bookland footage is available in production. Set `PUBLIC_BASE_URL` to the worker's HTTPS origin and mount a persistent directory at `DATA_DIR`. A public media URL is needed for platform ingestion. Keep the render API token private.

## Narration and publishing

Fish Audio narration is optional and off by default. If you enable it, provide your own `FISH_API_KEY` privately and use a voice for which you have commercial rights. No Fish key or generated voice recording is in this repository. This worker renders videos; it does not auto-publish them.

The private n8n draft points to the hosted worker and this 22-second composition, with a human approval gate. A [sanitized importable workflow template](n8n/README.md) is included here. Instagram and TikTok API nodes remain disabled until their official upload integrations are verified. This repository contains no private n8n credentials or claim that unattended publishing works. See [publishing/README.md](publishing/README.md) for the remaining platform checks.

The [next Music Reel production brief](music-rewind-campaign.md) uses the verified 15-second rewind as its single story, with a scored opening, voiceover, and mobile edit plan. The native EffectCraft edits used for final launch films are separate from this render worker. The footage here is included so maintainers can reproduce this specific Music demonstration. Replace it after an interface change and verify the new capture before presenting it as current product behavior.

The Bookland MIT license applies to the original worker source and Bookland logo. The recorded product captures include third-party player UI and album artwork, which are shown to demonstrate interoperability and are not relicensed by Bookland. Remotion, React, and other dependencies retain their own licenses.
