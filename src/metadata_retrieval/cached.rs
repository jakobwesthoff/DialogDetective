//! Cached metadata provider implementation
//!
//! This module provides a caching wrapper for metadata providers that
//! automatically stores and retrieves TV series data from a local cache.

use super::{MetadataProvider, MetadataRetrievalError, TVSeries};
use crate::cache::CacheStorage;

/// A caching wrapper for metadata providers
///
/// This provider wraps another metadata provider and caches the results
/// to avoid redundant network requests. The cache is persistent across
/// application runs.
pub(crate) struct CachedMetadataProvider<P>
where
    P: MetadataProvider,
{
    /// The underlying metadata provider
    provider: P,
    /// Cache storage for TV series data
    cache: CacheStorage<TVSeries>,
}

impl<P> CachedMetadataProvider<P>
where
    P: MetadataProvider,
{
    /// Creates a new cached metadata provider wrapping the given provider
    ///
    /// # Arguments
    ///
    /// * `provider` - The metadata provider to wrap
    /// * `cache` - The cache storage to use for caching
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let tvmaze = TvMazeProvider::new();
    /// let cache = CacheStorage::open("metadata")?;
    /// let cached = CachedMetadataProvider::new(tvmaze, cache);
    /// ```
    pub fn new(provider: P, cache: CacheStorage<TVSeries>) -> Self {
        Self { provider, cache }
    }

    /// Generates a cache key for a series query
    ///
    /// The key combines the series name with optional season numbers
    /// to ensure different queries are cached separately.
    fn cache_key(series_name: &str, season_numbers: &Option<Vec<usize>>) -> String {
        match season_numbers {
            None => series_name.to_string(),
            Some(seasons) => {
                let mut seasons_sorted = seasons.clone();
                seasons_sorted.sort_unstable();
                format!(
                    "{}_seasons_{}",
                    series_name,
                    seasons_sorted
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join("_")
                )
            }
        }
    }
}

impl<P> MetadataProvider for CachedMetadataProvider<P>
where
    P: MetadataProvider,
{
    fn fetch_series(
        &self,
        series_name: &str,
        season_numbers: Option<Vec<usize>>,
    ) -> Result<TVSeries, MetadataRetrievalError> {
        // Generate cache key
        let cache_key = Self::cache_key(series_name, &season_numbers);

        // Try to load from cache
        match self.cache.load(&cache_key) {
            Ok(Some(series)) => {
                // Cache hit - return cached data
                return Ok(series);
            }
            Ok(None) => {
                // Cache miss - continue to fetch from provider
            }
            Err(_) => {
                // Cache read error - continue to fetch from provider
                // We don't want cache failures to prevent metadata retrieval
            }
        }

        // Fetch from underlying provider
        let series = self.provider.fetch_series(series_name, season_numbers)?;

        // Store in cache (ignore errors to avoid failing the request)
        let _ = self.cache.store(&cache_key, &series);

        Ok(series)
    }
}
