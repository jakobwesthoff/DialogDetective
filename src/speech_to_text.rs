//! Speech-to-text module
//!
//! This module provides functionality to transcribe audio files to text
//! using Whisper speech recognition.

use crate::audio_extraction::AudioFile;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use thiserror::Error;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Errors that can occur during speech-to-text transcription
#[derive(Debug, Error)]
pub enum SpeechToTextError {
    /// Failed to load Whisper model
    #[error("Failed to load Whisper model from {path}: {message}")]
    ModelLoadFailed { path: PathBuf, message: String },

    /// Failed to read audio file
    #[error("Failed to read audio file {path}: {message}")]
    AudioReadFailed { path: PathBuf, message: String },

    /// Invalid audio format
    #[error("Invalid audio format: {0}")]
    InvalidAudioFormat(String),

    /// Transcription failed
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    /// Language detection failed
    #[error("Failed to detect language: invalid language ID {0}")]
    LanguageDetectionFailed(i32),

    /// Model not initialized
    #[error("Whisper model not initialized")]
    ModelNotInitialized,
}

/// Represents a transcribed text with metadata
#[derive(Debug, Clone)]
pub(crate) struct Transcript {
    /// The transcribed text content
    pub text: String,

    /// Language detected during transcription
    pub language: String,
}

/// Transcribes audio to text using Whisper
///
/// This function analyzes the audio file and produces a text transcript
/// of the spoken content. This is a key clue in solving the mystery of
/// identifying unknown video files.
///
/// # Arguments
///
/// * `audio` - The audio file to transcribe
/// * `model_path` - Path to the Whisper model file (e.g., ggml-base.bin)
///
/// # Returns
///
/// A `Transcript` containing the transcribed text and metadata,
/// or an error if transcription fails.
///
/// # Examples
///
/// ```ignore
/// let audio = audio_from_video(&video).unwrap();
/// let model_path = Path::new("models/ggml-base.bin");
/// let transcript = audio_to_text(&audio, model_path).unwrap();
/// println!("Transcribed: {}", transcript.text);
/// ```
pub(crate) fn audio_to_text(
    audio: &AudioFile,
    model_path: &Path,
) -> Result<Transcript, SpeechToTextError> {
    // Suppress whisper.cpp log output by installing logging hooks.
    // Since we don't have the log_backend or tracing_backend features enabled,
    // this effectively silences all whisper.cpp and GGML logs to stdout/stderr.
    // Safe to call multiple times - only has effect on first call.
    whisper_rs::install_logging_hooks();

    // Load Whisper model with GPU acceleration enabled
    let mut params = WhisperContextParameters::default();
    params.use_gpu(true); // Enable GPU (Metal on macOS, CUDA, or Vulkan) - falls back to CPU if unavailable

    let ctx = WhisperContext::new_with_params(
        model_path
            .to_str()
            .ok_or_else(|| SpeechToTextError::ModelLoadFailed {
                path: model_path.to_path_buf(),
                message: "Invalid UTF-8 in model path".to_string(),
            })?,
        params,
    )
    .map_err(|e| SpeechToTextError::ModelLoadFailed {
        path: model_path.to_path_buf(),
        message: e.to_string(),
    })?;

    // Read WAV file
    let reader =
        hound::WavReader::open(audio.deref()).map_err(|e| SpeechToTextError::AudioReadFailed {
            path: audio.deref().to_path_buf(),
            message: e.to_string(),
        })?;

    // Verify audio format (16kHz mono as extracted by ffmpeg)
    let spec = reader.spec();
    if spec.sample_rate != 16000 {
        return Err(SpeechToTextError::InvalidAudioFormat(format!(
            "Expected 16kHz sample rate, got {} Hz",
            spec.sample_rate
        )));
    }
    if spec.channels != 1 {
        return Err(SpeechToTextError::InvalidAudioFormat(format!(
            "Expected mono audio (1 channel), got {} channels",
            spec.channels
        )));
    }

    // Read i16 samples
    let samples: Vec<i16> = reader
        .into_samples::<i16>()
        .collect::<Result<Vec<i16>, _>>()
        .map_err(|e| SpeechToTextError::AudioReadFailed {
            path: audio.deref().to_path_buf(),
            message: e.to_string(),
        })?;

    // Convert i16 to f32
    let mut audio_data = vec![0.0f32; samples.len()];
    whisper_rs::convert_integer_to_float_audio(&samples, &mut audio_data)
        .map_err(|e| SpeechToTextError::InvalidAudioFormat(e.to_string()))?;

    // Drop i16 samples immediately to free memory
    drop(samples);

    // Create transcription parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Create a state for transcription
    let mut state = ctx.create_state().map_err(|e| {
        SpeechToTextError::TranscriptionFailed(format!("Failed to create state: {}", e))
    })?;

    // Run transcription
    state
        .full(params, &audio_data[..])
        .map_err(|e| SpeechToTextError::TranscriptionFailed(e.to_string()))?;

    // Drop audio data immediately to free memory
    drop(audio_data);

    // Get detected language
    let lang_id = state.full_lang_id_from_state();
    let language = whisper_rs::get_lang_str(lang_id)
        .ok_or(SpeechToTextError::LanguageDetectionFailed(lang_id))?
        .to_string();

    // Extract transcribed text from segments
    let mut text = String::new();
    for segment in state.as_iter() {
        text.push_str(&format!("{}", segment));
    }

    Ok(Transcript {
        text: text.trim().to_string(),
        language,
    })
}
