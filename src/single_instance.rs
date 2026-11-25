#[cfg(target_os = "macos")]
use std::{fs::{self, File, OpenOptions}, io, os::fd::AsRawFd, path::Path};

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

#[cfg(target_os = "macos")]
fn acquire_at(path: &Path) -> io::Result<Option<File>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new().create(true).read(true).write(true).open(path)?;
    // BSD flock releases the lock when this File is dropped, including after a crash.
    if unsafe { flock(file.as_raw_fd(), 2 | 4) } == 0 {
        Ok(Some(file))
    } else {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(error)
        }
    }
}

#[cfg(target_os = "macos")]
pub fn acquire() -> io::Result<Option<File>> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is unavailable"))?;
    acquire_at(&Path::new(&home)
        .join("Library/Application Support/Bookland/instance.lock"))
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::acquire_at;

    #[test]
    fn one_process_owns_the_lock_until_it_exits() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("instance.lock");
        let first = acquire_at(&path).unwrap().unwrap();
        assert!(acquire_at(&path).unwrap().is_none());
        drop(first);
        assert!(acquire_at(&path).unwrap().is_some());
    }
}
