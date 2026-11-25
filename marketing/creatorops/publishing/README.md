# Social publishing gates

The n8n workflow renders and validates a video, then branches to Instagram and TikTok nodes. Those publishing nodes stay disabled until the platform accounts, app permissions, and callbacks below are verified. A `Video Ready` result is a draft asset, not a published post.

## Instagram Reel

Use an Instagram Professional account and a Meta app with the publishing permission for the chosen login method. [Meta's content-publishing guide](https://developers.facebook.com/documentation/instagram-platform/content-publishing) lists the required token, host, and scopes for Instagram Login, Facebook Login, and Unified Login.

1. Create a media container with `POST /<IG_ID>/media`, `media_type=REELS`, `video_url=<public MP4 URL>`, and the approved caption.
2. Poll `GET /<IG_CONTAINER_ID>?fields=status_code`. Only publish when `status_code=FINISHED`; stop on `ERROR` or expiry.
3. Publish with `POST /<IG_ID>/media_publish`, passing `creation_id=<IG_CONTAINER_ID>`.
4. Save the returned media ID and read its permalink. Check `media_product_type=REELS` on the published media when using Facebook Login; `media_type=VIDEO` alone does not prove it is a Reel.

The worker's public MP4 URL must remain available while Meta fetches it. The n8n draft currently lacks the container-status poll and must not be enabled unchanged.

## TikTok Direct Post

[TikTok's Direct Post API](https://developers.tiktok.com/docs/en/content-posting-api-reference-direct-post) requires `video.publish`, a developer app, and an audited client for public visibility. An unaudited client is restricted to private posts. For `PULL_FROM_URL`, TikTok also requires ownership verification of the video URL prefix or domain. A temporary Cloudflare URL is unsuitable for that verification.

1. Call `POST /v2/post/publish/creator_info/query/` immediately before presenting or approving the post. Use its current privacy options and maximum video duration.
2. After the user approves the exact caption and privacy choice, call `POST /v2/post/publish/video/init/` with `source=PULL_FROM_URL`, the verified worker URL, and the chosen privacy setting. Store `publish_id`.
3. Poll [Get Post Status](https://developers.tiktok.com/docs/en/content-posting-api-reference-get-video-status) until `PUBLISH_COMPLETE` or `FAILED`. Do not equate an accepted init response with a live post.

The n8n draft currently lacks creator-info and post-status steps and has no TikTok app token. Leave its TikTok nodes disabled until those pieces pass a private test and the app audit supports public posting.

## Before enabling either branch

- Add account credentials in n8n's encrypted credential store, never to workflow JSON or this repository.
- Confirm the platform account receives the correct 9:16 video and audio in a private test.
- Record draft ID, render ID, platform container/publish ID, final URL, status, and any error in the CreatorOps data table so retries cannot duplicate a post.
- Keep the current human approval gate while these platform integrations are being verified. A later unattended mode needs explicit posting rules and duplicate protection.
