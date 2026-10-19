//! The app's files (spec F-ED-10): songs written without risking the previous version, and the recent files list
//! (`recent.json` in the config folder, as the Tauri app kept it).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// `path` first, without duplicates, at most 10.
pub fn push_recent(list: &[String], path: &str) -> Vec<String> {
    std::iter::once(path).chain(list.iter().map(String::as_str).filter(|p| *p != path)).take(10).map(String::from).collect()
}

/// The strings of a JSON array, as `recent.json` holds them; nothing when it's anything else.
pub fn parse_paths(json: &Value) -> Vec<String> {
    json.as_array().into_iter().flatten().filter_map(Value::as_str).map(String::from).collect()
}

/// The shortest trailing part of each path (`blues.json`, then `a/blues.json`...) that tells it apart from the others.
pub fn shortest_unique_paths(paths: &[String]) -> Vec<String> {
    let segments: Vec<Vec<&str>> = paths.iter().map(|p| p.split(['/', '\\']).collect()).collect();
    let tail = |parts: &[&str], depth: usize| parts[parts.len() - depth.min(parts.len())..].join("/");
    segments
        .iter()
        .enumerate()
        .map(|(i, parts)| {
            let mut depth = 1;
            while depth < parts.len()
                && segments.iter().enumerate().any(|(j, other)| j != i && tail(other, depth) == tail(parts, depth))
            {
                depth += 1;
            }
            tail(parts, depth)
        })
        .collect()
}

/// Writes `contents` to a temporary file next to `path`, then renames it over `path`: a crash or a full disk midway
/// leaves the previous file whole. The new file keeps the previous one's permissions.
pub fn write_atomic(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let name = path.file_name().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a file path"))?;
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let temp_names = (0..100).map(|_| {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
        path.with_file_name(format!(".{}.{}-{n}-{nanos}.tmp", name.to_string_lossy(), std::process::id()))
    });
    write_through(path, contents.as_ref(), temp_names)
}

/// `write_atomic` through the first of `temp_names` that doesn't exist yet: created new, so an existing file or
/// symlink planted at that name is never written through.
fn write_through(path: &Path, contents: &[u8], temp_names: impl Iterator<Item = PathBuf>) -> std::io::Result<()> {
    let mut last_error = std::io::Error::new(std::io::ErrorKind::AlreadyExists, "no free temporary file name");
    for temp in temp_names {
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = e;
                continue;
            }
            Err(e) => return Err(e),
        };
        let written = file
            .write_all(contents)
            .and_then(|()| match std::fs::metadata(path) {
                Ok(previous) => file.set_permissions(previous.permissions()),
                Err(_) => Ok(()),
            })
            .and_then(|()| file.sync_all())
            .and_then(|()| std::fs::rename(&temp, path));
        if written.is_err() {
            _ = std::fs::remove_file(&temp);
        }
        return written;
    }
    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_recent_moves_the_path_to_the_front_without_duplicates_and_keeps_ten() {
        // Given a full list of 10 paths
        let list: Vec<String> = (0..10).map(|i| format!("/{i}")).collect();

        // When pushing a path already in the list, then a new one
        let moved = push_recent(&list, "/5");
        let pushed = push_recent(&moved, "/new");

        // Then the path moves to the front, and the oldest one drops off
        assert_eq!(moved, ["/5", "/0", "/1", "/2", "/3", "/4", "/6", "/7", "/8", "/9"]);
        assert_eq!(pushed, ["/new", "/5", "/0", "/1", "/2", "/3", "/4", "/6", "/7", "/8"]);
    }

    #[test]
    fn shortest_unique_paths_keeps_just_the_file_name_unless_several_paths_share_it() {
        // Given paths, two of which share a file name (one under the same parent name as well)
        let paths = ["/a/blues.json", "/b/blues.json", "/x/c/rock.json", "/y/c/blues.json", "/solo.json"].map(String::from);

        // When / Then each one keeps only as many trailing parts as needed to be unique
        assert_eq!(shortest_unique_paths(&paths), ["a/blues.json", "b/blues.json", "rock.json", "c/blues.json", "solo.json"]);
    }

    #[test]
    fn shortest_unique_paths_handles_windows_separators_and_a_path_inside_another() {
        // Given / When / Then
        let paths = ["C:\\music\\blues.json", "blues.json"].map(String::from);
        assert_eq!(shortest_unique_paths(&paths), ["music/blues.json", "blues.json"]);
    }

    #[test]
    fn parse_paths_keeps_the_strings_of_an_array() {
        // Given / When / Then
        assert_eq!(parse_paths(&serde_json::json!(["/a.json", 1, "/b.json"])), ["/a.json", "/b.json"]);
        assert!(parse_paths(&serde_json::json!({ "paths": [] })).is_empty());
    }

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

    #[cfg(unix)]
    #[test]
    fn write_through_skips_a_symlink_planted_at_the_temporary_name_and_keeps_the_permissions() {
        // Given a read-only-for-others song file, and a symlink to another file where the first temporary name would be
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("madsister-write-symlink-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (path, victim) = (dir.join("song.madsister.json"), dir.join("victim"));
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(&victim, "untouched").unwrap();
        let (planted, free) = (dir.join(".planted.tmp"), dir.join(".free.tmp"));
        std::os::unix::fs::symlink(&victim, &planted).unwrap();

        // When writing through those names
        write_through(&path, b"new", [planted.clone(), free.clone()].into_iter()).unwrap();

        // Then the other file is untouched, the song has the new contents and its permissions, through the free name
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        let (contents, victim_contents) = (std::fs::read_to_string(&path).unwrap(), std::fs::read_to_string(&victim).unwrap());
        let (is_planted_kept, is_free_gone) = (planted.symlink_metadata().is_ok(), !free.exists());
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!((victim_contents.as_str(), contents.as_str(), mode), ("untouched", "new", 0o600));
        assert!(is_planted_kept && is_free_gone);
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
