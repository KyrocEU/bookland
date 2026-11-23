use std::{fs, path::{Path, PathBuf}, process::Command};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { Keep, Png, Jpeg }

pub fn is_kept(source: &Path, root: &Path) -> bool {
    source.parent().and_then(|parent| parent.canonicalize().ok())
        == root.canonicalize().ok()
}

pub fn can_convert(source: &Path) -> bool {
    source.is_file() && source.extension().and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "heic" | "heif" | "tif" | "tiff"))
}

fn destination(root: &Path, source: &Path, extension: Option<&str>) -> Result<PathBuf, String> {
    let name = source.file_name().ok_or("File has no name")?.to_string_lossy();
    let stem = source.file_stem().ok_or("File has no name")?.to_string_lossy();
    let source_extension = source.extension().and_then(|value| value.to_str());
    for index in 0..10_000 {
        let suffix = if index == 0 { String::new() } else { format!(" {index}") };
        let candidate = if let Some(extension) = extension {
            root.join(format!("{stem}{suffix}.{extension}"))
        } else if index == 0 {
            root.join(name.as_ref())
        } else if let Some(extension) = source_extension {
            root.join(format!("{stem}{suffix}.{extension}"))
        } else {
            root.join(format!("{name}{suffix}"))
        };
        if !candidate.exists() { return Ok(candidate); }
    }
    Err("Pocket has too many files with this name".into())
}

pub fn execute(action: Action, source: &Path, root: &Path) -> Result<PathBuf, String> {
    if !source.exists() { return Err("Source file is no longer available".into()); }
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    if action == Action::Keep && is_kept(source, root) {
        return Ok(source.to_path_buf());
    }
    let extension = match action { Action::Keep => None, Action::Png => Some("png"), Action::Jpeg => Some("jpg") };
    let output = destination(root, source, extension)?;
    let result = match action {
        Action::Keep => Command::new("/usr/bin/ditto").arg(source).arg(&output).output(),
        Action::Png | Action::Jpeg => {
            if !source.is_file() { return Err("Only image files can be converted".into()); }
            let format = if action == Action::Png { "png" } else { "jpeg" };
            Command::new("/usr/bin/sips").args(["-s", "format", format])
                .arg(source).arg("--out").arg(&output).output()
        }
    }.map_err(|error| error.to_string())?;
    if !result.status.success() || !output.exists() {
        let detail = String::from_utf8_lossy(&result.stderr);
        return Err(detail.lines().rev().find(|line| !line.trim().is_empty())
            .unwrap_or("File operation failed").chars().take(180).collect());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeping_creates_an_independent_copy_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sample.txt");
        let pocket = temp.path().join("Pocket");
        fs::write(&source, b"original").unwrap();
        let first = execute(Action::Keep, &source, &pocket).unwrap();
        let second = execute(Action::Keep, &source, &pocket).unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read(&first).unwrap(), b"original");
        fs::write(&source, b"changed").unwrap();
        assert_eq!(fs::read(&first).unwrap(), b"original");
    }

    #[test]
    fn image_conversion_writes_a_new_format() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sample.png");
        let pocket = temp.path().join("Pocket");
        image::RgbImage::from_pixel(2, 2, image::Rgb([10, 20, 30]))
            .save(&source).unwrap();
        let output = execute(Action::Jpeg, &source, &pocket).unwrap();
        assert_eq!(image::open(output).unwrap().width(), 2);
        assert!(source.exists());
    }
}
