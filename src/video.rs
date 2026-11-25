use std::{
    collections::VecDeque,
    fs,
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

fn publish_completed(source: &Path, folder: &Path) -> Result<PathBuf, String> {
    let name = source.file_name().ok_or("Download has no file name")?;
    let stem = source.file_stem().ok_or("Download has no file name")?
        .to_string_lossy();
    let extension = source.extension().map(|part| part.to_string_lossy());
    for number in 0..1000 {
        let destination = if number == 0 { folder.join(name) } else {
            let suffix = extension.as_ref().map(|part| format!(".{part}"))
                .unwrap_or_default();
            folder.join(format!("{stem} ({number}){suffix}"))
        };
        match fs::hard_link(source, &destination) {
            Ok(()) => return Ok(destination),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Could not save the completed video: {error}")),
        }
    }
    Err("Too many videos have the same name".into())
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
        fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        let staging = tempfile::Builder::new().prefix(".bookland-video-")
            .tempdir_in(folder).map_err(|error| error.to_string())?;
        let mut command = Command::new(tool);
        command.args([
            "--no-config", "--no-playlist", "--no-simulate", "--newline", "--progress",
            "--progress-template",
            "download:BOOKLAND_PROGRESS %(progress._percent_str)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.speed)s|%(progress.eta)s",
            "--print", "after_move:filepath",
            "--merge-output-format", "mp4",
            "-P", staging.path().to_str().ok_or("Invalid Tray path")?,
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
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                recent.push_back(line);
                if recent.len() > 100 { recent.pop_front(); }
            }
            recent.into_iter().collect::<Vec<_>>().join("\n")
        });
        let output = child.stdout.take().ok_or("No progress stream")?;
        let mut completed = None;
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            if let Some(progress) = parse_progress(&line) {
                let _ = tx.send(VideoEvent::Progress(progress));
            } else {
                let candidate = PathBuf::from(line.trim());
                if candidate.is_file() && candidate.starts_with(staging.path()) {
                    completed = Some(candidate);
                }
            }
        }
        let status = child.wait().map_err(|error| error.to_string())?;
        let errors = errors.join().unwrap_or_default();
        if status.success() {
            let source = completed.ok_or("Download ended without a saved file")?;
            let source = source.canonicalize().map_err(|error| error.to_string())?;
            let staging_root = staging.path().canonicalize()
                .map_err(|error| error.to_string())?;
            if !source.starts_with(staging_root) {
                return Err("Download output escaped its staging folder".into());
            }
            let path = publish_completed(&source, folder)?;
            let _ = tx.send(VideoEvent::Saved(path));
            Ok(())
        } else { Err(errors.lines().rev().find(|line| line.contains("ERROR"))
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
    #[test]
    fn publishing_keeps_an_existing_video() {
        let folder = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir_in(folder.path()).unwrap();
        let source = staging.path().join("solar.mp4");
        fs::write(&source, b"new video").unwrap();
        fs::write(folder.path().join("solar.mp4"), b"existing video").unwrap();
        let published = publish_completed(&source, folder.path()).unwrap();
        assert_eq!(published.file_name().unwrap(), "solar (1).mp4");
        assert_eq!(fs::read(folder.path().join("solar.mp4")).unwrap(), b"existing video");
        assert_eq!(fs::read(published).unwrap(), b"new video");
    }
    #[cfg(unix)]
    #[test]
    fn cancelled_download_removes_partial_file() {
        use std::{os::unix::fs::PermissionsExt, sync::mpsc, time::Duration};
        let root = tempfile::tempdir().unwrap();
        let tool = root.path().join("fake-downloader");
        fs::write(&tool, "#!/bin/sh\nset -eu\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = '-P' ]; then\n    shift\n    printf incomplete > \"$1/test.part\"\n    break\n  fi\n  shift\ndone\nexec sleep 30\n").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let folder = root.path().join("Tray");
        let (tx, rx) = mpsc::channel();
        let work = {
            let tool = tool.clone();
            let folder = folder.clone();
            std::thread::spawn(move || download(&tool, "test-url", &folder, tx))
        };
        let pid = match rx.recv_timeout(Duration::from_secs(3)).unwrap() {
            VideoEvent::Started(pid) => pid,
            _ => panic!("downloader did not start"),
        };
        let mut partial_found = false;
        for _ in 0..100 {
            partial_found = fs::read_dir(&folder).unwrap().filter_map(Result::ok)
                .any(|entry| entry.path().join("test.part").is_file());
            if partial_found { break; }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(partial_found, "fake downloader did not write its partial file");
        assert!(Command::new("kill").arg("-TERM").arg(pid.to_string())
            .status().unwrap().success());
        work.join().unwrap();
        assert!(rx.try_iter().any(|event| matches!(event, VideoEvent::Finished(Err(_)))));
        assert!(fs::read_dir(folder).unwrap().next().is_none(),
            "staging folder remained after cancellation");
    }
}
