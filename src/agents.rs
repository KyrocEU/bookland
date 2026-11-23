use serde_json::Value;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

#[derive(Clone, Default)]
pub struct AgentSnapshot {
    pub source: String,
    pub title: String,
    pub status: String,
    pub events: Vec<String>,
    pub last_message: Option<String>,
    pub path: PathBuf,
    pub updated: Option<SystemTime>,
    pub usage: Option<AgentUsage>,
}

#[derive(Clone, Default)]
pub struct AgentUsage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub limit_used_percent: Option<f32>,
    pub limit_window_minutes: Option<u64>,
}

pub fn newly_finished<'a>(previous: &[AgentSnapshot], current: &'a [AgentSnapshot])
    -> Option<&'a AgentSnapshot> {
    current.iter().find(|agent| agent.status == "Finished"
        && previous.iter().any(|old| old.path == agent.path
            && old.source == agent.source && old.title == agent.title
            && old.status == "Working"))
}

fn codex_usage(payload: &Value, previous: Option<AgentUsage>) -> Option<AgentUsage> {
    let usage = &payload["info"]["total_token_usage"];
    let mut result = previous.unwrap_or_default();
    if usage.is_object() {
        result.input_tokens = usage["input_tokens"].as_u64().unwrap_or(0);
        result.cached_input_tokens = usage["cached_input_tokens"].as_u64().unwrap_or(0);
        result.output_tokens = usage["output_tokens"].as_u64().unwrap_or(0);
    }
    let primary = &payload["rate_limits"]["primary"];
    if let Some(percent) = primary["used_percent"].as_f64() {
        result.limit_used_percent = Some(percent.clamp(0.0, 100.0) as f32);
        result.limit_window_minutes = primary["window_minutes"].as_u64();
    }
    (usage.is_object() || result.limit_used_percent.is_some()).then_some(result)
}

fn shortened(text: &str, max: usize) -> String {
    let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out: String = cleaned.chars().take(max).collect();
    if cleaned.chars().count() > max {
        out.push('…');
    }
    out
}

fn content_text(item: &Value) -> Option<String> {
    item.get("content")?
        .as_array()?
        .iter()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .find(|text| !text.trim().is_empty())
        .map(|text| shortened(text, 125))
}

fn claude_text(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return (!text.trim().is_empty()).then(|| shortened(text, 125));
    }
    content.as_array()?.iter()
        .filter(|part| part["type"] == "text")
        .filter_map(|part| part["text"].as_str())
        .find(|text| !text.trim().is_empty())
        .map(|text| shortened(text, 125))
}

fn recent_lines(path: &Path, max_bytes: u64) -> Option<Vec<String>> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(max_bytes))).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = text.lines();
    if len > max_bytes {
        lines.next(); // skip the partial first record
    }
    Some(lines.map(str::to_owned).collect())
}

#[derive(Clone)]
struct CodexContext {
    offset: u64,
    title: String,
    status: String,
    last_message: Option<String>,
}

fn context_from_line(line: &[u8], title: &mut Option<String>, status: &mut Option<String>,
                     last_message: &mut Option<Option<String>>) {
    // Most records contain tool arguments or output. Only decode candidate lifecycle records.
    let has = |needle: &[u8]| line.windows(needle.len()).any(|window| window == needle);
    if !has(b"task_started") && !has(b"task_complete") && !has(b"turn_aborted")
        && !has(b"UserMessage") && !has(b"AgentMessage") { return; }
    let Ok(record) = serde_json::from_slice::<Value>(line) else { return };
    if record["type"] != "event_msg" { return; }
    let payload = &record["payload"];
    match payload["type"].as_str().unwrap_or("") {
        "task_started" => {
            *status = Some("Working".into());
            *last_message = Some(None);
        }
        "task_complete" => *status = Some("Finished".into()),
        "turn_aborted" => *status = Some("Stopped".into()),
        "item_completed" if payload["item"]["type"] == "UserMessage" => {
            if let Some(text) = content_text(&payload["item"]) {
                *title = Some(text);
                *last_message = Some(None);
            }
        }
        "item_completed" if payload["item"]["type"] == "AgentMessage" => {
            if let Some(text) = content_text(&payload["item"]) {
                if !text.starts_with("::") { *last_message = Some(Some(text)); }
            }
        }
        _ => {}
    }
}

fn first_codex_title(path: &Path, len: u64) -> Option<String> {
    const MAX_PREFIX: u64 = 8 * 1024 * 1024;
    let mut file = File::open(path).ok()?;
    let mut bytes = vec![0; len.min(MAX_PREFIX) as usize];
    file.read_exact(&mut bytes).ok()?;
    for line in bytes.split(|byte| *byte == b'\n') {
        let mut title = None;
        context_from_line(line, &mut title, &mut None, &mut None);
        if title.is_some() { return title; }
    }
    None
}

fn context_from_history(path: &Path, len: u64) -> Option<CodexContext> {
    const BLOCK: u64 = 256 * 1024;
    const MAX_MESSAGE_SCAN: u64 = 64 * 1024 * 1024;
    let mut file = File::open(path).ok()?;
    let mut cursor = len;
    let mut carry = Vec::new();
    let mut status = None;
    let mut last_message = None;
    while cursor > 0 && (status.is_none() ||
        (last_message.is_none() && len - cursor < MAX_MESSAGE_SCAN)) {
        let start = cursor.saturating_sub(BLOCK);
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut bytes = vec![0; (cursor - start) as usize];
        file.read_exact(&mut bytes).ok()?;
        bytes.extend_from_slice(&carry);
        let mut lines = bytes.split(|byte| *byte == b'\n');
        carry = if start > 0 { lines.next().unwrap_or_default().to_vec() }
            else { Vec::new() };
        for line in lines.rev() {
            if status.is_some() && last_message.is_some() { break; }
            let mut found_status = None;
            let mut found_message = None;
            context_from_line(line, &mut None, &mut found_status, &mut found_message);
            if status.is_none() { status = found_status; }
            if last_message.is_none() && len - start <= MAX_MESSAGE_SCAN {
                last_message = found_message;
            }
        }
        cursor = start;
    }
    Some(CodexContext {
        offset: len,
        title: first_codex_title(path, len).unwrap_or_else(|| "Codex task".into()),
        status: status.unwrap_or_else(|| "Idle".into()),
        last_message: last_message.flatten(),
    })
}

fn codex_context(path: &Path) -> Option<CodexContext> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CodexContext>>> = OnceLock::new();
    let len = fs::metadata(path).ok()?.len();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let previous = cache.lock().ok()?.get(path).cloned();
    let context = if let Some(mut previous) = previous.filter(|cached| cached.offset <= len) {
        let delta = len - previous.offset;
        if delta > 4 * 1024 * 1024 {
            context_from_history(path, len)?
        } else if delta == 0 {
            previous
        } else {
            let mut file = File::open(path).ok()?;
            file.seek(SeekFrom::Start(previous.offset)).ok()?;
            let mut bytes = Vec::with_capacity(delta as usize);
            file.read_to_end(&mut bytes).ok()?;
            if let Some(end) = bytes.iter().rposition(|byte| *byte == b'\n') {
                for line in bytes[..end].split(|byte| *byte == b'\n') {
                    let mut title = None;
                    let mut status = None;
                    let mut last_message = None;
                    context_from_line(line, &mut title, &mut status, &mut last_message);
                    if previous.title == "Codex task" {
                        if let Some(title) = title { previous.title = title; }
                    }
                    if let Some(status) = status { previous.status = status; }
                    if let Some(last_message) = last_message {
                        previous.last_message = last_message;
                    }
                }
                previous.offset += end as u64 + 1;
            }
            previous
        }
    } else {
        context_from_history(path, len)?
    };
    cache.lock().ok()?.insert(path.to_owned(), context.clone());
    Some(context)
}

fn push_recent(events: &mut Vec<String>, line: String) {
    if events.len() == 5 { events.remove(0); }
    events.push(line);
}

fn command_event(item: &Value) -> String {
    let command = item["parsed_cmd"].as_array()
        .and_then(|parts| parts.iter().find_map(|part| part["cmd"].as_str()))
        .or_else(|| item["command"].as_array()
            .and_then(|parts| parts.last()).and_then(Value::as_str))
        .and_then(|text| text.lines().find(|line| !line.trim().is_empty()));
    let Some(command) = command else { return "Ran a command".into() };
    let summary = format!("Run {}", shortened(command, 64));
    if item["exit_code"].as_i64().is_some_and(|code| code != 0) {
        format!("{summary} · failed")
    } else { summary }
}

fn tool_event(item: &Value) -> String {
    let title = item["arguments"]["title"].as_str().filter(|title| !title.trim().is_empty());
    if let Some(title) = title {
        let title = shortened(title, 72);
        if item["status"] == "failed" { format!("{title} · failed") }
        else { title }
    } else if item["status"] == "failed" {
        "Connected tool failed".into()
    } else { "Used a connected tool".into() }
}

fn parse_codex(path: &Path) -> Option<AgentSnapshot> {
    let context = codex_context(path)?;
    let mut snap = AgentSnapshot {
        source: "Codex".into(),
        title: context.title,
        status: context.status,
        last_message: context.last_message,
        path: path.to_owned(),
        updated: fs::metadata(path).ok().and_then(|meta| meta.modified().ok()),
        ..Default::default()
    };
    let mut last_events = Vec::new();
    for line in recent_lines(path, 750_000)? {
        let Ok(record) = serde_json::from_str::<Value>(&line) else { continue };
        let payload = &record["payload"];
        if record["type"] != "event_msg" {
            continue;
        }
        match payload["type"].as_str().unwrap_or("") {
            "token_count" => snap.usage = codex_usage(payload, snap.usage.take()),
            "task_started" => {
                last_events.clear();
                snap.last_message = None;
            }
            "item_completed" => {
                let item = &payload["item"];
                match item["type"].as_str().unwrap_or("") {
                    "UserMessage" => {
                        if content_text(item).is_some() {
                            last_events.clear();
                            snap.last_message = None;
                        }
                    }
                    "AgentMessage" => {
                        if let Some(text) = content_text(item) {
                            if !text.starts_with("::") {
                                snap.last_message = Some(text.clone());
                                push_recent(&mut last_events, text);
                            }
                        }
                    }
                    "FileChange" => push_recent(&mut last_events, "Updated project files".into()),
                    "CommandExecution" => push_recent(&mut last_events, command_event(item)),
                    "McpToolCall" => push_recent(&mut last_events, tool_event(item)),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    snap.events = last_events;
    Some(snap)
}

fn parse_claude(path: &Path) -> Option<AgentSnapshot> {
    let updated = fs::metadata(path).ok().and_then(|meta| meta.modified().ok());
    let mut snapshot = AgentSnapshot {
        source: "Claude Code".into(),
        title: "Claude Code task".into(),
        status: "Idle".into(),
        path: path.to_owned(),
        updated,
        ..Default::default()
    };
    let mut has_conversation = false;
    for line in recent_lines(path, 750_000)? {
        let Ok(record) = serde_json::from_str::<Value>(&line) else { continue };
        if record["isSidechain"] == true { continue; }
        match record["type"].as_str().unwrap_or("") {
            "user" => {
                if let Some(text) = claude_text(&record["message"]["content"]) {
                    snapshot.title = text;
                    snapshot.events.clear();
                    snapshot.last_message = None;
                    snapshot.status = "Working".into();
                    has_conversation = true;
                }
            }
            "assistant" => {
                if let Some(text) = claude_text(&record["message"]["content"]) {
                    snapshot.last_message = Some(text.clone());
                    snapshot.events.push(text);
                    if snapshot.events.len() > 5 { snapshot.events.remove(0); }
                    has_conversation = true;
                }
                if record["message"]["stop_reason"] == "end_turn" {
                    snapshot.status = "Finished".into();
                }
            }
            "system" if record["subtype"] == "turn_duration" => {
                snapshot.status = "Finished".into();
            }
            _ => {}
        }
    }
    if !has_conversation { return None; }
    if snapshot.status == "Working" && updated.is_some_and(|time|
        time.elapsed().is_ok_and(|elapsed| elapsed.as_secs() > 120)) {
        snapshot.status = "Idle".into();
    }
    Some(snapshot)
}

fn collect_codex_files(root: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if depth == 0 || found.len() > 5000 { return; }
    let Ok(entries) = fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_codex_files(&path, depth - 1, found);
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            found.push(path);
        }
    }
}

fn codex_thread_names(codex_home: &Path) -> HashMap<PathBuf, String> {
    let database = codex_home.join("state_5.sqlite");
    if !database.is_file() { return HashMap::new(); }
    let Ok(output) = Command::new("/usr/bin/sqlite3")
        .args(["-readonly", "-json"])
        .arg(database)
        .arg("SELECT rollout_path, name FROM threads WHERE name IS NOT NULL AND name <> ''")
        .output() else { return HashMap::new() };
    if !output.status.success() { return HashMap::new(); }
    let Ok(rows) = serde_json::from_slice::<Vec<Value>>(&output.stdout) else {
        return HashMap::new();
    };
    rows.into_iter().filter_map(|row| {
        let path = row["rollout_path"].as_str()?;
        let name = row["name"].as_str()?;
        Some((PathBuf::from(path), shortened(name, 125)))
    }).collect()
}

fn craft_projects(root: &Path) -> Vec<AgentSnapshot> {
    let Ok(entries) = fs::read_dir(root) else { return Vec::new() };
    let mut snapshots = Vec::new();
    for entry in entries.flatten() {
        let folder = entry.path();
        let path = folder.join("craft.json");
        let Ok(data) = fs::read_to_string(&path) else { continue };
        let Ok(value) = serde_json::from_str::<Value>(&data) else { continue };
        let status = value["status"].as_str().unwrap_or("unknown");
        let mut snap = AgentSnapshot {
            source: "Craft Anything".into(),
            title: value["name"].as_str().unwrap_or("Craft project").into(),
            status: match status {
                "ready" => "Finished",
                "working" | "building" | "running" => "Working",
                "failed" | "error" => "Error",
                _ => "Idle",
            }.into(),
            path: folder,
            updated: fs::metadata(&path).ok().and_then(|meta| meta.modified().ok()),
            ..Default::default()
        };
        if let Ok(feed) = fs::read_to_string(snap.path.join("feed.json")) {
            if let Ok(feed) = serde_json::from_str::<Value>(&feed) {
                let mut events = Vec::new();
                if let Some(items) = feed["items"].as_array() {
                    for item in items.iter().rev() {
                        let line = item.get("Step").and_then(|step| step["text"].as_str())
                            .or_else(|| item.get("Work").and_then(|work| work["notes"].as_array())
                                .and_then(|notes| notes.last()).and_then(Value::as_str));
                        if let Some(line) = line {
                            events.push(shortened(line, 125));
                            if events.len() >= 5 { break; }
                        }
                    }
                }
                events.reverse();
                snap.events = events;
                snap.last_message = snap.events.last().cloned();
            }
        }
        snapshots.push(snap);
    }
    snapshots
}

fn external_feed(path: &Path) -> Vec<AgentSnapshot> {
    let Ok(data) = fs::read_to_string(path) else { return Vec::new() };
    let Ok(value) = serde_json::from_str::<Value>(&data) else { return Vec::new() };
    let Some(items) = value.as_array() else { return Vec::new() };
    let updated = fs::metadata(path).ok().and_then(|meta| meta.modified().ok());
    items.iter().take(8).filter_map(|item| {
        let title = item["title"].as_str()?;
        let events = item["events"].as_array().map(|rows| rows.iter()
            .filter_map(Value::as_str).take(5).map(|line| shortened(line, 125))
            .collect()).unwrap_or_default();
        Some(AgentSnapshot {
            source: shortened(item["source"].as_str().unwrap_or("Agent"), 30),
            title: shortened(title, 125),
            status: shortened(item["status"].as_str().unwrap_or("Idle"), 30),
            events,
            last_message: item["summary"].as_str().map(|text| shortened(text, 125)),
            path: path.to_owned(),
            updated,
            usage: None,
        })
    }).collect()
}

pub fn scan_local(home: &Path) -> Vec<AgentSnapshot> {
    let mut files = Vec::new();
    let desktop_codex = home.join(".codex");
    let island_codex = home.join("Library/Application Support/Bookland/CodexHome");
    for codex_home in [&desktop_codex, &island_codex] {
        collect_codex_files(&codex_home.join("sessions"), 5, &mut files);
    }
    files.sort_by_key(|path| fs::metadata(path).and_then(|meta| meta.modified()).ok());
    let mut agents = files.into_iter().rev().take(5).filter_map(|path| parse_codex(&path))
        .collect::<Vec<_>>();
    let mut names = codex_thread_names(&desktop_codex);
    names.extend(codex_thread_names(&island_codex));
    for agent in &mut agents {
        if let Some(name) = names.get(&agent.path) { agent.title = name.clone(); }
    }
    let claude_config = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from).unwrap_or_else(|| home.join(".claude"));
    let mut claude_files = Vec::new();
    collect_codex_files(&claude_config.join("projects"), 5, &mut claude_files);
    claude_files.sort_by_key(|path| fs::metadata(path).and_then(|meta| meta.modified()).ok());
    agents.extend(claude_files.into_iter().rev()
        .filter(|path| !path.to_string_lossy().contains("/subagents/"))
        .take(5)
        .filter_map(|path| parse_claude(&path)));
    let feed = home.join("Library/Application Support/Bookland/agents.json");
    let previous_feed = home.join("Library/Application Support/Nooklet/agents.json");
    agents.extend(external_feed(if feed.exists() { &feed } else { &previous_feed }));
    agents.sort_by_key(|agent| std::cmp::Reverse(agent.updated));
    agents.truncate(8);
    agents
}

pub fn scan_craft(home: &Path) -> Vec<AgentSnapshot> {
    let mut agents = Vec::new();
    for root in [home.join("Documents/Craft Anything"), home.join("Documents/Craft-Anything")] {
        agents.extend(craft_projects(&root));
    }
    agents.sort_by_key(|agent| std::cmp::Reverse(agent.updated));
    agents.dedup_by(|a,b| a.source == b.source && a.title == b.title
        && a.path.canonicalize().ok() == b.path.canonicalize().ok());
    agents.truncate(8);
    agents
}

pub fn scan(home: &Path) -> Vec<AgentSnapshot> {
    let mut agents = scan_local(home);
    agents.extend(scan_craft(home));
    agents.sort_by_key(|agent| std::cmp::Reverse(agent.updated));
    agents.truncate(8);
    agents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_activity_includes_island_codex_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let desktop = temp.path().join(".codex/sessions/2026/11/23");
        let island = temp.path().join("Library/Application Support/Bookland/CodexHome/sessions/2026/11/23");
        fs::create_dir_all(&desktop).unwrap();
        fs::create_dir_all(&island).unwrap();
        for (folder, title) in [(&desktop, "Desktop turn"), (&island, "Island turn")] {
            fs::write(folder.join("rollout.jsonl"), format!(
                "{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"item_completed\",\"item\":{{\"type\":\"UserMessage\",\"content\":[{{\"text\":\"{title}\"}}]}}}}}}\n"
            )).unwrap();
        }
        let activity = scan_local(temp.path());
        assert!(activity.iter().any(|row| row.title == "Desktop turn"));
        assert!(activity.iter().any(|row| row.title == "Island turn"));
    }

    #[test]
    fn completion_nudge_requires_a_working_to_finished_transition() {
        let mut working = AgentSnapshot {
            source: "Codex".into(),
            status: "Working".into(),
            path: PathBuf::from("/tmp/codex-session.jsonl"),
            ..Default::default()
        };
        assert!(newly_finished(&[], &[working.clone()]).is_none());
        let mut finished = working.clone();
        finished.status = "Finished".into();
        assert!(newly_finished(&[working.clone()], &[finished.clone()]).is_some());
        assert!(newly_finished(&[finished.clone()], &[finished]).is_none());
        let mut other_task = working.clone();
        other_task.title = "Another task".into();
        other_task.status = "Finished".into();
        assert!(newly_finished(&[working.clone()], &[other_task]).is_none());
        working.source = "Claude Code".into();
        assert!(newly_finished(&[working], &[AgentSnapshot {
            source: "Codex".into(), status: "Finished".into(),
            path: PathBuf::from("/tmp/codex-session.jsonl"),
            ..Default::default()
        }]).is_none());
    }

    #[test]
    fn codex_state_tracks_latest_turn() {
        let dir = std::env::temp_dir().join(format!("nooklet-agents-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.jsonl");
        fs::write(&path, concat!(
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"item_completed\",\"item\":{\"type\":\"UserMessage\",\"content\":[{\"text\":\"Build glass\"}]}}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"item_completed\",\"item\":{\"type\":\"AgentMessage\",\"content\":[{\"text\":\"Making progress\"}]}}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"item_completed\",\"item\":{\"type\":\"UserMessage\",\"content\":[{\"text\":\"Change only hover\"}]}}}\n"
        )).unwrap();
        let result = parse_codex(&path).unwrap();
        assert_eq!(result.title, "Build glass");
        assert_eq!(result.status, "Working");
        assert!(result.events.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn codex_usage_reads_cumulative_tokens_and_limit() {
        let payload: Value = serde_json::from_str(r#"{"info":{"total_token_usage":{"input_tokens":1234,"cached_input_tokens":400,"output_tokens":56}},"rate_limits":{"primary":{"used_percent":41.5,"window_minutes":10080}}}"#).unwrap();
        let usage = codex_usage(&payload, None).unwrap();
        assert_eq!(usage.input_tokens, 1234);
        assert_eq!(usage.cached_input_tokens, 400);
        assert_eq!(usage.output_tokens, 56);
        assert_eq!(usage.limit_used_percent, Some(41.5));
        assert_eq!(usage.limit_window_minutes, Some(10080));
    }

    #[test]
    fn codex_activity_uses_bounded_titles_without_tool_results() {
        let command: Value = serde_json::from_str(r#"{"type":"CommandExecution","parsed_cmd":[{"cmd":"rg -n 'island' app/src/main.rs"}],"aggregated_output":"PRIVATE_OUTPUT","exit_code":0}"#).unwrap();
        assert_eq!(command_event(&command), "Run rg -n 'island' app/src/main.rs");
        let failed: Value = serde_json::from_str(r#"{"type":"CommandExecution","parsed_cmd":[{"cmd":"cargo test"}],"exit_code":1}"#).unwrap();
        assert_eq!(command_event(&failed), "Run cargo test · failed");
        let tool: Value = serde_json::from_str(r#"{"type":"McpToolCall","arguments":{"title":"Inspecting original hover app","secret":"PRIVATE_ARGUMENT"},"result":"PRIVATE_RESULT","status":"completed"}"#).unwrap();
        assert_eq!(tool_event(&tool), "Inspecting original hover app");
    }

    #[test]
    fn codex_history_keeps_live_state_and_hides_tool_payloads() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("bookland-codex-history-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.jsonl");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"task_started"}}"#).unwrap();
        writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"UserMessage","content":[{"text":"Build the island"}]}}}"#).unwrap();
        writeln!(file, "{}", " ".repeat(800_000)).unwrap();
        writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"CommandExecution","command":"PRIVATE_COMMAND","status":"completed","exit_code":0}}}"#).unwrap();
        writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"McpToolCall","arguments":"PRIVATE_ARGUMENTS","status":"completed"}}}"#).unwrap();
        writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","content":[{"text":"::browser-complete{title=\"Internal marker\"}"}]}}}"#).unwrap();
        drop(file);
        let result = parse_codex(&path).unwrap();
        assert_eq!(result.title, "Build the island");
        assert_eq!(result.status, "Working");
        assert_eq!(result.events, ["Ran a command", "Used a connected tool"]);
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","content":[{"text":"Checking the island"}]}}}"#).unwrap();
        for _ in 0..6 {
            writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"McpToolCall","status":"completed"}}}"#).unwrap();
        }
        file.flush().unwrap();
        let result = parse_codex(&path).unwrap();
        assert_eq!(result.last_message.as_deref(), Some("Checking the island"));
        assert_eq!(result.events, ["Used a connected tool"; 5]);
        writeln!(file, "{}", " ".repeat(800_000)).unwrap();
        file.flush().unwrap();
        assert_eq!(parse_codex(&path).unwrap().last_message.as_deref(),
            Some("Checking the island"));
        writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"UserMessage","content":[{"text":"Next task"}]}}}"#).unwrap();
        file.flush().unwrap();
        assert!(parse_codex(&path).unwrap().last_message.is_none());
        assert_eq!(parse_codex(&path).unwrap().title, "Build the island");
        writeln!(file, "{}", r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#).unwrap();
        drop(file);
        assert_eq!(parse_codex(&path).unwrap().status, "Finished");
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn external_feed_is_bounded_and_local() {
        let dir = std::env::temp_dir().join(format!("nooklet-feed-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("agents.json");
        fs::write(&path, r#"[{"source":"My Agent","title":"Researching","status":"Working","events":["Started", "Reading"]}]"#).unwrap();
        let rows = external_feed(&path);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].events, vec!["Started", "Reading"]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn claude_transcript_tracks_prompts_and_completion() {
        let dir = std::env::temp_dir().join(format!("nooklet-claude-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.jsonl");
        fs::write(&path, concat!(
            "{\"type\":\"user\",\"message\":{\"content\":\"Build a panel\"}}\n",
            "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"Working on the panel\"}],\"stop_reason\":\"tool_use\"}}\n",
            "{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\",\"content\":\"private output\"}]}}\n",
            "{\"type\":\"system\",\"subtype\":\"turn_duration\"}\n"
        )).unwrap();
        let result = parse_claude(&path).unwrap();
        assert_eq!(result.title, "Build a panel");
        assert_eq!(result.status, "Finished");
        assert_eq!(result.events, vec!["Working on the panel"]);
        let _ = fs::remove_dir_all(dir);
    }
}
