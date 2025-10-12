//! DialogDetective - Automatically identify and rename unknown video files
//!
//! This library provides the core functionality for investigating video files,
//! analyzing their audio content, and solving the mystery of their true identity.

mod ai_matcher;
mod audio_extraction;
mod file_resolver;
mod metadata_retrieval;
mod speech_to_text;
mod temp;

use ai_matcher::{NaivePromptGenerator, SinglePromptGenerator};
use audio_extraction::audio_from_video;
use file_resolver::scan_for_videos;
use metadata_retrieval::{MetadataProvider, TvMazeProvider};
use speech_to_text::audio_to_text;

// Re-export error types
pub use audio_extraction::AudioExtractionError;
pub use file_resolver::FileResolverError;
pub use metadata_retrieval::MetadataRetrievalError;
pub use speech_to_text::SpeechToTextError;
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

    /// Error during speech-to-text transcription
    #[error("Speech-to-text error: {0}")]
    SpeechToText(#[from] SpeechToTextError),

    /// Error during metadata retrieval
    #[error("Metadata retrieval error: {0}")]
    MetadataRetrieval(#[from] MetadataRetrievalError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Investigates a directory for video files and transcribes their audio
///
/// This function scans the given directory recursively for video files,
/// extracts audio from each video, transcribes the audio to text using Whisper,
/// fetches episode metadata for the given show, and prints all information.
///
/// # Arguments
///
/// * `directory` - The directory path to investigate
/// * `model_path` - Path to the Whisper model file (e.g., ggml-base.bin)
/// * `show_name` - The name of the TV show to fetch metadata for
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
/// investigate_case(
///     Path::new("/path/to/videos"),
///     Path::new("models/ggml-base.bin"),
///     "Breaking Bad"
/// ).unwrap();
/// ```
pub fn investigate_case(
    directory: &Path,
    model_path: &Path,
    show_name: &str,
) -> Result<(), DialogDetectiveError> {
    println!(
        "DialogDetective reporting: Starting investigation in {} for {}...",
        directory.display(),
        show_name
    );

    // Fetch episode metadata
    println!("\n=== Fetching Episode Metadata ===");
    println!("Retrieving episode information for '{}'...", show_name);

    let provider = TvMazeProvider::new();
    let series = provider.fetch_series(show_name, None)?;

    println!(
        "Found {} season(s) for '{}'\n",
        series.seasons.len(),
        series.name
    );

    // Scan directory for video files
    println!("\nScanning for video files...");
    let videos = scan_for_videos(directory)?;

    if videos.is_empty() {
        println!("No video files found.");
        return Ok(());
    }

    println!("Found {} video file(s)\n", videos.len());

    // Store transcripts for each video
    let mut transcripts = Vec::new();

    // Process each video file
    for (index, video) in videos.iter().enumerate() {
        println!(
            "[{}/{}] Processing: {}",
            index + 1,
            videos.len(),
            video.path.display()
        );

        // Extract audio
        println!("  Extracting audio...");
        let audio = audio_from_video(video)?;

        // Transcribe audio to text
        println!("  Transcribing audio...");
        let transcript = audio_to_text(&audio, model_path)?;

        // Print transcript
        println!("  Language: {}", transcript.language);
        println!("  Transcript:\n{}\n", transcript.text);

        transcripts.push(transcript);
    }

    println!(
        "Investigation complete! Processed {} video(s).",
        videos.len()
    );

    // Generate prompts for each video
    let prompt_generator = NaivePromptGenerator::default();

    for (index, transcript) in transcripts.iter().enumerate() {
        println!(
            "=== Generated Prompt for Video {}/{}: {} ===\n",
            index + 1,
            videos.len(),
            videos[index].path.display()
        );

        let prompt = prompt_generator.generate_single_prompt(transcript, &series);
        println!("{}\n", prompt);
        println!("=== End of Prompt ===\n");
    }

    Ok(())
}
