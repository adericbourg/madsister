//! The app's files (spec F-ED-10): songs written without risking the previous version.

use std::io::Write;
use std::path::Path;

/// Writes `contents` to a temporary file next to `path`, then renames it over `path`: a crash or a full disk midway
/// leaves the previous file whole.
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let name = path.file_name().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a file path"))?;
    let temp = path.with_file_name(format!(".{}.tmp", name.to_string_lossy()));
    let written = std::fs::File::create(&temp)
        .and_then(|mut file| file.write_all(contents.as_bytes()).and_then(|()| file.sync_all()))
        .and_then(|()| std::fs::rename(&temp, path));
    if written.is_err() {
        _ = std::fs::remove_file(&temp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_replaces_the_file_and_leaves_no_temporary_file() {
        // Given a file with old contents
        let dir = std::env::temp_dir().join(format!("madsister-write-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("song.madsister.json");
        std::fs::write(&path, "old").unwrap();

        // When writing over it
        write_atomic(&path, "new").unwrap();

        // Then it has the new contents, alone in its folder
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        let contents = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(contents, "new");
        assert_eq!(names, ["song.madsister.json"]);
    }

    #[test]
    fn write_atomic_into_a_missing_folder_fails_without_writing() {
        // Given / When
        let path = std::env::temp_dir().join(format!("madsister-missing-{}", std::process::id())).join("song.json");

        // Then
        assert!(write_atomic(&path, "new").is_err());
        assert!(!path.exists());
    }
}
