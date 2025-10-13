//! Whisper model download and cache management
//!
//! This module handles automatic downloading and caching of Whisper GGML models
//! from Hugging Face. Models are stored in the system's standard cache directory
//! and reused across runs.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during model download operations
#[derive(Debug, Error)]
pub enum ModelDownloadError {
    /// Failed to determine cache directory location
    #[error("Failed to determine cache directory location")]
    CacheDirectoryNotFound,

    /// Failed to create or access cache directory
    #[error("Failed to create cache directory at {path}: {source}")]
    DirectoryCreationFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to download model from Hugging Face
    #[error("Failed to download model from {url}: {source}")]
    DownloadFailed {
        url: String,
        source: reqwest::Error,
    },

    /// Failed to write model file to cache
    #[error("Failed to write model file {path}: {source}")]
    WriteFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Model file is invalid or corrupted
    #[error("Invalid model file at {path}: {reason}")]
    InvalidModel { path: PathBuf, reason: String },

    /// HTTP error during download
    #[error("HTTP error downloading model: {0}")]
    HttpError(String),
}

/// Supported Whisper model names available from Hugging Face
///
/// This includes all models with various quantizations (q5_0, q5_1, q8_0)
/// from the ggerganov/whisper.cpp repository.
pub const SUPPORTED_MODELS: &[&str] = &[
    "tiny",
    "tiny.en",
    "tiny-q5_1",
    "tiny.en-q5_1",
    "tiny-q8_0",
    "base",
    "base.en",
    "base-q5_1",
    "base.en-q5_1",
    "base-q8_0",
    "small",
    "small.en",
    "small.en-tdrz",
    "small-q5_1",
    "small.en-q5_1",
    "small-q8_0",
    "medium",
    "medium.en",
    "medium-q5_0",
    "medium.en-q5_0",
    "medium-q8_0",
    "large-v1",
    "large-v2",
    "large-v2-q5_0",
    "large-v2-q8_0",
    "large-v3",
    "large-v3-q5_0",
    "large-v3-turbo",
    "large-v3-turbo-q5_0",
    "large-v3-turbo-q8_0",
];

/// Base URL for Whisper models on Hugging Face
const MODEL_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Minimum expected size for a model file (in bytes)
/// This helps detect incomplete downloads or corrupted files
const MIN_MODEL_SIZE: u64 = 1024 * 1024; // 1MB minimum

/// Ensures a Whisper model is available, downloading it if necessary
///
/// This function checks if the specified model exists in the cache directory.
/// If not found, it downloads the model from Hugging Face and stores it in
/// the cache for future use.
///
/// # Arguments
///
/// * `model_name` - Name of the Whisper model (e.g., "base", "base.en", "small")
///
/// # Returns
///
/// The path to the cached model file
///
/// # Examples
///
/// ```ignore
/// let model_path = ensure_model_available("base")?;
/// println!("Model ready at: {}", model_path.display());
/// ```
pub fn ensure_model_available(model_name: &str) -> Result<PathBuf, ModelDownloadError> {
    // Validate model name
    if !SUPPORTED_MODELS.contains(&model_name) {
        return Err(ModelDownloadError::InvalidModel {
            path: PathBuf::from(model_name),
            reason: format!(
                "Unsupported model name. Supported models: {}",
                SUPPORTED_MODELS.join(", ")
            ),
        });
    }

    // Get the cache directory for models
    let cache_dir = get_model_cache_dir()?;
    let model_path = cache_dir.join(format!("ggml-{}.bin", model_name));

    // Check if model already exists and is valid
    if model_path.exists() {
        // Verify the file has a reasonable size
        match fs::metadata(&model_path) {
            Ok(metadata) => {
                let size = metadata.len();
                if size >= MIN_MODEL_SIZE {
                    // Model exists and looks valid
                    return Ok(model_path);
                } else {
                    // File is too small, probably corrupted - remove and re-download
                    let _ = fs::remove_file(&model_path);
                }
            }
            Err(_) => {
                // Can't read metadata, remove and re-download
                let _ = fs::remove_file(&model_path);
            }
        }
    }

    // Model doesn't exist or is invalid - download it
    download_model(model_name, &model_path)?;

    Ok(model_path)
}

/// Downloads a Whisper model from Hugging Face
///
/// This function performs the actual HTTP download with progress reporting
/// and saves the model to the specified path.
///
/// # Arguments
///
/// * `model_name` - Name of the model to download
/// * `target_path` - Path where the model should be saved
///
/// # Returns
///
/// Ok(()) on success, or an error if download fails
fn download_model(model_name: &str, target_path: &Path) -> Result<(), ModelDownloadError> {
    let url = format!("{}/ggml-{}.bin", MODEL_BASE_URL, model_name);

    println!("🔍 Preparing evidence kit...");
    println!("📥 Downloading Whisper model '{}' from Hugging Face", model_name);
    println!("   This may take a few minutes depending on your connection...");
    print!("   Progress: ");
    io::stdout().flush().ok();

    // Create a blocking HTTP client
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600)) // 10 minute timeout
        .build()
        .map_err(|e| ModelDownloadError::DownloadFailed {
            url: url.clone(),
            source: e,
        })?;

    // Start the download
    let mut response = client
        .get(&url)
        .send()
        .map_err(|e| ModelDownloadError::DownloadFailed {
            url: url.clone(),
            source: e,
        })?;

    // Check HTTP status
    if !response.status().is_success() {
        return Err(ModelDownloadError::HttpError(format!(
            "HTTP {} while downloading model from {}",
            response.status(),
            url
        )));
    }

    // Get content length for progress reporting
    let total_size = response.content_length();

    // Create temporary file first (download to .tmp, then rename)
    let temp_path = target_path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path).map_err(|e| ModelDownloadError::WriteFailed {
        path: temp_path.clone(),
        source: e,
    })?;

    // Download with progress reporting
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192]; // 8KB buffer
    let mut last_progress_percent = 0;

    loop {
        let bytes_read = response.read(&mut buffer).map_err(|e| {
            ModelDownloadError::WriteFailed {
                path: temp_path.clone(),
                source: e,
            }
        })?;

        if bytes_read == 0 {
            break; // EOF
        }

        file.write_all(&buffer[..bytes_read])
            .map_err(|e| ModelDownloadError::WriteFailed {
                path: temp_path.clone(),
                source: e,
            })?;

        downloaded += bytes_read as u64;

        // Print progress every 10%
        if let Some(total) = total_size {
            let progress_percent = (downloaded * 100 / total) as u32;
            if progress_percent >= last_progress_percent + 10 {
                print!("{}% ", progress_percent);
                io::stdout().flush().ok();
                last_progress_percent = progress_percent;
            }
        }
    }

    println!("100% ✓");

    // Verify downloaded file size
    if downloaded < MIN_MODEL_SIZE {
        let _ = fs::remove_file(&temp_path);
        return Err(ModelDownloadError::InvalidModel {
            path: target_path.to_path_buf(),
            reason: format!(
                "Downloaded file is too small ({} bytes), expected at least {} bytes",
                downloaded, MIN_MODEL_SIZE
            ),
        });
    }

    // Rename temp file to final name (atomic operation)
    fs::rename(&temp_path, target_path).map_err(|e| ModelDownloadError::WriteFailed {
        path: target_path.to_path_buf(),
        source: e,
    })?;

    println!("✅ Model cached at: {}", target_path.display());

    Ok(())
}

/// Gets the cache directory for Whisper models
///
/// Returns the platform-specific cache directory path:
/// - Linux: ~/.cache/dialogdetective/models/
/// - macOS: ~/Library/Caches/dialogdetective/models/
/// - Windows: %LOCALAPPDATA%\dialogdetective\models\
fn get_model_cache_dir() -> Result<PathBuf, ModelDownloadError> {
    let proj_dirs = directories::ProjectDirs::from("de", "westhoffswelt", "dialogdetective")
        .ok_or(ModelDownloadError::CacheDirectoryNotFound)?;

    let cache_dir = proj_dirs.cache_dir().join("models");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&cache_dir).map_err(|e| ModelDownloadError::DirectoryCreationFailed {
        path: cache_dir.clone(),
        source: e,
    })?;

    Ok(cache_dir)
}

/// Returns the list of all supported model names
///
/// This is a convenience function that returns the list of model names
/// that can be downloaded from Hugging Face.
pub fn supported_models() -> &'static [&'static str] {
    SUPPORTED_MODELS
}

/// Lists all cached model files
///
/// Returns a list of model names (without the "ggml-" prefix and ".bin" extension)
/// that are currently cached.
pub fn list_cached_models() -> Result<Vec<String>, ModelDownloadError> {
    let cache_dir = get_model_cache_dir()?;

    let mut models = Vec::new();

    if let Ok(entries) = fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Extract model name from "ggml-{model}.bin" format
                    if file_name.starts_with("ggml-") && file_name.ends_with(".bin") {
                        let model_name = file_name
                            .strip_prefix("ggml-")
                            .and_then(|s| s.strip_suffix(".bin"))
                            .unwrap_or("")
                            .to_string();
                        if !model_name.is_empty() {
                            models.push(model_name);
                        }
                    }
                }
            }
        }
    }

    Ok(models)
}
