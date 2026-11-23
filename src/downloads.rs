use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Clone)]
pub struct DownloadEntry {
    pub path: PathBuf,
    pub bytes: u64,
    pub active: bool,
    pub modified: SystemTime,
    pub bytes_per_second: Option<u64>,
}

fn active_download(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    [".crdownload", ".download", ".part", ".partial"]
        .iter().any(|suffix| name.ends_with(suffix))
}

fn payload_size(path: &Path, depth: usize) -> u64 {
    if depth == 0 { return 0; }
    let Ok(entries) = fs::read_dir(path) else { return 0; };
    entries.flatten().map(|entry| {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { return 0; };
        if meta.is_file() && path.extension().is_none_or(|ext| ext != "plist") {
            meta.len()
        } else if meta.is_dir() {
            payload_size(&path, depth - 1)
        } else { 0 }
    }).sum()
}

pub fn scan(root: &Path) -> Result<Vec<DownloadEntry>, String> {
    let entries = fs::read_dir(root).map_err(|error| error.to_string())?;
    let mut downloads = entries.flatten().filter_map(|entry| {
        let path = entry.path();
        let meta = entry.metadata().ok()?;
        let active = active_download(&path);
        if !meta.is_file() && !(active && meta.is_dir()) { return None; }
        let modified = meta.modified().ok()?;
        Some(DownloadEntry {
            bytes: if meta.is_dir() { payload_size(&path, 3) } else { meta.len() },
            path,
            active,
            modified,
            bytes_per_second: None,
        })
    }).collect::<Vec<_>>();
    downloads.sort_by(|a, b| b.active.cmp(&a.active).then(b.modified.cmp(&a.modified)));
    downloads.truncate(12);
    Ok(downloads)
}

pub fn update_rates(previous: &[DownloadEntry], current: &mut [DownloadEntry], elapsed: Duration)
    -> Vec<String> {
    let seconds = elapsed.as_secs_f64().max(0.1);
    for item in current.iter_mut().filter(|item| item.active) {
        if let Some(old) = previous.iter().find(|old| old.path == item.path) {
            if item.bytes > old.bytes {
                item.bytes_per_second = Some(((item.bytes - old.bytes) as f64 / seconds) as u64);
            }
        }
    }
    previous.iter().filter(|old| old.active).filter_map(|old| {
        let stem = old.path.file_name()?.to_string_lossy();
        let lower = stem.to_lowercase();
        let suffix = [".crdownload", ".download", ".part", ".partial"]
            .into_iter().find(|suffix| lower.ends_with(suffix))?;
        let final_name = &stem[..stem.len() - suffix.len()];
        current.iter().find(|new| !new.active &&
            new.path.file_name().is_some_and(|name| name == final_name) &&
            new.modified >= old.modified)
            .map(|_| final_name.to_owned())
    }).collect()
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 { format!("{:.1} GB", bytes as f64 / 1_000_000_000.0) }
    else if bytes >= 1_000_000 { format!("{:.1} MB", bytes as f64 / 1_000_000.0) }
    else if bytes >= 1_000 { format!("{:.1} KB", bytes as f64 / 1_000.0) }
    else { format!("{bytes} B") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracks_growth_and_completion() {
        let root = tempfile::tempdir().unwrap();
        let partial = root.path().join("report.csv.crdownload");
        fs::write(&partial, vec![0_u8; 1000]).unwrap();
        let first = scan(root.path()).unwrap();
        assert!(first[0].active);
        fs::write(&partial, vec![0_u8; 3000]).unwrap();
        let mut second = scan(root.path()).unwrap();
        assert!(update_rates(&first, &mut second, Duration::from_secs(2)).is_empty());
        assert_eq!(second[0].bytes_per_second, Some(1000));
        fs::rename(&partial, root.path().join("report.csv")).unwrap();
        let mut third = scan(root.path()).unwrap();
        assert_eq!(update_rates(&second, &mut third, Duration::from_secs(2)), ["report.csv"]);
        assert!(!third[0].active);
    }
}
