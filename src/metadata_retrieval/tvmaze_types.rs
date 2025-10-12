/// TVMaze API response types for deserialization.
///
/// These structures mirror the JSON response format from the TVMaze API.
use serde::Deserialize;

/// The top-level response from the TVMaze singlesearch endpoint.
#[derive(Debug, Deserialize)]
pub(super) struct TvMazeShow {
    /// The name of the TV show
    pub name: String,
    /// Embedded resources (like episodes) when requested with ?embed=
    #[serde(rename = "_embedded")]
    pub embedded: Option<TvMazeEmbedded>,
}

/// Embedded resources in a TVMaze show response.
#[derive(Debug, Deserialize)]
pub(super) struct TvMazeEmbedded {
    /// List of episodes when embed=episodes is used
    pub episodes: Vec<TvMazeEpisode>,
}

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
