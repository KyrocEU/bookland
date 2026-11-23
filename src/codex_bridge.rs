use serde_json::Value;
use std::{
    env,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};
use tempfile::NamedTempFile;

#[derive(Debug)]
pub enum CodexEvent {
    Process(u32),
    Thread(String),
    ResetThread,
    Progress(String),
    Message(String),
    Finished(Result<(), String>),
}

pub(crate) fn binary() -> Option<PathBuf> {
    if let Some(path) = env::var_os("BOOKLAND_CODEX_BIN").map(PathBuf::from) {
        if path.is_file() { return Some(path); }
    }
    if let Some(path) = env::var_os("PATH") {
        for folder in env::split_paths(&path) {
            let candidate = folder.join("codex");
            if candidate.is_file() { return Some(candidate); }
        }
    }
    let home = env::var_os("HOME").map(PathBuf::from)?;
    let app_binary = Path::new("ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex");
    [
        home.join("Desktop").join(app_binary),
        home.join("Applications").join(app_binary),
        PathBuf::from("/Applications").join(app_binary),
        home.join(".local/bin/codex"),
        PathBuf::from("/opt/homebrew/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
    ].into_iter().find(|path| path.is_file())
}

fn bookland_codex_home() -> Result<Option<PathBuf>, String> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(None);
    };
    let source_home = env::var_os("CODEX_HOME").map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    let source_auth = source_home.join("auth.json");
    if !source_auth.is_file() {
        return Ok(None);
    }
    let isolated = home.join("Library/Application Support/Bookland/CodexHome");
    fs::create_dir_all(&isolated).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{symlink, PermissionsExt};
        fs::set_permissions(&isolated, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
        let link = isolated.join("auth.json");
        if fs::symlink_metadata(&link).is_err() {
            symlink(&source_auth, &link).map_err(|error| error.to_string())?;
        }
    }
    Ok(Some(isolated))
}

fn event_from_line(line: &str) -> Option<CodexEvent> {
    let value: Value = serde_json::from_str(line).ok()?;
    match value["type"].as_str()? {
        "thread.started" => value["thread_id"].as_str()
            .map(|id| CodexEvent::Thread(id.to_owned())),
        "turn.started" => Some(CodexEvent::Progress("Codex is thinking…".into())),
        "item.started" => {
            let item = &value["item"];
            if item["type"] == "command_execution" {
                Some(CodexEvent::Progress("Codex is running a command…".into()))
            } else { None }
        }
        "item.completed" => {
            let item = &value["item"];
            match item["type"].as_str()? {
                "agent_message" => item["text"].as_str()
                    .map(|text| CodexEvent::Message(text.chars().take(30_000).collect())),
                "file_change" => Some(CodexEvent::Progress("Codex updated project files".into())),
                _ => None,
            }
        }
        "turn.failed" => Some(CodexEvent::Progress(value["error"]["message"].as_str()
            .unwrap_or("Codex failed").to_owned())),
        _ => None,
    }
}

fn dispatch_line(line: &[u8], sender: &Sender<CodexEvent>, started: &mut bool,
                 completed: &mut bool, failure: &mut Option<String>) {
    let line = String::from_utf8_lossy(line);
    if let Ok(value) = serde_json::from_str::<Value>(&line) {
        match value["type"].as_str() {
            Some("thread.started") => *started = true,
            Some("turn.completed") => *completed = true,
            Some("turn.failed") => *failure = Some(value["error"]["message"].as_str()
                .unwrap_or("Codex could not finish this request").to_owned()),
            _ => {}
        }
    }
    if let Some(event) = event_from_line(&line) { let _ = sender.send(event); }
}

fn drain_output(reader: &mut File, pending: &mut Vec<u8>, sender: &Sender<CodexEvent>,
                started: &mut bool, completed: &mut bool,
                failure: &mut Option<String>, final_read: bool) -> std::io::Result<()> {
    reader.read_to_end(pending)?;
    while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
        let line: Vec<u8> = pending.drain(..=end).collect();
        dispatch_line(&line, sender, started, completed, failure);
    }
    if final_read && !pending.is_empty() {
        dispatch_line(pending, sender, started, completed, failure);
        pending.clear();
    }
    Ok(())
}

fn execute(prompt: String, workspace: PathBuf, session: Option<String>,
           allow_edits: bool, sender: Sender<CodexEvent>) {
    let Some(binary) = binary() else {
        let _ = sender.send(CodexEvent::Finished(Err("Codex CLI was not found on this Mac".into())));
        return;
    };
    if !workspace.is_dir() {
        let _ = sender.send(CodexEvent::Finished(Err("Bookland project folder is unavailable".into())));
        return;
    }
    let mut command = Command::new(binary);
    // The desktop app can inherit Codex's own tool pipes. A nested CLI must start
    // with a normal user environment so it does not attach to the parent's pipes.
    command.env_clear();
    for key in ["HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "LC_ALL",
        "CODEX_HOME", "OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_ORG_ID",
        "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "SSH_AUTH_SOCK"] {
        if let Some(value) = env::var_os(key) { command.env(key, value); }
    }
    command.env("PATH", env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()));
    match bookland_codex_home() {
        Ok(Some(home)) => { command.env("CODEX_HOME", home); }
        Ok(None) => {}
        Err(error) => {
            let _ = sender.send(CodexEvent::Finished(Err(format!(
                "Could not prepare Bookland Codex home: {error}"))));
            return;
        }
    }
    command.arg("exec");
    let background_flags = ["--disable", "plugins", "--disable", "remote_plugin",
        "--disable", "shell_snapshot", "--enable", "skip_host_skill_discovery",
        "-c", "suppress_unstable_features_warning=true"];
    command.args(background_flags)
        .args(["--json", "--skip-git-repo-check", "-C"])
        .arg(&workspace);
    if allow_edits { command.arg("--approve-for-me"); }
    else { command.args(["--sandbox", "read-only"]); }
    if let Some(id) = &session { command.arg("resume").arg(id); }
    // Redirect to private temporary files so CLI output stays available until
    // the background worker has read the full response.
    let output = match NamedTempFile::new() {
        Ok(file) => file,
        Err(error) => { let _ = sender.send(CodexEvent::Finished(Err(error.to_string()))); return; }
    };
    let errors = match NamedTempFile::new() {
        Ok(file) => file,
        Err(error) => { let _ = sender.send(CodexEvent::Finished(Err(error.to_string()))); return; }
    };
    let (output_writer, mut output_reader, errors_writer) = match
        (output.reopen(), output.reopen(), errors.reopen()) {
        (Ok(writer), Ok(reader), Ok(stderr)) => (writer, reader, stderr),
        _ => { let _ = sender.send(CodexEvent::Finished(Err("Could not prepare Codex output".into()))); return; }
    };
    command.arg(prompt).current_dir(workspace)
        .stdin(Stdio::null()).stdout(Stdio::from(output_writer)).stderr(Stdio::from(errors_writer));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = sender.send(CodexEvent::Finished(Err(error.to_string())));
            return;
        }
    };
    let _ = sender.send(CodexEvent::Process(child.id()));
    let start_time = Instant::now();
    let mut started = false;
    let mut timed_out = false;
    let mut pending = Vec::new();
    let mut completed = false;
    let mut failure = None;
    let status = loop {
        let _ = drain_output(&mut output_reader, &mut pending, &sender,
            &mut started, &mut completed, &mut failure, false);
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(error) => break Err(error),
        }
        if !started && start_time.elapsed() >= Duration::from_secs(30) {
            timed_out = true;
            let _ = stop(child.id());
        }
        thread::sleep(Duration::from_millis(80));
    };
    let _ = drain_output(&mut output_reader, &mut pending, &sender,
        &mut started, &mut completed, &mut failure, true);
    let stderr = std::fs::read_to_string(errors.path()).unwrap_or_default();
    let result = if let Some(error) = failure { Err(error) }
        else if timed_out {
            Err("The Codex CLI did not start within 30 seconds. Open Codex to continue there.".into())
        }
        else if status.as_ref().is_ok_and(|status| status.success()) && completed { Ok(()) }
        else {
            let detail = stderr.lines().rev().find(|line| !line.trim().is_empty())
                .unwrap_or("Codex exited before completing the reply");
            Err(detail.chars().take(300).collect())
        };
    if session.is_some() && !started { let _ = sender.send(CodexEvent::ResetThread); }
    let _ = sender.send(CodexEvent::Finished(result));
}

pub fn run(prompt: String, workspace: PathBuf, session: Option<String>,
           allow_edits: bool) -> Receiver<CodexEvent> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || execute(prompt, workspace, session, allow_edits, sender));
    receiver
}

pub fn stop(process_id: u32) -> Result<(), String> {
    let status = Command::new("/bin/kill").args(["-TERM", &process_id.to_string()])
        .status().map_err(|error| error.to_string())?;
    if status.success() { Ok(()) } else { Err("Could not stop Codex".into()) }
}

#[cfg(test)]
mod tests {
    use super::{event_from_line, CodexEvent};

    #[test]
    fn parses_codex_reply_and_thread() {
        assert!(matches!(event_from_line(r#"{"type":"thread.started","thread_id":"abc"}"#),
            Some(CodexEvent::Thread(id)) if id == "abc"));
        assert!(matches!(event_from_line(r#"{"type":"item.completed","item":{"type":"agent_message","text":"Hello"}}"#),
            Some(CodexEvent::Message(text)) if text == "Hello"));
    }
}
