//! DialogDetective - Automatically identify and rename unknown video files
//!
//! This library provides the core functionality for investigating video files,
//! analyzing their audio content, and solving the mystery of their true identity.

mod ai_matcher;
mod audio_extraction;
mod cache;
mod file_resolver;
mod metadata_retrieval;
mod speech_to_text;
mod temp;

use ai_matcher::{ClaudeCodeMatcher, EpisodeMatcher, NaivePromptGenerator};
use audio_extraction::audio_from_video;
use cache::CacheStorage;
use file_resolver::{scan_for_videos, VideoFile};
use metadata_retrieval::{CachedMetadataProvider, Episode, MetadataProvider, TVSeries, TvMazeProvider};
use speech_to_text::audio_to_text;
use std::time::Duration;

// Re-export error types
pub use ai_matcher::EpisodeMatchingError;
pub use audio_extraction::AudioExtractionError;
pub use cache::CacheError;
pub use file_resolver::FileResolverError;
pub use metadata_retrieval::MetadataRetrievalError;
pub use speech_to_text::SpeechToTextError;
use std::io;
use std::path::Path;
use thiserror::Error;

/// Represents the result of matching a video file to an episode
///
/// This structure contains the "evidence" that correlates a video file
/// with a specific episode from a TV series.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    /// The video file that was matched
    pub video: VideoFile,

    /// The episode that was matched
    pub episode: Episode,
}

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

    /// Error during cache operations
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    /// Error during episode matching
    #[error("Episode matching error: {0}")]
    EpisodeMatching(#[from] EpisodeMatchingError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Investigates a directory for video files and matches them to episodes
///
/// This function scans the given directory recursively for video files,
/// extracts audio from each video, transcribes the audio to text using Whisper,
/// fetches episode metadata for the given show, and uses AI to match each video
/// to its corresponding episode.
///
/// # Arguments
///
/// * `directory` - The directory path to investigate
/// * `model_path` - Path to the Whisper model file (e.g., ggml-base.bin)
/// * `show_name` - The name of the TV show to fetch metadata for
///
/// # Returns
///
/// A vector of `MatchResult` containing the matched video files and their episodes
///
/// # Examples
///
/// ```no_run
/// use dialog_detective::investigate_case;
/// use std::path::Path;
///
/// let matches = investigate_case(
///     Path::new("/path/to/videos"),
///     Path::new("models/ggml-base.bin"),
///     "Breaking Bad"
/// ).unwrap();
///
/// for match_result in matches {
///     println!("Matched: {} -> S{:02}E{:02}",
///         match_result.video.path.display(),
///         match_result.episode.season_number,
///         match_result.episode.episode_number
///     );
/// }
/// ```
pub fn investigate_case(
    directory: &Path,
    model_path: &Path,
    show_name: &str,
) -> Result<Vec<MatchResult>, DialogDetectiveError> {
    println!(
        "DialogDetective reporting: Starting investigation in {} for {}...",
        directory.display(),
        show_name
    );

    // Fetch episode metadata with caching
    println!("\n=== Fetching Episode Metadata ===");
    println!("Retrieving episode information for '{}'...", show_name);

    // Initialize cache with 1-day TTL (24 hours)
    let cache =
        CacheStorage::<TVSeries>::open("metadata", Some(Duration::from_secs(24 * 60 * 60)))?;

    // Wrap the provider with caching
    let tvmaze_provider = TvMazeProvider::new();
    let provider = CachedMetadataProvider::new(tvmaze_provider, cache);

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
        return Ok(Vec::new());
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
        "\nTranscription complete! Processed {} video(s).",
        videos.len()
    );

    // Initialize the matcher with the prompt generator
    println!("\n=== Matching Episodes ===");
    let prompt_generator = NaivePromptGenerator::default();
    let matcher = ClaudeCodeMatcher::new(prompt_generator);

    let mut match_results = Vec::new();

    // Match each video to an episode
    for (index, (video, transcript)) in videos.iter().zip(transcripts.iter()).enumerate() {
        println!(
            "[{}/{}] Matching: {}",
            index + 1,
            videos.len(),
            video.path.display()
        );

        let episode = matcher.match_episode(transcript, &series)?;

        let match_result = MatchResult {
            video: video.clone(),
            episode,
        };

        match_results.push(match_result);
    }

    println!(
        "\nInvestigation complete! Matched {} video(s).",
        match_results.len()
    );

    Ok(match_results)
}
