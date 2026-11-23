use std::{fs, process::Command};
use tempfile::NamedTempFile;

const MUSIC_SCRIPT: &str = include_str!("music_artwork.applescript");
const MAX_ART_BYTES: usize = 5_000_000;

fn music_artwork() -> Result<Option<Vec<u8>>, String> {
    let file = NamedTempFile::new().map_err(|error| error.to_string())?;
    let output = Command::new("osascript")
        .arg("-e").arg(MUSIC_SCRIPT).arg(file.path())
        .output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().chars().take(200).collect());
    }
    if !String::from_utf8_lossy(&output.stdout).contains("saved") { return Ok(None); }
    let bytes = fs::read(file.path()).map_err(|error| error.to_string())?;
    if bytes.is_empty() || bytes.len() > MAX_ART_BYTES { return Ok(None); }
    Ok(Some(bytes))
}

fn spotify_artwork() -> Result<Option<Vec<u8>>, String> {
    let output = Command::new("osascript")
        .args(["-e", "with timeout of 8 seconds\ntell application \"Spotify\" to artwork url of current track\nend timeout"])
        .output().map_err(|error| error.to_string())?;
    if !output.status.success() { return Ok(None); }
    let url = String::from_utf8_lossy(&output.stdout);
    let url = url.trim();
    if !url.starts_with("https://") || url.len() > 2048 { return Ok(None); }
    let image = Command::new("curl")
        .args(["--silent", "--show-error", "--fail", "--location", "--max-time", "8",
            "--max-filesize", "5000000", "--proto", "=https", "--proto-redir", "=https"])
        .arg(url).output().map_err(|error| error.to_string())?;
    if !image.status.success() || image.stdout.is_empty() || image.stdout.len() > MAX_ART_BYTES {
        return Ok(None);
    }
    Ok(Some(image.stdout))
}

pub fn fetch(app: &str) -> Result<Option<Vec<u8>>, String> {
    match app {
        "Music" => music_artwork(),
        "Spotify" => spotify_artwork(),
        _ => Ok(None),
    }
}
