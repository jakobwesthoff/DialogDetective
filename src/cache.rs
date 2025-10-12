//! Cache storage module
//!
//! This module provides persistent caching functionality using the system's
//! standard cache directory. Data is serialized to JSON format for storage.

use serde::{Deserialize, Serialize};
use std::fs;
use std::marker::PhantomData;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during cache operations
#[derive(Debug, Error)]
pub enum CacheError {
    /// Failed to determine cache directory location
    #[error("Failed to determine cache directory location")]
    CacheDirectoryNotFound,

    /// Failed to create or access cache directory
    #[error("Failed to create cache directory at {path}: {source}")]
    DirectoryCreationFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to read cached data
    #[error("Failed to read cache file {path}: {source}")]
    ReadFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to write cached data
    #[error("Failed to write cache file {path}: {source}")]
    WriteFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to deserialize cached data
    #[error("Failed to deserialize cache file {path}: {source}")]
    DeserializationFailed {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Failed to serialize data for caching
    #[error("Failed to serialize data: {0}")]
    SerializationFailed(#[from] serde_json::Error),
}

/// A generic cache storage for serializable data
///
/// This structure provides persistent caching of data that implements
/// `Serialize` and `Deserialize`. Data is stored as JSON files in the
/// system's standard cache directory.
pub(crate) struct CacheStorage<T> {
    /// The directory where cached data is stored
    cache_dir: PathBuf,
    /// Phantom data for the generic type
    _phantom: PhantomData<T>,
}

impl<T> CacheStorage<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Opens or creates a cache storage with the given name
    ///
    /// The cache will be stored in the system's standard cache directory
    /// under a subdirectory named after the application and the provided name.
    /// The name will be sanitized (lowercased, non-alphanumeric characters
    /// replaced with underscores).
    ///
    /// # Arguments
    ///
    /// * `name` - The name for this cache storage
    ///
    /// # Returns
    ///
    /// A Result containing the CacheStorage or a CacheError
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let cache: CacheStorage<Transcript> = CacheStorage::open("transcripts")?;
    /// ```
    pub fn open(name: &str) -> Result<Self, CacheError> {
        // Get the cache directory for this application
        let proj_dirs = directories::ProjectDirs::from("de", "westhoffswelt", "dialogdetective")
            .ok_or(CacheError::CacheDirectoryNotFound)?;

        // Sanitize the cache name
        let sanitized_name = sanitize_name(name);

        // Build the full cache directory path
        let cache_dir = proj_dirs.cache_dir().join(&sanitized_name);

        // Create the directory if it doesn't exist
        fs::create_dir_all(&cache_dir).map_err(|e| CacheError::DirectoryCreationFailed {
            path: cache_dir.clone(),
            source: e,
        })?;

        Ok(Self {
            cache_dir,
            _phantom: PhantomData,
        })
    }

    /// Loads cached data for the given identifier
    ///
    /// # Arguments
    ///
    /// * `identifier` - A unique identifier for the cached data
    ///
    /// # Returns
    ///
    /// An Option containing the cached data if it exists and is valid,
    /// or None if the data doesn't exist. Returns an error if the data
    /// exists but cannot be read or deserialized.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some(transcript) = cache.load("video_123")? {
    ///     println!("Found cached transcript: {}", transcript.text);
    /// }
    /// ```
    pub fn load(&self, identifier: &str) -> Result<Option<T>, CacheError> {
        let sanitized_id = sanitize_name(identifier);
        let file_path = self.cache_dir.join(format!("{}.json", sanitized_id));

        // If file doesn't exist, return None
        if !file_path.exists() {
            return Ok(None);
        }

        // Read the file
        let content = fs::read_to_string(&file_path).map_err(|e| CacheError::ReadFailed {
            path: file_path.clone(),
            source: e,
        })?;

        // Deserialize the JSON
        let data =
            serde_json::from_str(&content).map_err(|e| CacheError::DeserializationFailed {
                path: file_path,
                source: e,
            })?;

        Ok(Some(data))
    }

    /// Stores data in the cache with the given identifier
    ///
    /// # Arguments
    ///
    /// * `identifier` - A unique identifier for the cached data
    /// * `data` - The data to cache
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```ignore
    /// cache.store("video_123", &transcript)?;
    /// ```
    pub fn store(&self, identifier: &str, data: &T) -> Result<(), CacheError> {
        let sanitized_id = sanitize_name(identifier);
        let file_path = self.cache_dir.join(format!("{}.json", sanitized_id));

        // Serialize to JSON
        let content = serde_json::to_string_pretty(data)?;

        // Write to file
        fs::write(&file_path, content).map_err(|e| CacheError::WriteFailed {
            path: file_path,
            source: e,
        })?;

        Ok(())
    }

    /// Returns the path to the cache directory
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}

/// Sanitizes a name for use in file paths
///
/// Converts to lowercase and replaces all characters that are not
/// a-z, 0-9, or hyphen with underscores.
fn sanitize_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("Simple"), "simple");
        assert_eq!(sanitize_name("With Spaces"), "with_spaces");
        assert_eq!(sanitize_name("With-Hyphens"), "with-hyphens");
        assert_eq!(sanitize_name("Special!@#$%"), "special_____");
        assert_eq!(sanitize_name("Mixed123ABC"), "mixed123abc");
    }
}
