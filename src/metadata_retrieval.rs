/// Data structures and traits for TV series metadata retrieval.
///
/// This module provides structures to represent TV series, seasons, and episodes
/// with their associated metadata (names, summaries, etc.), as well as traits
/// for implementing metadata providers.

use thiserror::Error;

/// Errors that can occur during metadata retrieval operations.
#[derive(Debug, Error)]
pub enum MetadataRetrievalError {
    // Error variants will be added as needed during implementation
}

/// Represents a single episode of a TV series.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Episode {
    /// The season number this episode belongs to
    pub season_number: usize,
    /// The episode number within the season
    pub episode_number: usize,
    /// The episode title
    pub name: String,
    /// A brief summary or description of the episode
    pub summary: String,
}

/// Represents a season of a TV series.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Season {
    /// The season number
    pub season_number: usize,
    /// List of episodes in this season
    pub episodes: Vec<Episode>,
}

/// Represents a complete TV series with all seasons and episodes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TVSeries {
    /// The name of the TV series
    pub name: String,
    /// List of seasons in this series
    pub seasons: Vec<Season>,
}

/// Trait for metadata providers that can fetch TV series information.
///
/// Implementors of this trait can retrieve episode metadata from various sources
/// such as TVDB, TMDB, or other episode databases.
pub(crate) trait MetadataProvider {
    /// Fetches metadata for a TV series.
    ///
    /// # Arguments
    ///
    /// * `series_name` - The name of the TV series to fetch
    /// * `season_numbers` - Optional list of specific season numbers to retrieve.
    ///                      If None, all seasons will be fetched.
    ///
    /// # Returns
    ///
    /// A Result containing the TVSeries with metadata, or a MetadataRetrievalError
    fn fetch_series(
        &self,
        series_name: &str,
        season_numbers: Option<Vec<usize>>,
    ) -> Result<TVSeries, MetadataRetrievalError>;
}
