use std::{process::{Command, Stdio}, time::{Duration, Instant}};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AirPlayDevice {
    pub name: String,
    pub available: bool,
    pub selected: bool,
    pub supports_audio: bool,
}

const LIST_SCRIPT: &str = r#"
tell application "Music"
    set rows to {}
    repeat with route in AirPlay devices
        set end of rows to (name of route as text) & (character id 31) & (available of route as text) & (character id 31) & (selected of route as text) & (character id 31) & (supports audio of route as text)
    end repeat
    set AppleScript's text item delimiters to character id 30
    return rows as text
end tell
"#;

const SELECT_SCRIPT: &str = r#"
on run argv
    set targetName to item 1 of argv
    set shouldSelect to ((item 2 of argv) is "true")
    tell application "Music"
        repeat with route in AirPlay devices
            if (name of route as text) is targetName then
                if not (available of route) then error "The AirPlay device is unavailable"
                set selected of route to shouldSelect
                return "OK"
            end if
        end repeat
    end tell
    error "The AirPlay device was not found"
end run
"#;

fn parse_devices(output: &str) -> Vec<AirPlayDevice> {
    output.trim_end_matches(['\n', '\r']).split('\u{1e}').filter_map(|line| {
        let mut columns = line.split('\u{1f}');
        let name = columns.next()?.trim();
        let available = columns.next()? == "true";
        let selected = columns.next()? == "true";
        let supports_audio = columns.next()? == "true";
        if name.is_empty() || columns.next().is_some() { return None; }
        Some(AirPlayDevice {
            name: name.into(), available, selected, supports_audio,
        })
    }).collect()
}

fn osascript(script: &str, arguments: &[&str]) -> Result<String, String> {
    let mut child = Command::new("/usr/bin/osascript")
        .args(["-e", script, "--"])
        .args(arguments)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|error| error.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().map_err(|error| error.to_string())?;
                if status.success() { return Ok(String::from_utf8_lossy(&output.stdout).to_string()); }
                let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                return Err(if detail.is_empty() { "Apple Music did not respond".into() }
                    else { detail.chars().take(180).collect() });
            }
            Ok(None) if started.elapsed() < Duration::from_secs(3) =>
                std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Apple Music did not respond within 3 seconds".into());
            }
        }
    }
}

pub fn list_music() -> Result<Vec<AirPlayDevice>, String> {
    osascript(LIST_SCRIPT, &[]).map(|output| parse_devices(&output))
}

pub fn set_music(name: &str, selected: bool) -> Result<(), String> {
    let result = osascript(SELECT_SCRIPT, &[name, if selected { "true" } else { "false" }])?;
    if result.trim() == "OK" { Ok(()) } else { Err("Apple Music did not change the AirPlay route".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_music_routes_and_ignores_incomplete_records() {
        let output = "Living Room\u{1f}true\u{1f}false\u{1f}true\u{1e}Mac\u{1f}true\u{1f}true\u{1f}true\u{1e}bad\u{1f}true\n";
        let devices = parse_devices(output);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "Living Room");
        assert!(!devices[0].selected);
        assert!(devices[1].selected);
    }
}
