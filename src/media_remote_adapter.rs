use std::{io::Read, path::PathBuf, process::{Command, Stdio}, time::{Duration, Instant}};

fn bundled_paths() -> Option<(PathBuf, PathBuf)> {
    let executable = std::env::current_exe().ok()?;
    let contents = executable.parent()?.parent()?;
    let script = contents.join("Resources/mediaremote-adapter.pl");
    let framework = contents.join("Frameworks/MediaRemoteAdapter.framework");
    (script.is_file() && framework.is_dir()).then_some((script, framework))
}

fn run(action: &str, argument: String) -> bool {
    let Some((script, framework)) = bundled_paths() else { return false; };
    let Ok(mut child) = Command::new("/usr/bin/perl")
        .arg(script).arg(framework).arg(action).arg(argument)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn() else { return false; };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if started.elapsed() < Duration::from_secs(4) =>
                std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

pub fn send(command: u8) -> bool {
    run("send", command.to_string())
}

pub fn seek(seconds: f64) -> bool {
    if !seconds.is_finite() || !(0.0..=86400.0).contains(&seconds) { return false; }
    run("seek", ((seconds * 1_000_000.0).round() as u64).to_string())
}

pub fn now_playing() -> Option<serde_json::Value> {
    let (script, framework) = bundled_paths()?;
    let mut child = Command::new("/usr/bin/perl")
        .arg(script).arg(framework).args(["get", "--now", "--no-artwork"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut result = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let length = match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(length) => length,
                Err(_) => return None,
            };
            if result.len() + length > 64 * 1024 { return None; }
            result.extend_from_slice(&buffer[..length]);
        }
        Some(result)
    });
    let started = Instant::now();
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if started.elapsed() < Duration::from_secs(4) =>
                std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let bytes = reader.join().ok()??;
    if !success { return None; }
    serde_json::from_slice(&bytes).ok()
}
