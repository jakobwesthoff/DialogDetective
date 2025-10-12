//! Cache storage module
//!
//! This module provides persistent caching functionality using the system's
//! standard cache directory. Data is serialized to JSON format for storage.

use serde::{Deserialize, Serialize};
use std::fs;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
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

/// Internal wrapper for cached data with timestamp
#[derive(Debug, Serialize, Deserialize)]
struct CachedItem<T> {
    data: T,
    timestamp: SystemTime,
}

/// A generic cache storage for serializable data
///
/// This structure provides persistent caching of data that implements
/// `Serialize` and `Deserialize`. Data is stored as JSON files in the
/// system's standard cache directory.
pub(crate) struct CacheStorage<T> {
    /// The directory where cached data is stored
    cache_dir: PathBuf,
    /// Optional time-to-live for cached items
    ttl: Option<Duration>,
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
    /// * `ttl` - Optional time-to-live for cached items. If provided, items older
    ///           than this duration will be considered expired and automatically removed.
    ///
    /// # Returns
    ///
    /// A Result containing the CacheStorage or a CacheError
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Cache without TTL
    /// let cache: CacheStorage<Transcript> = CacheStorage::open("transcripts", None)?;
    ///
    /// // Cache with 24-hour TTL
    /// let cache: CacheStorage<Transcript> = CacheStorage::open("transcripts", Some(Duration::from_secs(86400)))?;
    /// ```
    pub fn open(name: &str, ttl: Option<Duration>) -> Result<Self, CacheError> {
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
            ttl,
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
    /// An Option containing the cached data if it exists and is not expired,
    /// or None if the data doesn't exist or is expired. Returns an error if the data
    /// exists but cannot be read or deserialized. Expired items are automatically removed.
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

        // Deserialize the JSON (wrapped with timestamp)
        let cached_item: CachedItem<T> =
            serde_json::from_str(&content).map_err(|e| CacheError::DeserializationFailed {
                path: file_path.clone(),
                source: e,
            })?;

        // Check if TTL is set and if the item is expired
        if let Some(ttl) = self.ttl {
            if let Ok(age) = SystemTime::now().duration_since(cached_item.timestamp) {
                if age > ttl {
                    // Item is expired, remove it
                    let _ = self.remove(identifier);
                    return Ok(None);
                }
            }
        }

        Ok(Some(cached_item.data))
    }

    /// Stores data in the cache with the given identifier
    ///
    /// If the item already exists, it will be overwritten with a new timestamp.
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

        // Wrap data with current timestamp
        let cached_item = CachedItem {
            data,
            timestamp: SystemTime::now(),
        };

        // Serialize to JSON
        let content = serde_json::to_string_pretty(&cached_item)?;

        // Write to file
        fs::write(&file_path, content).map_err(|e| CacheError::WriteFailed {
            path: file_path,
            source: e,
        })?;

        Ok(())
    }

    /// Removes a cached item with the given identifier
    ///
    /// # Arguments
    ///
    /// * `identifier` - A unique identifier for the cached data
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure. Returns Ok(()) even if the file
    /// doesn't exist (idempotent operation).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// cache.remove("video_123")?;
    /// ```
    pub fn remove(&self, identifier: &str) -> Result<(), CacheError> {
        let sanitized_id = sanitize_name(identifier);
        let file_path = self.cache_dir.join(format!("{}.json", sanitized_id));

        // Remove file if it exists (ignore error if it doesn't exist)
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| CacheError::WriteFailed {
                path: file_path,
                source: e,
            })?;
        }

        Ok(())
    }

    /// Returns the path to the cache directory
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Removes all expired items from the cache
    ///
    /// This method scans all cached items and removes those that have exceeded
    /// their TTL. Only works on cache storages that have a TTL configured.
    /// Returns the number of items that were removed.
    ///
    /// # Returns
    ///
    /// A Result containing the count of removed items, or None if no TTL is set
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some(removed_count) = cache.clean()? {
    ///     println!("Removed {} expired items", removed_count);
    /// }
    /// ```
    pub fn clean(&self) -> Result<Option<usize>, CacheError> {
        // Only works if TTL is set
        let ttl = match self.ttl {
            Some(ttl) => ttl,
            None => return Ok(None),
        };

        let mut removed_count = 0;

        // Read all files in the cache directory
        let entries = fs::read_dir(&self.cache_dir).map_err(|e| CacheError::ReadFailed {
            path: self.cache_dir.clone(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| CacheError::ReadFailed {
                path: self.cache_dir.clone(),
                source: e,
            })?;

            let path = entry.path();

            // Only process .json files
            if !path.extension().map_or(false, |ext| ext == "json") {
                continue;
            }

            // Try to read and deserialize the file
            match fs::read_to_string(&path) {
                Ok(content) => {
                    // Try to deserialize to get the timestamp
                    if let Ok(cached_item) =
                        serde_json::from_str::<CachedItem<serde_json::Value>>(&content)
                    {
                        // Check if expired
                        if let Ok(age) = SystemTime::now().duration_since(cached_item.timestamp) {
                            if age > ttl {
                                // Remove expired file
                                if fs::remove_file(&path).is_ok() {
                                    removed_count += 1;
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // If we can't read the file, skip it
                    continue;
                }
            }
        }

        Ok(Some(removed_count))
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
