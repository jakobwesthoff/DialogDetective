//! File resolver module for investigating video files
//!
//! This module provides functionality to scan directories and identify video files
//! by analyzing their content using MIME type detection.

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during file resolution
#[derive(Debug, Error)]
pub enum FileResolverError {
    /// Path is not a directory
    #[error("Path is not a directory: {0}")]
    NotADirectory(PathBuf),

    /// Failed to read directory
    #[error("Failed to read directory {path}: {source}")]
    ReadDirectoryFailed { path: PathBuf, source: io::Error },

    /// Failed to read directory entry
    #[error("Failed to read directory entry: {0}")]
    ReadEntryFailed(#[from] io::Error),
}

/// Represents a detected video file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFile {
    /// Path to the video file
    pub path: PathBuf,
}

/// Investigates a directory recursively to find all video files
///
/// This function scans the given directory and all subdirectories,
/// analyzing each file to detect video files by their content (not extension).
///
/// # Arguments
///
/// * `dir_path` - The directory path to investigate
///
/// # Returns
///
/// A vector of `VideoFile` structs representing all discovered video files,
/// or an error if the directory cannot be read.
pub(crate) fn scan_for_videos(dir_path: &Path) -> Result<Vec<VideoFile>, FileResolverError> {
    let mut video_files = Vec::new();
    scan_directory_recursive(dir_path, &mut video_files)?;
    Ok(video_files)
}

/// Recursively scans a directory and collects video files
fn scan_directory_recursive(
    dir_path: &Path,
    video_files: &mut Vec<VideoFile>,
) -> Result<(), FileResolverError> {
    if !dir_path.is_dir() {
        return Err(FileResolverError::NotADirectory(dir_path.to_path_buf()));
    }

    for entry in fs::read_dir(dir_path).map_err(|e| FileResolverError::ReadDirectoryFailed {
        path: dir_path.to_path_buf(),
        source: e,
    })? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively investigate subdirectories
            scan_directory_recursive(&path, video_files)?;
        } else if path.is_file() {
            // Analyze file to determine if it's a video
            if is_video_file(&path) {
                video_files.push(VideoFile { path });
            }
        }
    }

    Ok(())
}

/// Analyzes a file to determine if it's a video file
///
/// Returns true if the file is a recognized video format, false otherwise.
/// Only reads the first 8KB of the file for efficiency.
fn is_video_file(file_path: &Path) -> bool {
    // Only read the first 8KB for file type detection
    const BUFFER_SIZE: usize = 8192;

    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let bytes_read = match file.read(&mut buffer) {
        Ok(n) => n,
        Err(_) => return false,
    };

    // Truncate buffer to actual bytes read
    buffer.truncate(bytes_read);

    infer::is_video(&buffer)
}

/// Computes SHA256 hash of a video file for use as a cache key
///
/// This function reads the entire video file in 512KB chunks and computes
/// a SHA256 hash. The hash is used to identify cached transcripts, ensuring
/// that identical files can reuse cached results even if renamed or moved.
///
/// # Arguments
///
/// * `video_path` - Path to the video file to hash
///
/// # Returns
///
/// A hex-encoded SHA256 hash string, or an error if the file cannot be read.
///
/// # Examples
///
/// ```ignore
/// let hash = compute_video_hash(Path::new("video.mp4"))?;
/// println!("Video hash: {}", hash);
/// ```
pub(crate) fn compute_video_hash(video_path: &Path) -> Result<String, FileResolverError> {
    const BUFFER_SIZE: usize = 512 * 1024; // 512KB chunks

    let mut file = File::open(video_path).map_err(|e| FileResolverError::ReadEntryFailed(e))?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(FileResolverError::ReadEntryFailed)?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    #[test]
    fn test_scan_nonexistent_directory() {
        let result = scan_for_videos(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_file_instead_of_directory() {
        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_file.txt");
        File::create(&temp_file).unwrap();

        let result = scan_for_videos(&temp_file);
        assert!(result.is_err());

        // Cleanup
        fs::remove_file(&temp_file).ok();
    }
}
