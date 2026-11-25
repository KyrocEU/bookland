# Bookland n8n workflow template

[`bookland-video-pipeline.template.json`](bookland-video-pipeline.template.json) is a sanitized export of the 29-node Bookland Music video pipeline. It includes a brief form, AI draft, structured output, a Data Table record, a three-day human approval form, a render request, polling, output validation, and disabled Instagram and TikTok branches. It contains no worker token, Fish key, OpenAI key, account credential ID, n8n instance ID, or live worker URL. The template imports inactive.

To use it in your own n8n workspace:

1. Import the JSON file. Create a Data Table named `creatorops_video_drafts` with text columns `run_id`, `brand_name`, `topic`, `status`, `draft_json`, `approval_url`, and `submitted_at`. Select that table in **Save Draft**, **Mark Approved**, and **Mark Rejected**.
2. Select an AI credential for **AI Model**. The model in the template is a draft choice; choose one available in your workspace.
3. Create one encrypted **Simplified Custom Auth** credential for the render worker. Use the auth template `{"headers":{"Authorization":"Bearer {{token}}"}}` and enter the worker's `WORKER_TOKEN` in the credential's `token` field. Select the credential on **Submit Render Job** and **Get Render Status**.
4. Set `worker_base_url` in **Render Config (EDIT ME)** to your deployed HTTPS worker. The placeholder `<SET_WORKER_URL>` deliberately fails the **Worker Configured?** gate. Keep `composition_id` as `BooklandMusicReal`; the render request sets `inputProps.feature` to `music`.
5. Run one private execution. Check the generated package against the real footage, approve its draft, and verify **Video Ready** returns a 22-second, 1080 × 1920 MP4. The live n8n draft has not yet passed this full hosted path.

The Instagram and TikTok HTTP nodes are disabled deliberately. Their container and publish status checks, official API permissions, and duplicate protection are incomplete. Follow [social publishing gates](../publishing/README.md) before enabling them. The approved launch films have also used EffectCraft; this workflow is an additional reproducible Music format, not a replacement for those edits.
