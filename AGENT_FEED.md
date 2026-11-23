# Live agent activity

Bookland reads recent Codex and Claude Code session events and optional Craft Anything project updates from
this Mac every five seconds. It displays task titles, visible agent messages,
and completion status. Separately, the Codex chat in Agents runs the installed Codex CLI when you send a message.

In the hover island's Agents Progress view, click a selected Codex task's message bubble to open its compact chat. Desktop Codex tasks use the installed Codex app-server and resume the selected desktop thread. They start in read-only mode; the Allow edits switch enables workspace writes for a turn. Command or file approvals and Codex questions appear in the island **only for turns started through Bookland**. A turn already running in the Codex desktop app keeps its approvals there; Bookland shows its activity and offers Open Codex to review it. The separate Bookland app-server connection cannot subscribe to that desktop process's pending requests. Bookland-private Codex tasks use Bookland's private Codex home. While the selected desktop task is Working, its island Send button is disabled. Right-click the task title to choose a recent task. Activity returns to Progress. Bookland records the thread's original approval and sandbox settings before any edit-enabled desktop turn, then restores them on completion or on the next launch after a forced exit. Recovery of an active thread waits until a later launch or another send attempt in that thread.

Other local tools can publish an activity feed by atomically replacing:

`~/Library/Application Support/Bookland/agents.json`

The file is a JSON array, up to eight entries. For example:

```json
[
  {
    "source": "My Agent",
    "title": "Drafting a report",
    "status": "Working",
    "summary": "Checking the final figures",
    "events": ["Gathering evidence", "Writing summary"]
  }
]
```

Each entry needs a `title`. `source`, `status`, `summary`, and `events` are optional. `summary` appears in the compact Agents glass bubble.
Bookland shows at most five recent event lines per entry. A task that changes from `Working` to `Finished` while Bookland is running also raises a closed-island completion cue. Clicking the cue opens the matching Agents entry. The first scan establishes a baseline, so old finished entries do not alert on startup. An agent that writes
this file controls what the island displays; it should publish only activity
the person intends to see on screen.
