use std::{fs, io::{Read, Write}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::pocket::{self, Action};

const FORMAT: &str = "bookland.pipeline";
const MAX_PACKAGE_BYTES: u64 = 128 * 1024;
const MAX_MANIFEST_BYTES: u64 = 32 * 1024;

pub const CONVERSIONS: [(Action, &str); 10] = [
    (Action::Png, "PNG"), (Action::Jpeg, "JPEG"),
    (Action::Heic, "HEIC"), (Action::Tiff, "TIFF"),
    (Action::Webp, "WebP"), (Action::Aiff, "AIFF"),
    (Action::Alac, "ALAC"), (Action::Flac, "FLAC"),
    (Action::Docx, "DOCX"), (Action::Html, "HTML"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<Action>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    format: String,
    version: u32,
    pipeline: Pipeline,
}

fn family(action: Action) -> Option<u8> {
    match action {
        Action::Png | Action::Jpeg | Action::Heic | Action::Tiff | Action::Webp => Some(1),
        Action::Aiff | Action::Alac | Action::Flac => Some(2),
        Action::Docx | Action::Html => Some(3),
        _ => None,
    }
}

pub fn can_append(steps: &[Action], next: Action) -> bool {
    if steps.len() >= 6 || next == Action::Keep || steps.contains(&Action::Zip) { return false; }
    if next == Action::Zip { return !steps.is_empty(); }
    let Some(kind) = family(next) else { return false; };
    steps.last().is_none_or(|last| family(*last) == Some(kind) && *last != next)
}

impl Pipeline {
    pub fn validate(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 48 || name.chars().any(char::is_control) {
            return Err("Pipeline name must be 1–48 printable characters".into());
        }
        if self.steps.is_empty() || self.steps.len() > 6 {
            return Err("A pipeline needs 1–6 steps".into());
        }
        let mut previous = Vec::new();
        for &step in &self.steps {
            if !can_append(&previous, step) {
                return Err("Pipeline steps must use one file type and change format".into());
            }
            previous.push(step);
        }
        Ok(())
    }

    pub fn can_run(&self, source: &Path) -> bool {
        self.validate().is_ok() && self.steps.first()
            .is_some_and(|action| pocket::can_convert_to(source, *action))
    }

    pub fn summary(&self) -> String {
        self.steps.iter().map(|step| if *step == Action::Zip { "ZIP" }
            else { CONVERSIONS.iter().find(|(action, _)| action == step)
                .map(|(_, name)| *name).unwrap_or("?") })
            .collect::<Vec<_>>().join(" → ")
    }
}

pub fn run(pipeline: &Pipeline, source: &Path, root: &Path) -> Result<PathBuf, String> {
    pipeline.validate()?;
    if !pipeline.can_run(source) {
        return Err("This file cannot start that pipeline".into());
    }
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    // All intermediate products stay in a temporary directory inside Pocket.
    // Only a successful final product is copied into the user's saved files.
    let staging = tempfile::tempdir_in(root).map_err(|error| error.to_string())?;
    let mut current = source.to_path_buf();
    for &step in &pipeline.steps {
        if step != Action::Zip && !pocket::can_convert_to(&current, step) {
            return Err(format!("{} cannot follow the previous conversion", pipeline.summary()));
        }
        current = pocket::execute(step, &current, staging.path())?;
    }
    pocket::execute(Action::Keep, &current, root)
}

fn safe_filename(name: &str) -> String {
    let slug = name.trim().chars().filter(|c| c.is_alphanumeric() ||
        matches!(c, ' ' | '-' | '_')).take(48).collect::<String>();
    if slug.is_empty() { "Pipeline".into() } else { slug }
}

pub fn export(pipeline: &Pipeline, root: &Path) -> Result<PathBuf, String> {
    pipeline.validate()?;
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|error| error.to_string())?;
    let package = Package { format: FORMAT.into(), version: 1, pipeline: pipeline.clone() };
    let manifest = serde_json::to_vec_pretty(&package).map_err(|error| error.to_string())?;
    if manifest.len() as u64 > MAX_MANIFEST_BYTES { return Err("Pipeline is too large".into()); }
    {
        let mut writer = ZipWriter::new(temp.as_file_mut());
        writer.start_file("manifest.json", SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored))
            .map_err(|error| error.to_string())?;
        writer.write_all(&manifest).map_err(|error| error.to_string())?;
        writer.finish().map_err(|error| error.to_string())?;
    }
    let stem = safe_filename(&pipeline.name);
    for index in 0..1000 {
        let suffix = if index == 0 { String::new() } else { format!(" {index}") };
        let path = root.join(format!("{stem}{suffix}.booklandpipeline"));
        if path.exists() { continue; }
        return temp.persist_noclobber(&path).map(|_| path)
            .map_err(|error| error.error.to_string());
    }
    Err("Too many exported pipelines with this name".into())
}

pub fn import(path: &Path) -> Result<Pipeline, String> {
    if path.extension().and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("booklandpipeline")) {
        return Err("Choose a .booklandpipeline file".into());
    }
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_PACKAGE_BYTES {
        return Err("Pipeline package is too large".into());
    }
    let mut archive = ZipArchive::new(fs::File::open(path).map_err(|error| error.to_string())?)
        .map_err(|_| "This is not a Bookland pipeline package".to_string())?;
    if archive.len() != 1 || archive.has_duplicate_names() {
        return Err("Pipeline package must contain only its manifest".into());
    }
    let mut entry = archive.by_index(0).map_err(|error| error.to_string())?;
    if entry.name().map_err(|error| error.to_string())?.as_ref() != "manifest.json"
        || entry.is_dir() || entry.encrypted() || entry.size() > MAX_MANIFEST_BYTES {
        return Err("Pipeline manifest is invalid".into());
    }
    let mut bytes = Vec::new();
    entry.by_ref().take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("Pipeline manifest is too large".into());
    }
    let package: Package = serde_json::from_slice(&bytes)
        .map_err(|_| "Pipeline manifest cannot be read".to_string())?;
    if package.format != FORMAT || package.version != 1 {
        return Err("Unsupported pipeline package version".into());
    }
    package.pipeline.validate()?;
    Ok(package.pipeline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_round_trip_and_rejects_unknown_steps() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = Pipeline { name: "Photos to WebP".into(),
            steps: vec![Action::Jpeg, Action::Webp] };
        let path = export(&pipeline, temp.path()).unwrap();
        let imported = import(&path).unwrap();
        assert_eq!(imported.name, pipeline.name);
        assert_eq!(imported.steps, pipeline.steps);
        assert!(path.starts_with(temp.path()));
        assert_ne!(export(&pipeline, temp.path()).unwrap(), path);
        assert!(Pipeline { name: "bad".into(), steps: vec![Action::Keep] }.validate().is_err());
        assert!(Pipeline { name: "bad".into(), steps: vec![Action::Png, Action::Png] }
            .validate().is_err());
        assert!(Pipeline { name: "bad".into(), steps: vec![Action::Png, Action::Flac] }
            .validate().is_err());
        assert!(Pipeline { name: "bad".into(), steps: vec![Action::Zip] }.validate().is_err());
        assert!(Pipeline { name: "valid".into(),
            steps: vec![Action::Jpeg, Action::Zip] }.validate().is_ok());
        let extra = temp.path().join("extra.booklandpipeline");
        {
            let mut archive = ZipWriter::new(fs::File::create(&extra).unwrap());
            archive.start_file("manifest.json", SimpleFileOptions::default()).unwrap();
            archive.write_all(br#"{"format":"bookland.pipeline","version":1,"pipeline":{"name":"extra","steps":["webp"]}}"#).unwrap();
            archive.start_file("unexpected.txt", SimpleFileOptions::default()).unwrap();
            archive.write_all(b"never extracted").unwrap();
            archive.finish().unwrap();
        }
        assert!(import(&extra).is_err());
    }

    #[test]
    fn pipeline_runs_steps_and_saves_only_final_file() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("input.png");
        let pocket = temp.path().join("Pocket");
        image::RgbImage::from_pixel(16, 16, image::Rgb([5, 40, 180]))
            .save(&source).unwrap();
        let pipeline = Pipeline { name: "Photos to WebP".into(),
            steps: vec![Action::Jpeg, Action::Webp] };
        let first = run(&pipeline, &source, &pocket).unwrap();
        let second = run(&pipeline, &source, &pocket).unwrap();
        assert_ne!(first, second);
        assert_eq!(image::open(&first).unwrap().width(), 16);
        assert_eq!(fs::read_dir(&pocket).unwrap().count(), 2);
        assert!(source.exists());
        assert!(!pipeline.can_run(&temp.path().join("wrong.txt")));
        let zipped = Pipeline { name: "WebP ZIP".into(),
            steps: vec![Action::Webp, Action::Zip] };
        let archive = run(&zipped, &source, &pocket).unwrap();
        assert_eq!(archive.extension().and_then(|s| s.to_str()), Some("zip"));
        assert_eq!(fs::read_dir(&pocket).unwrap().count(), 3);
    }
}
