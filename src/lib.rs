//! DialogDetective - Automatically identify and rename unknown video files
//!
//! This library provides the core functionality for investigating video files,
//! analyzing their audio content, and solving the mystery of their true identity.

mod ai_matcher;
mod audio_extraction;
mod cache;
mod file_operations;
mod file_resolver;
mod metadata_retrieval;
mod speech_to_text;
mod temp;

use ai_matcher::{ClaudeCodeMatcher, EpisodeMatcher, GeminiCliMatcher, NaivePromptGenerator};
use audio_extraction::audio_from_video;
use cache::CacheStorage;
use file_resolver::{VideoFile, scan_for_videos};
use metadata_retrieval::{
    CachedMetadataProvider, Episode, MetadataProvider, TVSeries, TvMazeProvider,
};
use speech_to_text::audio_to_text;
use std::time::Duration;

// Re-export error types
pub use ai_matcher::EpisodeMatchingError;
pub use audio_extraction::AudioExtractionError;
pub use cache::CacheError;
pub use file_operations::FileOperationError;
pub use file_resolver::FileResolverError;
pub use metadata_retrieval::MetadataRetrievalError;
pub use speech_to_text::SpeechToTextError;

// Re-export file operations types
pub use file_operations::{
    PlannedOperation, detect_duplicates, execute_copy, execute_rename, plan_operations,
    sanitize_filename, format_filename,
};

use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// AI matcher type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatcherType {
    /// Use Gemini CLI for episode matching
    Gemini,
    /// Use Claude Code CLI for episode matching
    Claude,
}

/// Progress event emitted during investigation
///
/// These events allow library users to track progress and provide feedback
/// during the investigation process.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Investigation started
    Started {
        directory: PathBuf,
        show_name: String,
    },

    /// Fetching episode metadata
    FetchingMetadata { show_name: String },

    /// Metadata successfully fetched
    MetadataFetched {
        series_name: String,
        season_count: usize,
    },

    /// Scanning directory for video files
    ScanningVideos,

    /// Video files found
    VideosFound { count: usize },

    /// Processing a specific video file
    ProcessingVideo {
        index: usize,
        total: usize,
        video_path: PathBuf,
    },

    /// Extracting audio from video
    ExtractingAudio {
        video_path: PathBuf,
        temp_path: PathBuf,
    },

    /// Transcribing audio to text
    TranscribingAudio {
        video_path: PathBuf,
        temp_path: PathBuf,
    },

    /// Transcription completed
    TranscriptionComplete {
        video_path: PathBuf,
        language: String,
        text: String,
    },

    /// Matching a specific video to an episode
    MatchingVideo {
        index: usize,
        total: usize,
        video_path: PathBuf,
    },

    /// Investigation complete
    Complete { match_count: usize },
}

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
/// Progress events are emitted through the provided callback, allowing library
/// users to track progress, display status, or remain silent.
///
/// # Arguments
///
/// * `directory` - The directory path to investigate
/// * `model_path` - Path to the Whisper model file (e.g., ggml-base.bin)
/// * `show_name` - The name of the TV show to fetch metadata for
/// * `season_filter` - Optional list of season numbers to filter (None fetches all seasons)
/// * `matcher_type` - The AI matcher to use (Gemini or Claude)
/// * `progress_callback` - Closure called with progress events (can be empty for silent operation)
///
/// # Returns
///
/// A vector of `MatchResult` containing the matched video files and their episodes
///
/// # Examples
///
/// ```no_run
/// use dialog_detective::{investigate_case, ProgressEvent, MatcherType};
/// use std::path::Path;
///
/// // With progress output and season filtering
/// let matches = investigate_case(
///     Path::new("/path/to/videos"),
///     Path::new("models/ggml-base.bin"),
///     "Breaking Bad",
///     Some(vec![1, 2]),  // Only seasons 1 and 2
///     MatcherType::Gemini,
///     |event| {
///         match event {
///             ProgressEvent::ProcessingVideo { index, total, video_path } => {
///                 println!("[{}/{}] Processing: {}", index, total, video_path.display());
///             }
///             _ => {} // Handle other events as needed
///         }
///     }
/// ).unwrap();
///
/// // Silent operation with all seasons
/// let matches = investigate_case(
///     Path::new("/path/to/videos"),
///     Path::new("models/ggml-base.bin"),
///     "Breaking Bad",
///     None,  // All seasons
///     MatcherType::Claude,
///     |_| {} // Ignore all progress events
/// ).unwrap();
/// ```
pub fn investigate_case<F>(
    directory: &Path,
    model_path: &Path,
    show_name: &str,
    season_filter: Option<Vec<usize>>,
    matcher_type: MatcherType,
    mut progress_callback: F,
) -> Result<Vec<MatchResult>, DialogDetectiveError>
where
    F: FnMut(ProgressEvent),
{
    progress_callback(ProgressEvent::Started {
        directory: directory.to_path_buf(),
        show_name: show_name.to_string(),
    });

    // Fetch episode metadata with caching
    progress_callback(ProgressEvent::FetchingMetadata {
        show_name: show_name.to_string(),
    });

    // Initialize cache with 1-day TTL (24 hours)
    let cache =
        CacheStorage::<TVSeries>::open("metadata", Some(Duration::from_secs(24 * 60 * 60)))?;

    // Wrap the provider with caching
    let tvmaze_provider = TvMazeProvider::new();
    let provider = CachedMetadataProvider::new(tvmaze_provider, cache);

    let series = provider.fetch_series(show_name, season_filter)?;

    progress_callback(ProgressEvent::MetadataFetched {
        series_name: series.name.clone(),
        season_count: series.seasons.len(),
    });

    // Scan directory for video files
    progress_callback(ProgressEvent::ScanningVideos);
    let videos = scan_for_videos(directory)?;

    if videos.is_empty() {
        progress_callback(ProgressEvent::VideosFound { count: 0 });
        return Ok(Vec::new());
    }

    progress_callback(ProgressEvent::VideosFound {
        count: videos.len(),
    });

    // Initialize the matcher based on the selected type
    let prompt_generator = NaivePromptGenerator::default();
    let matcher: Box<dyn EpisodeMatcher> = match matcher_type {
        MatcherType::Gemini => Box::new(GeminiCliMatcher::new(prompt_generator)),
        MatcherType::Claude => Box::new(ClaudeCodeMatcher::new(prompt_generator)),
    };

    let mut match_results = Vec::new();

    // Process each video file: transcribe then match immediately
    for (index, video) in videos.iter().enumerate() {
        progress_callback(ProgressEvent::ProcessingVideo {
            index,
            total: videos.len(),
            video_path: video.path.clone(),
        });

        // Extract audio
        let audio = audio_from_video(video)?;
        progress_callback(ProgressEvent::ExtractingAudio {
            video_path: video.path.clone(),
            temp_path: audio.to_path_buf(),
        });

        // Transcribe audio to text
        progress_callback(ProgressEvent::TranscribingAudio {
            video_path: video.path.clone(),
            temp_path: audio.to_path_buf(),
        });
        let transcript = audio_to_text(&audio, model_path)?;

        progress_callback(ProgressEvent::TranscriptionComplete {
            video_path: video.path.clone(),
            language: transcript.language.clone(),
            text: transcript.text.clone(),
        });

        // Match the video to an episode immediately after transcription
        progress_callback(ProgressEvent::MatchingVideo {
            index,
            total: videos.len(),
            video_path: video.path.clone(),
        });

        let episode = matcher.match_episode(&transcript, &series)?;

        let match_result = MatchResult {
            video: video.clone(),
            episode,
        };

        match_results.push(match_result);
    }

    progress_callback(ProgressEvent::Complete {
        match_count: match_results.len(),
    });

    Ok(match_results)
}
