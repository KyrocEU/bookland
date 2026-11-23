use std::{fs, process::Command};
use tempfile::NamedTempFile;

const MUSIC_SCRIPT: &str = include_str!("music_artwork.applescript");
const MAX_ART_BYTES: usize = 5_000_000;

pub fn decode_thumbnail(bytes: &[u8]) -> Option<eframe::egui::ColorImage> {
    if bytes.is_empty() || bytes.len() > MAX_ART_BYTES { return None; }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format().ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader.decode().ok()?.thumbnail(256, 256).to_rgba8();
    Some(eframe::egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize], rgba.as_raw()))
}

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

#[cfg(test)]
mod tests {
    use super::decode_thumbnail;

    #[test]
    fn artwork_thumbnail_is_bounded_before_decode() {
        let small = decode_thumbnail(include_bytes!("../assets/brand/bookland-mark.png"))
            .expect("small brand image should decode");
        assert_eq!(small.size, [256, 256]);

        let huge = image::DynamicImage::ImageRgba8(image::RgbaImage::new(2049, 1));
        let mut encoded = std::io::Cursor::new(Vec::new());
        huge.write_to(&mut encoded, image::ImageFormat::Png).unwrap();
        assert!(decode_thumbnail(&encoded.into_inner()).is_none());
    }
}
