use crate::codex_bridge;
use serde_json::{json, Value};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

#[derive(Clone, Debug)]
pub struct Approval {
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct UserQuestion {
    pub id: String,
    pub header: String,
    pub prompt: String,
    pub options: Vec<(String, String)>,
    pub allow_other: bool,
    pub secret: bool,
}

#[derive(Clone, Debug)]
pub struct UserInput {
    pub questions: Vec<UserQuestion>,
    pub is_blocking: bool,
}

pub enum Event {
    Process(u32),
    Progress(String),
    Message(String),
    Approval(Approval),
    Question(UserInput),
    Finished(Result<(), String>),
}

pub enum Decision { AllowOnce, Deny, Answer(Vec<(String, String)>) }

fn is_approval_request(method: &str) -> bool {
    matches!(method, "item/commandExecution/requestApproval" |
        "item/fileChange/requestApproval")
}

fn is_user_input_request(method: &str) -> bool {
    method == "item/tool/requestUserInput"
}

fn request(stdin: &mut impl Write, stdout: &mut impl BufRead, pending: &mut Vec<Value>,
    id: i32, method: &str, params: Value) -> Result<Value, String> {
    serde_json::to_writer(&mut *stdin,
        &json!({"id": id, "method": method, "params": params}))
        .map_err(|error| error.to_string())?;
    stdin.write_all(b"\n").and_then(|_| stdin.flush())
        .map_err(|error| error.to_string())?;
    let mut line = String::new();
    loop {
        line.clear();
        if stdout.read_line(&mut line).map_err(|error| error.to_string())? == 0 {
            return Err("Codex desktop connection ended".into());
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
        if let Some(incoming_method) = value["method"].as_str() {
            if incoming_method == "item/completed" || incoming_method == "turn/completed"
                || !value["id"].is_null() {
                pending.push(value);
            }
            continue;
        }
        if value["id"] != id { continue; }
        if !value["error"].is_null() {
            return Err(value["error"]["message"].as_str()
                .unwrap_or("Codex desktop rejected the request").to_owned());
        }
        return Ok(value["result"].clone());
    }
}

fn notify(stdin: &mut impl Write, method: &str) -> Result<(), String> {
    serde_json::to_writer(&mut *stdin, &json!({"method": method}))
        .map_err(|error| error.to_string())?;
    stdin.write_all(b"\n").and_then(|_| stdin.flush())
        .map_err(|error| error.to_string())
}

fn approval_display(value: &Value) -> Approval {
    let params = &value["params"];
    let command = params["command"].as_str().unwrap_or("");
    let reason = params["reason"].as_str().unwrap_or("");
    let root = params["grantRoot"].as_str().unwrap_or("");
    let cwd = params["cwd"].as_str().unwrap_or("");
    let title = if value["method"] == "item/fileChange/requestApproval" {
        "Codex requests a file change".to_owned()
    } else { "Codex requests a command".to_owned() };
    let detail = [reason, command, root, cwd].into_iter().filter(|part| !part.is_empty())
        .collect::<Vec<_>>().join(" · ");
    Approval { title, detail }
}

fn answer_approval(stdin: &mut impl Write, id: &Value, decision: Decision)
    -> Result<(), String> {
    let reply = json!({"id": id, "result": {"decision": match decision {
        Decision::AllowOnce => "accept", Decision::Deny | Decision::Answer(_) => "decline" }}});
    serde_json::to_writer(&mut *stdin, &reply).map_err(|error| error.to_string())?;
    stdin.write_all(b"\n").and_then(|_| stdin.flush())
        .map_err(|error| error.to_string())
}

fn user_input_display(value: &Value) -> Result<UserInput, String> {
    let params = &value["params"];
    let questions = params["questions"].as_array()
        .ok_or("Codex sent an invalid question request")?;
    if questions.is_empty() || questions.len() > 8 {
        return Err("Codex sent an unsupported number of questions".into());
    }
    let mut displayed = Vec::with_capacity(questions.len());
    for question in questions {
        let id = question["id"].as_str().filter(|id| !id.is_empty())
            .ok_or("Codex sent a question without an ID")?;
        let prompt = question["question"].as_str()
            .ok_or("Codex sent a question without text")?;
        let options = question["options"].as_array().map(|options| options.iter()
            .filter_map(|option| Some((option["label"].as_str()?.to_owned(),
                option["description"].as_str().unwrap_or("").to_owned())))
            .collect()).unwrap_or_default();
        displayed.push(UserQuestion { id: id.to_owned(),
            header: question["header"].as_str().unwrap_or("").to_owned(),
            prompt: prompt.to_owned(), options,
            allow_other: question["isOther"].as_bool().unwrap_or(false),
            secret: question["isSecret"].as_bool().unwrap_or(false) });
    }
    Ok(UserInput { questions: displayed,
        is_blocking: params["isBlocking"].as_bool().unwrap_or(true) })
}

fn answer_user_input(stdin: &mut impl Write, id: &Value,
    answers: Vec<(String, String)>) -> Result<(), String> {
    let answers = answers.into_iter().filter(|(_, answer)| !answer.trim().is_empty())
        .map(|(question, answer)| (question, json!({"answers": [answer]})))
        .collect::<serde_json::Map<String, Value>>();
    serde_json::to_writer(&mut *stdin,
        &json!({"id": id, "result": {"answers": answers}}))
        .map_err(|error| error.to_string())?;
    stdin.write_all(b"\n").and_then(|_| stdin.flush())
        .map_err(|error| error.to_string())
}

fn answer_unsupported(stdin: &mut impl Write, id: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *stdin, &json!({"id": id, "error": {
        "code": -32601, "message": "This Codex request is not supported in the Bookland island"
    }})).map_err(|error| error.to_string())?;
    stdin.write_all(b"\n").and_then(|_| stdin.flush())
        .map_err(|error| error.to_string())
}

fn original_settings(thread_id: &str, resumed: &Value) -> Result<Value, String> {
    let sandbox = &resumed["sandbox"];
    let mode = match sandbox["type"].as_str() {
        Some("readOnly") if sandbox["networkAccess"] == false => "read-only",
        Some("workspaceWrite") if sandbox["networkAccess"] == false
            && sandbox["writableRoots"].as_array().is_some_and(Vec::is_empty)
            && sandbox["excludeTmpdirEnvVar"] == false
            && sandbox["excludeSlashTmp"] == false => "workspace-write",
        Some("dangerFullAccess") => "danger-full-access",
        _ => return Err("This Codex chat has custom sandbox settings that Bookland cannot preserve yet".into()),
    };
    if resumed["approvalPolicy"].is_null() || resumed["approvalsReviewer"].is_null() {
        return Err("Codex did not expose this chat's permission settings".into());
    }
    Ok(json!({"threadId": thread_id, "excludeTurns": true,
        "approvalPolicy": resumed["approvalPolicy"],
        "approvalsReviewer": resumed["approvalsReviewer"], "sandbox": mode}))
}

fn codex_home() -> Result<PathBuf, String> {
    env::var_os("CODEX_HOME").map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .ok_or("Codex home is unavailable".into())
}

fn app_server_command(binary: &Path, home: &Path) -> Command {
    let mut command = Command::new(binary);
    command.env_clear();
    for key in ["HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "LC_ALL",
        "OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_ORG_ID",
        "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "SSH_AUTH_SOCK"] {
        if let Some(value) = env::var_os(key) { command.env(key, value); }
    }
    command.env("CODEX_HOME", home)
        .env("PATH", env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()))
        .args(["app-server", "--disable", "plugins", "--disable", "remote_plugin"])
        .current_dir(crate::codex_workspace())
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    command
}

fn restore_dir() -> Result<PathBuf, String> {
    env::var_os("HOME").map(|home| PathBuf::from(home)
        .join("Library/Application Support/Bookland/DesktopCodexRestore"))
        .ok_or("Bookland home is unavailable".into())
}

fn record_path(dir: &Path, home: &Path, thread_id: &str) -> PathBuf {
    let tag = home.to_string_lossy().as_bytes().iter().fold(
        0xcbf29ce484222325_u64, |hash, byte| (hash ^ u64::from(*byte))
            .wrapping_mul(0x100000001b3));
    dir.join(format!("{thread_id}-{tag:016x}.json"))
}

fn persist_restore_at(dir: &Path, home: &Path, params: &Value)
    -> Result<PathBuf, String> {
    let thread_id = params["threadId"].as_str()
        .ok_or("Codex restore record has no thread ID")?;
    if thread_id.len() != 36 || !thread_id.bytes().all(|byte|
        byte.is_ascii_hexdigit() || byte == b'-') {
        return Err("Codex restore record has an invalid thread ID".into());
    }
    fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())?;
    let record = record_path(dir, home, thread_id);
    if record.exists() {
        return Err("A prior Codex permission restore is still pending for this chat".into());
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?.as_nanos();
    let temp = dir.join(format!(".{thread_id}-{}-{stamp}.tmp", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&temp).map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut file, &json!({"home": home.to_string_lossy(),
            "params": params})).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::hard_link(&temp, &record).map_err(|error| error.to_string())?;
        Ok(())
    })();
    let _ = fs::remove_file(&temp);
    result.map(|()| record)
}

fn clear_restore(record: &Path) -> Result<(), String> {
    match fs::remove_file(record) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn restore_settings(command: &mut Command, params: Value, check_idle: bool)
    -> Result<(), String> {
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let outcome = (|| -> Result<(), String> {
        let mut stdin = child.stdin.take().ok_or("Codex restore input unavailable")?;
        let mut stdout = BufReader::new(child.stdout.take()
            .ok_or("Codex restore output unavailable")?);
        let mut pending = Vec::new();
        request(&mut stdin, &mut stdout, &mut pending, 11, "initialize",
            json!({"clientInfo": {"name": "bookland", "title": "Bookland Island",
                "version": env!("CARGO_PKG_VERSION")}}))?;
        notify(&mut stdin, "initialized")?;
        if check_idle {
            let metadata = request(&mut stdin, &mut stdout, &mut pending, 12,
                "thread/read", json!({"threadId": params["threadId"]}))?;
            if metadata["thread"]["status"]["type"] == "active" {
                return Err("Codex chat is still active; permission recovery will wait".into());
            }
        }
        request(&mut stdin, &mut stdout, &mut pending, 13, "thread/resume", params)?;
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    outcome
}

fn recover_pending() -> Result<(), String> {
    let dir = restore_dir()?;
    if !dir.is_dir() { return Ok(()); }
    let home = codex_home()?;
    let mut first_error = None;
    for entry in fs::read_dir(&dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "json")
            || !entry.file_type().map_err(|error| error.to_string())?.is_file() {
            continue;
        }
        let record: Value = match fs::read(&path).map_err(|error| error.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes)
                .map_err(|error| error.to_string())) {
            Ok(record) => record,
            Err(error) => { first_error.get_or_insert(error); continue; }
        };
        if record["home"] != home.to_string_lossy().as_ref() { continue; }
        let params = &record["params"];
        let Some(id) = params["threadId"].as_str() else {
            first_error.get_or_insert("Codex restore record is missing a thread ID".into());
            continue;
        };
        if record_path(&dir, &home, id) != path {
            first_error.get_or_insert("Codex restore record has an unexpected name".into());
            continue;
        }
        let result = (|| -> Result<(), String> {
            let binary = codex_bridge::binary()
                .ok_or("Codex CLI was not found for permission recovery")?;
            let mut command = app_server_command(&binary, &home);
            restore_settings(&mut command, params.clone(), true)?;
            clear_restore(&path)
        })();
        if let Err(error) = result { first_error.get_or_insert(error); }
    }
    first_error.map_or(Ok(()), Err)
}

pub fn recover_pending_async() {
    thread::spawn(|| {
        if let Err(error) = recover_pending() {
            eprintln!("Bookland Codex permission recovery: {error}");
        }
    });
}

fn execute(thread_id: String, prompt: String, allow_edits: bool,
    sender: Sender<Event>, decisions: Receiver<Decision>) {
    let result = (|| -> Result<(), String> {
        let binary = codex_bridge::binary()
            .ok_or("Codex CLI was not found on this Mac")?;
        let home = codex_home()?;
        let dir = restore_dir()?;
        let prior_record = record_path(&dir, &home, &thread_id);
        if prior_record.exists() {
            recover_pending()?;
            if prior_record.exists() {
                return Err("A previous Codex permission restore is pending for this chat".into());
            }
        }
        let mut command = app_server_command(&binary, &home);
        let mut child = command.spawn().map_err(|error| error.to_string())?;
        let _ = sender.send(Event::Process(child.id()));
        let mut settings_to_restore = None;
        let mut restore_record = None;
        let outcome = (|| -> Result<(), String> {
            let mut stdin = child.stdin.take().ok_or("Codex desktop input unavailable")?;
            let mut stdout = BufReader::new(child.stdout.take()
                .ok_or("Codex desktop output unavailable")?);
            let mut pending = Vec::new();
            request(&mut stdin, &mut stdout, &mut pending, 1, "initialize",
                json!({"clientInfo": {"name": "bookland", "title": "Bookland Island",
                    "version": env!("CARGO_PKG_VERSION")}}))?;
            notify(&mut stdin, "initialized")?;
            let metadata = request(&mut stdin, &mut stdout, &mut pending, 2, "thread/read",
                json!({"threadId": thread_id}))?;
            if metadata["thread"]["status"]["type"] == "active" {
                return Err("This Codex chat is already working. Continue it after the current turn ends.".into());
            }
            let resumed = request(&mut stdin, &mut stdout, &mut pending, 3, "thread/resume",
                json!({"threadId": thread_id, "excludeTurns": true}))?;
            if resumed["thread"]["status"]["type"] == "active" {
                return Err("This Codex chat became active in another window. Try again after it finishes.".into());
            }
            let original = original_settings(&thread_id, &resumed)?;
            restore_record = Some(persist_restore_at(&dir, &home, &original)?);
            settings_to_restore = Some(original);
            let _ = sender.send(Event::Progress("Sending to the selected Codex chat…".into()));
            let (approval_policy, sandbox) = if allow_edits {
                ("on-request", json!({"type": "workspaceWrite", "writableRoots": [],
                    "networkAccess": false, "excludeTmpdirEnvVar": false,
                    "excludeSlashTmp": false}))
            } else { ("never", json!({"type": "readOnly", "networkAccess": false})) };
            let started = request(&mut stdin, &mut stdout, &mut pending, 4, "turn/start", json!({
                "threadId": thread_id,
                "input": [{"type": "text", "text": prompt, "text_elements": []}],
                "approvalPolicy": approval_policy,
                "approvalsReviewer": "user",
                "sandboxPolicy": sandbox
            }))?;
            let turn_id = started["turn"]["id"].as_str()
                .ok_or("Codex did not identify the started turn")?;
            let mut line = String::new();
            loop {
                let value = if pending.is_empty() {
                    line.clear();
                    if stdout.read_line(&mut line).map_err(|error| error.to_string())? == 0 {
                        return Err("Codex desktop connection ended before the reply finished".into());
                    }
                    let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
                    value
                } else { pending.remove(0) };
                let params = &value["params"];
                if let Some(method) = value["method"].as_str() {
                    if !value["id"].is_null() && !is_approval_request(method)
                        && !is_user_input_request(method) {
                        answer_unsupported(&mut stdin, &value["id"])?;
                        continue;
                    }
                }
                let method = value["method"].as_str().unwrap_or("");
                if !value["id"].is_null()
                    && (is_approval_request(method) || is_user_input_request(method))
                    && (params["threadId"] != thread_id || params["turnId"] != turn_id) {
                    answer_unsupported(&mut stdin, &value["id"])?;
                    continue;
                }
                if params["threadId"] != thread_id { continue; }
                if !value["id"].is_null() && is_user_input_request(method)
                    && params["turnId"] == turn_id {
                    let input = user_input_display(&value)?;
                    let blocking = input.is_blocking;
                    let _ = sender.send(Event::Question(input));
                    let decision = if blocking { decisions.recv().unwrap_or(Decision::Deny) }
                        else {
                            let ms = params["autoResolutionMs"].as_u64()
                                .unwrap_or(15_000).clamp(1_000, 30_000);
                            decisions.recv_timeout(Duration::from_millis(ms))
                                .unwrap_or(Decision::Answer(Vec::new()))
                        };
                    match decision {
                        Decision::Answer(answers) =>
                            answer_user_input(&mut stdin, &value["id"], answers)?,
                        _ => answer_unsupported(&mut stdin, &value["id"])?,
                    }
                    let _ = sender.send(Event::Progress("Waiting for Codex…".into()));
                    continue;
                }
                if let Some(method) = value["method"].as_str() {
                    if is_approval_request(method) && params["turnId"] == turn_id {
                        let id = &value["id"];
                        if id.is_null() { return Err("Codex approval has no request ID".into()); }
                        if allow_edits {
                            let _ = sender.send(Event::Approval(approval_display(&value)));
                            let decision = decisions.recv().unwrap_or(Decision::Deny);
                            answer_approval(&mut stdin, id, decision)?;
                            let _ = sender.send(Event::Progress("Waiting for Codex…".into()));
                        } else {
                            answer_approval(&mut stdin, id, Decision::Deny)?;
                        }
                        continue;
                    }
                }
                match value["method"].as_str() {
                    Some("item/completed") if params["turnId"] == turn_id
                        && params["item"]["type"] == "agentMessage" => {
                        if let Some(message) = params["item"]["text"].as_str() {
                            let _ = sender.send(Event::Message(
                                message.chars().take(30_000).collect()));
                        }
                    }
                    Some("turn/completed") if params["turn"]["id"] == turn_id => {
                        return if params["turn"]["status"] == "completed" { Ok(()) }
                            else { Err(params["turn"]["error"]["message"].as_str()
                                .unwrap_or("Codex did not finish this turn").to_owned()) };
                    }
                    _ => {}
                }
            }
        })();
        let _ = child.kill();
        let _ = child.wait();
        let restored = settings_to_restore.map(|params| {
            restore_settings(&mut command, params, true)?;
            if let Some(record) = &restore_record { clear_restore(record)?; }
            Ok::<(), String>(())
        });
        match (outcome, restored) {
            (Ok(()), Some(Err(error))) => Err(format!(
                "Reply finished, but the chat's prior permission settings could not be restored: {error}")),
            (Err(error), Some(Err(restore_error))) => Err(format!(
                "{error}; prior permission settings could not be restored: {restore_error}")),
            (outcome, _) => outcome,
        }
    })();
    let _ = sender.send(Event::Finished(result));
}

pub fn run(thread_id: String, prompt: String, allow_edits: bool)
    -> (Receiver<Event>, Sender<Decision>) {
    let (sender, receiver) = mpsc::channel();
    let (decisions_tx, decisions_rx) = mpsc::channel();
    thread::spawn(move || execute(thread_id, prompt, allow_edits, sender, decisions_rx));
    (receiver, decisions_tx)
}

#[cfg(test)]
mod tests {
    use super::{answer_approval, answer_user_input, approval_display,
        clear_restore, original_settings, persist_restore_at, request,
        user_input_display, Decision};
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn preserves_fast_reply_before_start_acknowledgment() {
        let output = b"{\"method\":\"turn/completed\",\"params\":{\"threadId\":\"thread\",\"turn\":{\"id\":\"turn\",\"status\":\"completed\"}}}\n{\"id\":4,\"result\":{\"turn\":{\"id\":\"turn\"}}}\n";
        let mut input = Vec::new();
        let mut pending = Vec::new();
        let result = request(&mut input, &mut Cursor::new(output), &mut pending,
            4, "turn/start", json!({})).unwrap();
        assert_eq!(result["turn"]["id"], "turn");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0]["method"], "turn/completed");
    }

    #[test]
    fn approval_response_is_single_use_and_keeps_server_request_id() {
        let request = json!({"method": "item/commandExecution/requestApproval",
            "id": "approval-7", "params": {"command": "printf test",
                "cwd": "/tmp/bookland", "reason": "Needs access"}});
        let display = approval_display(&request);
        assert_eq!(display.title, "Codex requests a command");
        assert!(display.detail.contains("printf test"));
        let mut output = Vec::new();
        answer_approval(&mut output, &request["id"], Decision::AllowOnce).unwrap();
        let answer: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(answer["id"], "approval-7");
        assert_eq!(answer["result"]["decision"], "accept");
    }

    #[test]
    fn restores_only_sandbox_modes_that_can_be_round_tripped() {
        let original = json!({"approvalPolicy": "on-request",
            "approvalsReviewer": "user", "sandbox": {"type": "readOnly",
                "networkAccess": false}});
        let params = original_settings("thread", &original).unwrap();
        assert_eq!(params["sandbox"], "read-only");
        assert_eq!(params["approvalPolicy"], "on-request");
        let custom = json!({"approvalPolicy": "never", "approvalsReviewer": "user",
            "sandbox": {"type": "readOnly", "networkAccess": true}});
        assert!(original_settings("thread", &custom).is_err());
    }

    #[test]
    fn parses_multiple_codex_questions_and_answers_by_question_id() {
        let request = json!({"method": "item/tool/requestUserInput", "id": 72,
            "params": {"threadId": "thread", "turnId": "turn", "isBlocking": true,
                "questions": [
                    {"id": "layout", "header": "Layout", "question": "Choose a layout",
                        "isOther": true, "isSecret": false,
                        "options": [{"label": "Compact", "description": "Smaller island"}]},
                    {"id": "name", "header": "Name", "question": "What name?",
                        "isOther": true, "isSecret": false, "options": null}]}});
        let displayed = user_input_display(&request).unwrap();
        assert_eq!(displayed.questions.len(), 2);
        assert_eq!(displayed.questions[0].options[0].0, "Compact");
        let mut output = Vec::new();
        answer_user_input(&mut output, &request["id"], vec![
            ("layout".into(), "Compact".into()), ("name".into(), "Bookland".into())]).unwrap();
        let answer: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(answer["result"]["answers"]["layout"]["answers"][0], "Compact");
        assert_eq!(answer["result"]["answers"]["name"]["answers"][0], "Bookland");
    }

    #[test]
    fn permission_restore_record_is_complete_before_it_becomes_visible() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("bookland-restore-test-{}-{stamp}",
            std::process::id()));
        let home = dir.join("isolated-codex-home");
        let id = "01a206cd-698b-7db1-bbfb-957fecf84abd";
        let params = json!({"threadId": id, "approvalPolicy": "never",
            "approvalsReviewer": "user", "sandbox": "read-only"});
        let path = persist_restore_at(&dir, &home, &params).unwrap();
        let stored: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(stored["params"], params);
        assert_eq!(stored["home"], home.to_string_lossy().as_ref());
        assert!(persist_restore_at(&dir, &home, &params).is_err());
        clear_restore(&path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
}
