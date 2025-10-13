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

// Public submodule for model downloading
pub mod model_downloader;

use ai_matcher::{ClaudeCodeMatcher, EpisodeMatcher, GeminiCliMatcher, NaivePromptGenerator};
use audio_extraction::audio_from_video;
use cache::CacheStorage;
use file_resolver::{VideoFile, compute_video_hash, scan_for_videos};
use metadata_retrieval::{
    CachedMetadataProvider, Episode, MetadataProvider, TVSeries, TvMazeProvider,
};
use speech_to_text::{Transcript, audio_to_text};
use std::time::Duration;

/// Computes a cache key for matching results
///
/// The cache key is composed of the video hash, show name, season filter,
/// and matcher type to ensure cached results are only reused when all
/// matching parameters are identical.
fn compute_matching_cache_key(
    video_hash: &str,
    show_name: &str,
    season_filter: &Option<Vec<usize>>,
    matcher_type: MatcherType,
) -> String {
    // Sanitize show name (lowercase, replace non-alphanumeric with underscores)
    let sanitized_show = show_name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    // Format season filter
    let seasons_str = match season_filter {
        Some(seasons) if !seasons.is_empty() => {
            let mut sorted = seasons.clone();
            sorted.sort_unstable();
            sorted
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join("-")
        }
        _ => "all".to_string(),
    };

    // Format matcher type
    let matcher_str = match matcher_type {
        MatcherType::Gemini => "gemini",
        MatcherType::Claude => "claude",
    };

    format!(
        "{}_{}_{}_{}",
        video_hash, sanitized_show, seasons_str, matcher_str
    )
}

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
    PlannedOperation, detect_duplicates, execute_copy, execute_rename, format_filename,
    plan_operations, sanitize_filename,
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

    /// Computing hash of video file
    Hashing { video_path: PathBuf },

    /// Hash computation finished
    HashingFinished { video_path: PathBuf },

    /// Extracting audio from video
    AudioExtraction {
        video_path: PathBuf,
        temp_path: PathBuf,
    },

    /// Audio extraction finished
    AudioExtractionFinished {
        video_path: PathBuf,
        temp_path: PathBuf,
    },

    /// Transcribing audio to text
    Transcription {
        video_path: PathBuf,
        temp_path: PathBuf,
    },

    /// Transcription finished
    TranscriptionFinished {
        video_path: PathBuf,
        language: String,
        text: String,
    },

    /// Transcript loaded from cache
    TranscriptCacheHit {
        video_path: PathBuf,
        language: String,
    },

    /// Matching video to an episode
    Matching {
        index: usize,
        total: usize,
        video_path: PathBuf,
    },

    /// Episode matching finished
    MatchingFinished {
        video_path: PathBuf,
        episode: Episode,
    },

    /// Matching result loaded from cache
    MatchingCacheHit {
        video_path: PathBuf,
        episode: Episode,
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

    // Initialize metadata cache with 1-day TTL (24 hours)
    let metadata_cache =
        CacheStorage::<TVSeries>::open("metadata", Some(Duration::from_secs(24 * 60 * 60)))?;

    // Initialize transcript cache with 1-day TTL (24 hours)
    let transcript_cache =
        CacheStorage::<Transcript>::open("transcripts", Some(Duration::from_secs(24 * 60 * 60)))?;

    // Initialize matching cache with 1-day TTL (24 hours)
    let matching_cache =
        CacheStorage::<Episode>::open("matching", Some(Duration::from_secs(24 * 60 * 60)))?;

    // Clean expired caches at startup
    transcript_cache.clean()?;
    matching_cache.clean()?;

    // Wrap the provider with caching
    let tvmaze_provider = TvMazeProvider::new();
    let provider = CachedMetadataProvider::new(tvmaze_provider, metadata_cache);

    let series = provider.fetch_series(show_name, season_filter.clone())?;

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

        // Compute video hash for cache lookup
        progress_callback(ProgressEvent::Hashing {
            video_path: video.path.clone(),
        });
        let video_hash = compute_video_hash(&video.path)?;
        progress_callback(ProgressEvent::HashingFinished {
            video_path: video.path.clone(),
        });

        let transcript = if let Some(cached_transcript) = transcript_cache.load(&video_hash)? {
            // Cache hit - use cached transcript
            progress_callback(ProgressEvent::TranscriptCacheHit {
                video_path: video.path.clone(),
                language: cached_transcript.language.clone(),
            });
            cached_transcript
        } else {
            // Cache miss - extract audio and transcribe
            progress_callback(ProgressEvent::AudioExtraction {
                video_path: video.path.clone(),
                temp_path: PathBuf::new(), // Will be set after extraction
            });
            let audio = audio_from_video(video)?;
            progress_callback(ProgressEvent::AudioExtractionFinished {
                video_path: video.path.clone(),
                temp_path: audio.to_path_buf(),
            });

            progress_callback(ProgressEvent::Transcription {
                video_path: video.path.clone(),
                temp_path: audio.to_path_buf(),
            });
            let transcript = audio_to_text(&audio, model_path)?;

            // Store in cache for future use
            transcript_cache.store(&video_hash, &transcript)?;

            progress_callback(ProgressEvent::TranscriptionFinished {
                video_path: video.path.clone(),
                language: transcript.language.clone(),
                text: transcript.text.clone(),
            });

            transcript
        };

        // Match the video to an episode (with caching)
        let matching_cache_key =
            compute_matching_cache_key(&video_hash, show_name, &season_filter, matcher_type);

        let episode = if let Some(cached_episode) = matching_cache.load(&matching_cache_key)? {
            // Cache hit - use cached matching result
            progress_callback(ProgressEvent::MatchingCacheHit {
                video_path: video.path.clone(),
                episode: cached_episode.clone(),
            });
            cached_episode
        } else {
            // Cache miss - perform matching
            progress_callback(ProgressEvent::Matching {
                index,
                total: videos.len(),
                video_path: video.path.clone(),
            });

            let episode = matcher.match_episode(&transcript, &series)?;

            // Store in cache for future use
            matching_cache.store(&matching_cache_key, &episode)?;

            progress_callback(ProgressEvent::MatchingFinished {
                video_path: video.path.clone(),
                episode: episode.clone(),
            });

            episode
        };

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
