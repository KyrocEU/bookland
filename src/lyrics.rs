use serde_json::Value;
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub seconds: f32,
    pub text: String,
}

pub fn parse_lrc(input: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    for raw in input.lines().take(1000) {
        let Some((stamp, text)) = raw.trim().strip_prefix('[')
            .and_then(|line| line.split_once(']')) else { continue; };
        let Some((minutes, seconds)) = stamp.split_once(':') else { continue; };
        let (Ok(minutes), Ok(seconds)) = (minutes.parse::<u32>(), seconds.parse::<f32>())
            else { continue; };
        if !seconds.is_finite() || !(0.0..60.0).contains(&seconds) { continue; }
        lines.push(Line { seconds: minutes as f32 * 60.0 + seconds,
            text: text.trim().chars().take(500).collect() });
    }
    lines.sort_by(|a, b| a.seconds.total_cmp(&b.seconds));
    lines
}

pub fn active_line(lines: &[Line], position: f32) -> Option<usize> {
    lines.iter().rposition(|line| line.seconds <= position)
}

fn request(path: &str, title: &str, artist: &str) -> Option<Value> {
    let output = Command::new("curl")
        .args(["--get", "--silent", "--show-error", "--fail", "--max-time", "8"])
        .arg("--data-urlencode").arg(format!("track_name={title}"))
        .arg("--data-urlencode").arg(format!("artist_name={artist}"))
        .arg(format!("https://lrclib.net/api/{path}"))
        .output().ok()?;
    if !output.status.success() || output.stdout.len() > 1_000_000 { return None; }
    serde_json::from_slice(&output.stdout).ok()
}

pub fn fetch(title: &str, artist: &str) -> Result<Vec<Line>, String> {
    if title.trim().is_empty() || artist.trim().is_empty() {
        return Err("No song metadata for lyrics".into());
    }
    if let Some(data) = request("get", title, artist) {
        let lines = data["syncedLyrics"].as_str().map(parse_lrc).unwrap_or_default();
        if !lines.is_empty() { return Ok(lines); }
    }
    if let Some(matches) = request("search", title, artist).and_then(|data| data.as_array().cloned()) {
        let title = title.trim().to_lowercase();
        let artist = artist.trim().to_lowercase();
        for track in matches.into_iter().take(20) {
            if track["trackName"].as_str().is_some_and(|value| value.trim().to_lowercase() == title)
                && track["artistName"].as_str().is_some_and(|value| value.trim().to_lowercase() == artist) {
                let lines = track["syncedLyrics"].as_str().map(parse_lrc).unwrap_or_default();
                if !lines.is_empty() { return Ok(lines); }
            }
        }
    }
    Err("Synced lyrics were not found".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_tracks_synced_lines() {
        let lines = parse_lrc("[00:12.34] First\n[00:18.00] Second\n[offset:0]\n[00:30.10] Third");
        assert_eq!(lines.len(), 3);
        assert_eq!(active_line(&lines, 11.0), None);
        assert_eq!(active_line(&lines, 18.2), Some(1));
        assert_eq!(lines[2].text, "Third");
    }
}
