use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_IMAGES: usize = 8;
pub const MAX_BYTES: usize = 8_000_000;
const MAX_DIMENSION: usize = 4096;
const MAGIC: &[u8; 5] = b"BLIM\x01";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedImage {
    pub fingerprint: u64,
    pub width: usize,
    pub height: usize,
    pub favorite: bool,
}

pub fn valid_dimensions(width: usize, height: usize, length: usize) -> bool {
    width > 0 && height > 0 && width <= MAX_DIMENSION && height <= MAX_DIMENSION
        && width.checked_mul(height).and_then(|pixels| pixels.checked_mul(4)) == Some(length)
        && length <= MAX_BYTES
}

fn image_path(directory: &Path, fingerprint: u64) -> PathBuf {
    directory.join(format!("{fingerprint:016x}.rgba"))
}

fn header(image: &SavedImage) -> [u8; 13] {
    let mut bytes = [0u8; 13];
    bytes[..5].copy_from_slice(MAGIC);
    bytes[5..9].copy_from_slice(&(image.width as u32).to_le_bytes());
    bytes[9..13].copy_from_slice(&(image.height as u32).to_le_bytes());
    bytes
}

pub fn write_image(directory: &Path, image: &SavedImage, rgba: &[u8]) -> io::Result<()> {
    if !valid_dimensions(image.width, image.height, rgba.len()) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid clipboard image"));
    }
    fs::create_dir_all(directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    }
    let path = image_path(directory, image.fingerprint);
    let expected_header = header(image);
    if fs::metadata(&path).is_ok_and(|meta| meta.len() == (13 + rgba.len()) as u64) {
        let mut existing_header = [0u8; 13];
        if File::open(&path).and_then(|mut file| file.read_exact(&mut existing_header)).is_ok()
            && existing_header == expected_header {
            return Ok(());
        }
    }
    let temporary = path.with_extension("rgba.tmp");
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(&expected_header)?;
    file.write_all(rgba)?;
    file.flush()?;
    drop(file);
    fs::rename(temporary, path)
}

pub fn read_image(directory: &Path, image: &SavedImage) -> Option<Vec<u8>> {
    let length = image.width.checked_mul(image.height)?.checked_mul(4)?;
    if !valid_dimensions(image.width, image.height, length) { return None; }
    let mut file = File::open(image_path(directory, image.fingerprint)).ok()?;
    let mut header = [0u8; 13];
    file.read_exact(&mut header).ok()?;
    if header != self::header(image) { return None; }
    let mut rgba = vec![0; length];
    file.read_exact(&mut rgba).ok()?;
    let mut extra = [0];
    if file.read(&mut extra).ok()? != 0 { return None; }
    Some(rgba)
}

pub fn prune(directory: &Path, keep: &[SavedImage]) {
    let allowed: HashSet<_> = keep.iter()
        .map(|image| format!("{:016x}.rgba", image.fingerprint)).collect();
    let Ok(entries) = fs::read_dir(directory) else { return; };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let managed = name.strip_suffix(".rgba")
            .or_else(|| name.strip_suffix(".rgba.tmp"))
            .is_some_and(|stem| stem.len() == 16 && stem.bytes().all(|byte|
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        if managed && !allowed.contains(&name) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        std::env::temp_dir().join(format!("bookland-clipboard-images-{}-{}",
            std::process::id(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))
    }

    #[test]
    fn image_round_trip_and_prune_preserve_only_current_history() {
        let root = scratch();
        let first = SavedImage { fingerprint: 1, width: 2, height: 1, favorite: false };
        let second = SavedImage { fingerprint: 2, width: 1, height: 1, favorite: true };
        write_image(&root, &first, &[1; 8]).unwrap();
        write_image(&root, &second, &[2; 4]).unwrap();
        assert_eq!(read_image(&root, &first), Some(vec![1; 8]));
        fs::write(root.join("notes.txt"), "leave alone").unwrap();
        prune(&root, &[second.clone()]);
        assert!(read_image(&root, &first).is_none());
        assert_eq!(read_image(&root, &second), Some(vec![2; 4]));
        assert!(root.join("notes.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_dimensions_and_trailing_data_are_rejected() {
        let root = scratch();
        let invalid = SavedImage { fingerprint: 3, width: 4097, height: 1, favorite: false };
        assert!(write_image(&root, &invalid, &[0; 4]).is_err());
        let valid = SavedImage { fingerprint: 4, width: 1, height: 1, favorite: false };
        write_image(&root, &valid, &[1; 4]).unwrap();
        OpenOptions::new().append(true).open(image_path(&root, 4)).unwrap()
            .write_all(&[9]).unwrap();
        assert!(read_image(&root, &valid).is_none());
        let path = image_path(&root, 4);
        fs::write(&path, [0; 17]).unwrap();
        write_image(&root, &valid, &[1; 4]).unwrap();
        assert_eq!(read_image(&root, &valid), Some(vec![1; 4]));
        fs::remove_dir_all(root).unwrap();
    }
}
