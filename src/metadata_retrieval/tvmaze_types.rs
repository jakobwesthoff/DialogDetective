/// TVMaze API response types for deserialization.
///
/// These structures mirror the JSON response format from the TVMaze API.
use serde::Deserialize;

// =========================================================
// Search endpoint types (/search/shows)
// =========================================================

/// A single search result from the `/search/shows` endpoint.
///
/// The search endpoint returns an array of these, each containing a relevance
/// score and the matching show's metadata.
#[derive(Debug, Deserialize)]
pub(super) struct TvMazeSearchResult {
    /// Relevance score — present in the API response but not read directly;
    /// results arrive pre-sorted by score descending.
    #[allow(dead_code)]
    pub score: f64,
    pub show: TvMazeSearchShow,
}

/// Show metadata within a search result.
///
/// This is a subset of the full show object — just enough to identify the
/// series and present it as a selectable candidate.
#[derive(Debug, Deserialize)]
pub(super) struct TvMazeSearchShow {
    pub id: u64,
    pub name: String,
    /// ISO date string like "2008-01-20", used to extract the premiere year
    pub premiered: Option<String>,
}

// =========================================================
// Episode types (/shows/{id}/episodes)
// =========================================================

/// A single episode from the TVMaze API.
#[derive(Debug, Deserialize)]
pub(super) struct TvMazeEpisode {
    /// Season number (0 for specials)
    pub season: usize,
    /// Episode number within the season
    pub number: usize,
    /// Episode title (may be null for episodes without a title)
    pub name: Option<String>,
    /// Episode summary in HTML format (may be null)
    pub summary: Option<String>,
}
