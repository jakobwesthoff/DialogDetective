//! Temporary file management module
//!
//! This module provides RAII-based temporary file handling with automatic cleanup.

use std::fs::{self, File};
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// Guard for temporary resources that automatically cleans up on drop
#[derive(Debug)]
pub(crate) enum TempGuard {
    /// Temporary file that will be deleted when dropped
    File(PathBuf),
    // Future: Directory variant can be added here
    // Directory(PathBuf),
}

impl TempGuard {
    /// Get the path to the temporary resource
    pub(crate) fn path(&self) -> &Path {
        match self {
            TempGuard::File(path) => path,
        }
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        match self {
            TempGuard::File(path) => {
                // Silently ignore errors during cleanup
                let _ = fs::remove_file(path);
            }
        }
    }
}

impl Deref for TempGuard {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}

/// Creates a temporary file and returns a guard that will clean it up on drop
///
/// The file is created in the system's temporary directory with a unique name
/// generated using ULID (monotonic, sortable unique identifier).
/// When the returned `TempGuard` is dropped, the file is automatically deleted.
///
/// # Returns
///
/// A `TempGuard::File` variant containing the path to the created temporary file,
/// or an error if the file could not be created.
///
/// # Examples
///
/// ```ignore
/// let temp = create_temp_file("audio", "mp3").unwrap();
/// // Use temp.path() to access the file
/// // File is automatically deleted when temp goes out of scope
/// ```
pub(crate) fn create_temp_file(prefix: &str, extension: &str) -> io::Result<TempGuard> {
    let temp_dir = std::env::temp_dir();

    // Create a unique filename using ULID (monotonic and sortable)
    let ulid = ulid::Ulid::new();
    let filename = format!("{}_{}.{}", prefix, ulid, extension);

    let path = temp_dir.join(filename);

    // Create the file
    File::create(&path)?;

    Ok(TempGuard::File(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_file() {
        let temp = create_temp_file("test", "txt").unwrap();
        let path = temp.path().to_path_buf();

        // File should exist
        assert!(path.exists());
        assert!(path.is_file());

        // Filename should contain prefix and extension
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("test_"));
        assert!(filename.ends_with(".txt"));

        // Drop the guard
        drop(temp);

        // File should be cleaned up
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_guard_path() {
        let temp = create_temp_file("test", "dat").unwrap();
        let path = temp.path();

        assert!(path.exists());
        assert!(path.is_absolute());
    }

    #[test]
    fn test_multiple_temp_files_unique() {
        let temp1 = create_temp_file("test", "txt").unwrap();
        let temp2 = create_temp_file("test", "txt").unwrap();

        // Should have different paths
        assert_ne!(temp1.path(), temp2.path());

        // Both should exist
        assert!(temp1.path().exists());
        assert!(temp2.path().exists());
    }

    #[test]
    fn test_temp_file_cleanup_on_drop() {
        let path = {
            let temp = create_temp_file("cleanup_test", "tmp").unwrap();
            let path = temp.path().to_path_buf();
            assert!(path.exists());
            path
            // temp is dropped here
        };

        // File should be gone after guard is dropped
        assert!(!path.exists());
    }
}
