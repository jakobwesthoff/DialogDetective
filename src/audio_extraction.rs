//! Audio extraction module
//!
//! This module provides functionality to extract audio from video files
//! using ffmpeg.

use crate::file_resolver::VideoFile;
use crate::temp::{TempError, TempGuard, create_temp_file};
use ffmpeg_sidecar::command::{FfmpegCommand, ffmpeg_is_installed};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during audio extraction
#[derive(Debug, Error)]
pub enum AudioExtractionError {
    /// FFmpeg is not installed
    #[error(
        "FFmpeg is not installed. Please install FFmpeg and ensure it's in your PATH, or place it in the same directory as this executable."
    )]
    FfmpegNotInstalled,

    /// Invalid video file path
    #[error("Invalid video file path: {0}")]
    InvalidVideoPath(PathBuf),

    /// Invalid temporary file path
    #[error("Invalid temporary file path")]
    InvalidTempPath,

    /// Failed to spawn FFmpeg process
    #[error("Failed to spawn FFmpeg process: {0}")]
    FfmpegSpawnFailed(String),

    /// FFmpeg execution failed
    #[error("FFmpeg execution failed: {0}")]
    FfmpegExecutionFailed(String),

    /// Failed to create temporary file
    #[error("Failed to create temporary file: {0}")]
    TempFileError(#[from] TempError),
}

/// Represents an extracted audio file
///
/// This struct wraps a temporary file containing the extracted audio in WAV format
/// (16kHz, mono, 16-bit PCM), ready for speech-to-text processing with whisper.
/// The audio file is automatically cleaned up when the `AudioFile` is dropped.
#[derive(Debug)]
pub(crate) struct AudioFile {
    /// Temporary file containing the extracted audio
    temp_file: TempGuard,
}

impl AudioFile {
    /// Creates a new AudioFile wrapping a temporary file guard
    fn new(temp_file: TempGuard) -> Self {
        Self { temp_file }
    }
}

impl Deref for AudioFile {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.temp_file
    }
}

/// Extracts audio from a video file
///
/// This function analyzes the video file, extracts its audio track,
/// and saves it to a temporary file. The temporary file is automatically
/// cleaned up when the returned `AudioFile` is dropped.
///
/// # Arguments
///
/// * `video` - The video file to extract audio from
///
/// # Returns
///
/// An `AudioFile` containing the extracted audio, or an error if extraction fails.
///
/// # Examples
///
/// ```ignore
/// let video = VideoFile { path: PathBuf::from("video.mp4") };
/// let audio = audio_from_video(&video).unwrap();
/// // Use &*audio to access the Path
/// // Audio file is automatically deleted when audio goes out of scope
/// ```
pub(crate) fn audio_from_video(video: &VideoFile) -> Result<AudioFile, AudioExtractionError> {
    // Check if ffmpeg is installed
    if !ffmpeg_is_installed() {
        return Err(AudioExtractionError::FfmpegNotInstalled);
    }

    // Create temporary file for audio output (WAV format for whisper-rs)
    let temp_audio = create_temp_file("audio_extract", "wav")?;

    // Extract audio from video using ffmpeg in whisper-compatible format
    // -i: input file
    // -vn: no video (audio only)
    // -ar 16000: 16kHz sample rate (required by whisper)
    // -ac 1: mono audio (single channel, required by whisper)
    // -c:a pcm_s16le: 16-bit PCM little-endian WAV (required by whisper)
    // -y: overwrite output file without asking
    FfmpegCommand::new()
        .input(
            video
                .path
                .to_str()
                .ok_or_else(|| AudioExtractionError::InvalidVideoPath(video.path.clone()))?,
        )
        .args(["-vn"]) // No video
        .args(["-ar", "16000"]) // 16kHz sample rate
        .args(["-ac", "1"]) // Mono (1 channel)
        .args(["-c:a", "pcm_s16le"]) // 16-bit PCM WAV
        .args(["-y"]) // Overwrite without asking
        .output(
            temp_audio
                .path()
                .to_str()
                .ok_or_else(|| AudioExtractionError::InvalidTempPath)?,
        )
        .spawn()
        .map_err(|e| AudioExtractionError::FfmpegSpawnFailed(e.to_string()))?
        .iter()
        .map_err(|e| AudioExtractionError::FfmpegExecutionFailed(e.to_string()))?
        .for_each(|_event| {
            // Iterate through events until completion
            // We could log progress here if needed
        });

    // Return AudioFile wrapping the temp file
    Ok(AudioFile::new(temp_audio))
}
