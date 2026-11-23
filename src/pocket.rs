use std::{fs, path::{Path, PathBuf}, process::Command};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { Keep, Png, Jpeg, Heic, Tiff, Aiff, Alac, Zip }

pub fn is_kept(source: &Path, root: &Path) -> bool {
    source.parent().and_then(|parent| parent.canonicalize().ok())
        == root.canonicalize().ok()
}

pub fn can_convert(source: &Path) -> bool {
    source.is_file() && source.extension().and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "heic" | "heif" | "tif" | "tiff" |
            "webp" | "bmp" | "gif" | "avif" | "wav" | "aif" | "aiff" |
            "m4a" | "mp3" | "caf" | "flac"))
}

pub fn can_convert_to(source: &Path, action: Action) -> bool {
    if !can_convert(source) { return false; }
    let extension = source.extension().and_then(|value| value.to_str())
        .unwrap_or_default().to_ascii_lowercase();
    let source_is_audio = matches!(extension.as_str(),
        "wav" | "aif" | "aiff" | "m4a" | "mp3" | "caf" | "flac");
    match action {
        Action::Aiff => source_is_audio && !matches!(extension.as_str(), "aif" | "aiff"),
        Action::Alac => source_is_audio,
        Action::Png | Action::Jpeg | Action::Heic | Action::Tiff => {
            !source_is_audio && !matches!((action, extension.as_str()),
                (Action::Png, "png") |
                (Action::Jpeg, "jpg" | "jpeg") |
                (Action::Heic, "heic" | "heif") |
                (Action::Tiff, "tif" | "tiff"))
        }
        Action::Keep | Action::Zip => false,
    }
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

fn archive_destination(root: &Path, source: &Path) -> Result<PathBuf, String> {
    let name = source.file_name().ok_or("File has no name")?.to_string_lossy();
    for index in 0..10_000 {
        let suffix = if index == 0 { String::new() } else { format!(" {index}") };
        let candidate = root.join(format!("{name}{suffix}.zip"));
        if !candidate.exists() { return Ok(candidate); }
    }
    Err("Pocket has too many ZIPs with this name".into())
}

pub fn execute(action: Action, source: &Path, root: &Path) -> Result<PathBuf, String> {
    if !source.exists() { return Err("Source file is no longer available".into()); }
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    if action == Action::Keep && is_kept(source, root) {
        return Ok(source.to_path_buf());
    }
    let output = if action == Action::Zip {
        archive_destination(root, source)?
    } else {
        let extension = match action {
            Action::Keep => None, Action::Png => Some("png"),
            Action::Jpeg => Some("jpg"), Action::Heic => Some("heic"),
            Action::Tiff => Some("tiff"), Action::Aiff => Some("aiff"),
            Action::Alac => Some("m4a"), Action::Zip => unreachable!(),
        };
        destination(root, source, extension)?
    };
    let result = match action {
        Action::Keep => Command::new("/usr/bin/ditto").arg(source).arg(&output).output(),
        Action::Zip => {
            let mut command = Command::new("/usr/bin/ditto");
            command.args(["-c", "-k", "--sequesterRsrc"]);
            if source.is_dir() { command.arg("--keepParent"); }
            command.current_dir(source.parent().filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new(".")))
                .arg(source.file_name().ok_or("File has no name")?)
                .arg(&output).output()
        }
        Action::Png | Action::Jpeg | Action::Heic | Action::Tiff => {
            if !source.is_file() { return Err("Only image files can be converted".into()); }
            let format = match action {
                Action::Png => "png", Action::Jpeg => "jpeg",
                Action::Heic => "heic", Action::Tiff => "tiff",
                _ => unreachable!(),
            };
            Command::new("/usr/bin/sips").args(["-s", "format", format])
                .arg(source).arg("--out").arg(&output).output()
        }
        Action::Aiff | Action::Alac => {
            if !can_convert_to(source, action) {
                return Err("This audio file cannot be converted to that format".into());
            }
            let (file, data) = if action == Action::Aiff {
                ("AIFF", "BEI16")
            } else { ("m4af", "alac") };
            Command::new("/usr/bin/afconvert")
                .args(["-f", file, "-d", data])
                .arg(source).arg(&output).output()
        }
    }.map_err(|error| error.to_string())?;
    if !result.status.success() || !output.exists() {
        let _ = fs::remove_file(&output);
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

    #[test]
    fn image_conversion_supports_heic_and_tiff() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sample.png");
        let pocket = temp.path().join("Pocket");
        image::RgbImage::from_pixel(32, 32, image::Rgb([10, 20, 30]))
            .save(&source).unwrap();
        for (action, extension) in [(Action::Heic, "heic"), (Action::Tiff, "tiff")] {
            let output = execute(action, &source, &pocket).unwrap();
            assert_eq!(output.extension().and_then(|value| value.to_str()), Some(extension));
            let result = Command::new("/usr/bin/sips")
                .arg("-g").arg("format").arg(&output).output().unwrap();
            assert!(result.status.success());
            assert!(String::from_utf8_lossy(&result.stdout).contains(extension));
        }
    }

    #[test]
    fn audio_conversion_writes_aiff_and_alac() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sample.wav");
        let pocket = temp.path().join("Pocket");
        let samples = vec![0_u8; 8_820];
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + samples.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&44_100_u32.to_le_bytes());
        wav.extend_from_slice(&88_200_u32.to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        wav.extend_from_slice(&samples);
        fs::write(&source, wav).unwrap();
        for (action, extension, codec) in [(Action::Aiff, "aiff", "AIFF"),
            (Action::Alac, "m4a", "alac")] {
            assert!(can_convert_to(&source, action));
            let output = execute(action, &source, &pocket).unwrap();
            assert_eq!(output.extension().and_then(|value| value.to_str()), Some(extension));
            let info = Command::new("/usr/bin/afinfo").arg(&output).output().unwrap();
            assert!(info.status.success());
            assert!(String::from_utf8_lossy(&info.stdout).contains(codec));
        }
        assert!(!can_convert_to(&source, Action::Heic));
    }

    #[test]
    fn zip_preserves_files_and_folders_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("project");
        let pocket = temp.path().join("Pocket");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("note.txt"), b"Bookland archive").unwrap();
        let first = execute(Action::Zip, &source, &pocket).unwrap();
        let second = execute(Action::Zip, &source, &pocket).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.file_name().unwrap(), "project.zip");
        let extracted = temp.path().join("Extracted");
        fs::create_dir(&extracted).unwrap();
        let result = Command::new("/usr/bin/ditto")
            .args(["-x", "-k"]).arg(&first).arg(&extracted).status().unwrap();
        assert!(result.success());
        assert_eq!(fs::read(extracted.join("project/note.txt")).unwrap(),
            b"Bookland archive");

        let loose_file = temp.path().join("loose.txt");
        fs::write(&loose_file, b"single file").unwrap();
        let file_zip = execute(Action::Zip, &loose_file, &pocket).unwrap();
        let file_result = Command::new("/usr/bin/ditto")
            .args(["-x", "-k"]).arg(&file_zip).arg(&extracted).status().unwrap();
        assert!(file_result.success());
        assert_eq!(fs::read(extracted.join("loose.txt")).unwrap(), b"single file");
    }
}
