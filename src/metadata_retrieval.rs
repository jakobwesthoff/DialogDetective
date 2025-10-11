/// Data structures for TV series metadata.
///
/// This module provides structures to represent TV series, seasons, and episodes
/// with their associated metadata (names, summaries, etc.).

/// Represents a single episode of a TV series.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Episode {
    /// The season number this episode belongs to
    pub season_number: u32,
    /// The episode number within the season
    pub episode_number: u32,
    /// The episode title
    pub name: String,
    /// A brief summary or description of the episode
    pub summary: String,
}

/// Represents a season of a TV series.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Season {
    /// The season number
    pub season_number: u32,
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
