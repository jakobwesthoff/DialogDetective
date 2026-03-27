/// Data structures and traits for TV series metadata retrieval.
///
/// This module provides structures to represent TV series, seasons, and episodes
/// with their associated metadata (names, summaries, etc.), as well as traits
/// for implementing metadata providers.
mod cached;
mod tvmaze;
mod tvmaze_types;

pub(crate) use cached::CachedMetadataProvider;
pub(crate) use tvmaze::TvMazeProvider;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during metadata retrieval operations.
#[derive(Debug, Error)]
pub enum MetadataRetrievalError {
    /// Request to the metadata provider failed
    #[error("Request failed: {0}")]
    RequestError(String),

    /// Failed to parse the provider's JSON response
    #[error("Failed to parse API response: {0}")]
    ParseError(String),

    /// The requested series was not found
    #[error("Series not found: {0}")]
    SeriesNotFound(String),

    /// The API returned invalid or unexpected data
    #[error("API returned invalid data: {0}")]
    InvalidData(String),
}

/// A candidate TV series returned from a search query.
///
/// Represents a potential match before the user has confirmed which series
/// they want. Contains just enough information for display and selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesCandidate {
    /// Provider-specific ID (e.g. TVMaze show ID)
    pub id: u64,
    /// Series name as returned by the provider
    pub name: String,
    /// Premiere year (extracted from premiered date), if available
    pub year: Option<u16>,
}

/// Represents a single episode of a TV series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Season {
    /// The season number
    pub season_number: usize,
    /// List of episodes in this season
    pub episodes: Vec<Episode>,
}

/// Represents a complete TV series with all seasons and episodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TVSeries {
    /// The name of the TV series
    pub name: String,
    /// List of seasons in this series
    pub seasons: Vec<Season>,
}

/// Trait for metadata providers that can fetch TV series information.
///
/// The retrieval process is split into two steps: searching for candidates
/// and then fetching full episode data for the selected candidate. This
/// allows the caller to present multiple matches and let the user choose.
pub(crate) trait MetadataProvider {
    /// Searches for TV series matching the given name.
    ///
    /// Returns up to 10 candidates sorted by relevance score.
    fn search_series(
        &self,
        series_name: &str,
    ) -> Result<Vec<SeriesCandidate>, MetadataRetrievalError>;

    /// Fetches full episode metadata for a specific series candidate.
    ///
    /// # Arguments
    ///
    /// * `candidate` - The selected series candidate from `search_series`
    /// * `season_numbers` - Optional list of specific season numbers to retrieve.
    ///                      If None, all seasons will be fetched.
    fn fetch_series(
        &self,
        candidate: &SeriesCandidate,
        season_numbers: Option<Vec<usize>>,
    ) -> Result<TVSeries, MetadataRetrievalError>;
}
