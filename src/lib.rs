//! DialogDetective - Automatically identify and rename unknown video files
//!
//! This library provides the core functionality for investigating video files,
//! analyzing their audio content, and solving the mystery of their true identity.

mod audio_extraction;
mod file_resolver;
mod temp;

use audio_extraction::audio_from_video;
use file_resolver::scan_for_videos;

// Re-export error types
pub use audio_extraction::AudioExtractionError;
pub use file_resolver::FileResolverError;
use std::fs;
use std::io;
use std::path::Path;
use thiserror::Error;

/// Top-level error type for DialogDetective operations
#[derive(Debug, Error)]
pub enum DialogDetectiveError {
    /// Error during file resolution
    #[error("File resolution error: {0}")]
    FileResolver(#[from] FileResolverError),

    /// Error during audio extraction
    #[error("Audio extraction error: {0}")]
    AudioExtraction(#[from] AudioExtractionError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Investigates a directory for video files and extracts their audio
///
/// This function scans the given directory recursively for video files,
/// extracts audio from each video, and saves the audio files beside the
/// original video files.
///
/// # Arguments
///
/// * `directory` - The directory path to investigate
///
/// # Returns
///
/// A Result indicating success or failure
///
/// # Examples
///
/// ```no_run
/// use dialog_detective::investigate_case;
/// use std::path::Path;
///
/// investigate_case(Path::new("/path/to/videos")).unwrap();
/// ```
pub fn investigate_case(directory: &Path) -> Result<(), DialogDetectiveError> {
    println!("DialogDetective reporting: Starting investigation in {}...", directory.display());

    // Scan directory for video files
    println!("\nScanning for video files...");
    let videos = scan_for_videos(directory)?;

    if videos.is_empty() {
        println!("No video files found.");
        return Ok(());
    }

    println!("Found {} video file(s)\n", videos.len());

    // Extract audio from each video file
    for (index, video) in videos.iter().enumerate() {
        println!("[{}/{}] Processing: {}", index + 1, videos.len(), video.path.display());

        // Extract audio
        println!("  Extracting audio...");
        let audio = audio_from_video(video)?;

        // Determine output path (beside the video file)
        let output_path = video.path.with_extension("wav");

        // Copy audio file to the target location
        println!("  Copying to: {}", output_path.display());
        fs::copy(&*audio, &output_path)?;

        println!("  ✓ Audio extracted: {}\n", output_path.display());
    }

    println!("Investigation complete! Processed {} video(s).", videos.len());

    Ok(())
}
