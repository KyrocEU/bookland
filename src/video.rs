use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::Sender,
};

#[derive(Clone, Default)]
pub struct VideoProgress {
    pub percent: Option<f32>,
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    pub speed: Option<u64>,
    pub eta: Option<u64>,
}

pub enum VideoEvent {
    Started(u32),
    Progress(VideoProgress),
    Saved(PathBuf),
    Finished(Result<(), String>),
}

pub fn youtube_url(value: &str) -> Option<String> {
    let value = value.trim();
    let after_scheme = value.strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))?;
    let (host, rest) = after_scheme.split_once('/').unwrap_or((after_scheme, ""));
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let host = host.strip_prefix("m.").unwrap_or(host);
    let host = host.strip_prefix("music.").unwrap_or(host);
    let id = if host == "youtu.be" {
        rest.split(['?', '#', '/']).next().unwrap_or("")
    } else if host == "youtube.com" {
        if let Some(query) = rest.strip_prefix("watch?") {
            query.split('&').find_map(|pair| pair.strip_prefix("v=")).unwrap_or("")
                .split(['&', '#']).next().unwrap_or("")
        } else {
            ["shorts/", "live/", "embed/"].iter()
                .find_map(|prefix| rest.strip_prefix(prefix))?
                .split(['?', '#', '/']).next().unwrap_or("")
        }
    } else { return None; };
    if id.len() != 11 || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_') {
        return None;
    }
    Some(format!("https://www.youtube.com/watch?v={id}"))
}

pub fn tool_path() -> Option<PathBuf> {
    let bundled = std::env::current_exe().ok()?.parent()?.parent()?
        .join("Resources/yt-dlp_macos");
    if bundled.is_file() { return Some(bundled); }
    let local = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/yt-dlp_macos");
    local.is_file().then_some(local)
}

fn parse_progress(line: &str) -> Option<VideoProgress> {
    let text = line.strip_prefix("BOOKLAND_PROGRESS ")?;
    let parts: Vec<_> = text.split('|').collect();
    if parts.len() != 5 { return None; }
    Some(VideoProgress {
        percent: parts[0].trim().trim_end_matches('%').parse().ok(),
        downloaded: parts[1].trim().parse().ok(),
        total: parts[2].trim().parse().ok(),
        speed: parts[3].trim().parse::<f64>().ok().map(|value| value as u64),
        eta: parts[4].trim().parse().ok(),
    })
}

pub fn download(tool: &Path, url: &str, folder: &Path, tx: Sender<VideoEvent>) {
    let outcome = (|| -> Result<(), String> {
        std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        let mut command = Command::new(tool);
        command.args([
            "--no-config", "--no-playlist", "--no-simulate", "--newline", "--progress",
            "--progress-template",
            "download:BOOKLAND_PROGRESS %(progress._percent_str)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.speed)s|%(progress.eta)s",
            "--print", "after_move:filepath",
            "--merge-output-format", "mp4",
            "-P", folder.to_str().ok_or("Invalid Tray path")?,
            "-o", "%(title).100B [%(id)s].%(ext)s",
        ]);
        if let Some(home) = std::env::var_os("HOME") {
            let ffmpeg = PathBuf::from(home).join(".local/bin/ffmpeg");
            if ffmpeg.is_file() { command.arg("--ffmpeg-location").arg(ffmpeg); }
        }
        let mut child = command.arg(url).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().map_err(|error| error.to_string())?;
        let _ = tx.send(VideoEvent::Started(child.id()));
        let stderr = child.stderr.take().ok_or("No error stream")?;
        let errors = std::thread::spawn(move || {
            let mut recent = VecDeque::new();
            for line in BufReader::new(stderr).lines().flatten() {
                recent.push_back(line);
                if recent.len() > 100 { recent.pop_front(); }
            }
            recent.into_iter().collect::<Vec<_>>().join("\n")
        });
        let output = child.stdout.take().ok_or("No progress stream")?;
        let mut saved = false;
        for line in BufReader::new(output).lines().flatten() {
            if let Some(progress) = parse_progress(&line) {
                let _ = tx.send(VideoEvent::Progress(progress));
            } else {
                let candidate = PathBuf::from(line.trim());
                if candidate.is_file() && candidate.starts_with(folder) {
                    saved = true;
                    let _ = tx.send(VideoEvent::Saved(candidate));
                }
            }
        }
        let status = child.wait().map_err(|error| error.to_string())?;
        let errors = errors.join().unwrap_or_default();
        if status.success() && saved { Ok(()) }
        else if status.success() { Err("Download ended without a saved file".into()) }
        else { Err(errors.lines().rev().find(|line| line.contains("ERROR"))
            .or_else(|| errors.lines().last()).unwrap_or("Download failed").to_owned()) }
    })();
    let _ = tx.send(VideoEvent::Finished(outcome));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonicalizes_only_video_links() {
        assert_eq!(youtube_url("https://youtu.be/abcdefghijk?si=tracking"),
            Some("https://www.youtube.com/watch?v=abcdefghijk".into()));
        assert_eq!(youtube_url("https://www.youtube.com/shorts/abcdefghijk"),
            Some("https://www.youtube.com/watch?v=abcdefghijk".into()));
        assert!(youtube_url("https://youtube.com/playlist?list=test").is_none());
        assert!(youtube_url("https://youtube.com.evil.test/watch?v=abcdefghijk").is_none());
    }
    #[test]
    fn parses_structured_progress() {
        let p = parse_progress("BOOKLAND_PROGRESS  25.0%|250|1000|500.2|3").unwrap();
        assert_eq!(p.percent, Some(25.0));
        assert_eq!(p.total, Some(1000));
        assert_eq!(p.speed, Some(500));
        assert_eq!(p.eta, Some(3));
    }
}
