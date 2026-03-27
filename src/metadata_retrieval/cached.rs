//! Cached metadata provider implementation
//!
//! This module provides a caching wrapper for metadata providers that
//! automatically stores and retrieves both search results and TV series
//! data from a local cache.

use super::{MetadataProvider, MetadataRetrievalError, SeriesCandidate, TVSeries};
use crate::cache::CacheStorage;

/// A caching wrapper for metadata providers.
///
/// Wraps another metadata provider and caches both search results and
/// episode metadata to avoid redundant network requests. Caches are
/// persistent across application runs.
pub(crate) struct CachedMetadataProvider<P>
where
    P: MetadataProvider,
{
    provider: P,
    /// Cache for search results, keyed by lowercased query string
    search_cache: CacheStorage<Vec<SeriesCandidate>>,
    /// Cache for episode metadata, keyed by provider ID + season filter
    metadata_cache: CacheStorage<TVSeries>,
}

impl<P> CachedMetadataProvider<P>
where
    P: MetadataProvider,
{
    /// Creates a new cached metadata provider wrapping the given provider.
    pub fn new(
        provider: P,
        search_cache: CacheStorage<Vec<SeriesCandidate>>,
        metadata_cache: CacheStorage<TVSeries>,
    ) -> Self {
        Self {
            provider,
            search_cache,
            metadata_cache,
        }
    }

    /// Generates a cache key for a search query.
    fn search_cache_key(series_name: &str) -> String {
        series_name.to_lowercase()
    }

    /// Generates a cache key for episode metadata.
    ///
    /// Uses the provider-specific ID to ensure different shows with
    /// similar names are cached separately.
    fn metadata_cache_key(
        candidate: &SeriesCandidate,
        season_numbers: &Option<Vec<usize>>,
    ) -> String {
        match season_numbers {
            None => format!("tvmaze_{}", candidate.id),
            Some(seasons) => {
                let mut seasons_sorted = seasons.clone();
                seasons_sorted.sort_unstable();
                format!(
                    "tvmaze_{}_seasons_{}",
                    candidate.id,
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
    fn search_series(
        &self,
        series_name: &str,
    ) -> Result<Vec<SeriesCandidate>, MetadataRetrievalError> {
        let cache_key = Self::search_cache_key(series_name);

        // Try to load from cache
        match self.search_cache.load(&cache_key) {
            Ok(Some(candidates)) => return Ok(candidates),
            Ok(None) => {}
            Err(_) => {
                // Cache read error — continue to fetch from provider
            }
        }

        let candidates = self.provider.search_series(series_name)?;

        // Store in cache (ignore errors to avoid failing the request)
        let _ = self.search_cache.store(&cache_key, &candidates);

        Ok(candidates)
    }

    fn fetch_series(
        &self,
        candidate: &SeriesCandidate,
        season_numbers: Option<Vec<usize>>,
    ) -> Result<TVSeries, MetadataRetrievalError> {
        let cache_key = Self::metadata_cache_key(candidate, &season_numbers);

        // Try to load from cache
        match self.metadata_cache.load(&cache_key) {
            Ok(Some(series)) => return Ok(series),
            Ok(None) => {}
            Err(_) => {
                // Cache read error — continue to fetch from provider
            }
        }

        let series = self.provider.fetch_series(candidate, season_numbers)?;

        // Store in cache (ignore errors to avoid failing the request)
        let _ = self.metadata_cache.store(&cache_key, &series);

        Ok(series)
    }
}
