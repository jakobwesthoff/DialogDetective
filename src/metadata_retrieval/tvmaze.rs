/// TVMaze metadata provider implementation.
use super::tvmaze_types::{TvMazeEpisode, TvMazeShow};
use super::{Episode, MetadataProvider, MetadataRetrievalError, Season, TVSeries};
use std::collections::HashMap;

/// Metadata provider for the TVMaze API.
///
/// This provider fetches TV series information from https://api.tvmaze.com
/// using the singlesearch endpoint with embedded episodes.
pub(crate) struct TvMazeProvider {
    client: reqwest::blocking::Client,
    base_url: String,
}

impl TvMazeProvider {
    /// Creates a new TVMaze provider instance.
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            base_url: "https://api.tvmaze.com".to_string(),
        }
    }

    /// Converts a TVMaze episode to our internal Episode structure.
    fn convert_episode(tvmaze_episode: TvMazeEpisode) -> Episode {
        Episode {
            season_number: tvmaze_episode.season,
            episode_number: tvmaze_episode.number,
            name: tvmaze_episode.name.unwrap_or_else(|| "Unknown".to_string()),
            summary: tvmaze_episode
                .summary
                .map(|s| nanohtml2text::html2text(&s).trim().to_string())
                .unwrap_or_default(),
        }
    }

    /// Converts TVMaze show data to our internal TVSeries structure.
    ///
    /// Groups episodes by season and optionally filters by season numbers.
    fn convert_to_series(
        tvmaze_show: TvMazeShow,
        season_filter: Option<Vec<usize>>,
    ) -> Result<TVSeries, MetadataRetrievalError> {
        // Extract episodes from embedded data
        let episodes = tvmaze_show
            .embedded
            .ok_or_else(|| {
                MetadataRetrievalError::InvalidData("No episodes found in API response".to_string())
            })?
            .episodes;

        // Group episodes by season number
        let mut seasons_map: HashMap<usize, Vec<Episode>> = HashMap::new();

        for tvmaze_episode in episodes {
            // Skip if filtering seasons and this season is not in the filter
            if let Some(ref filter) = season_filter {
                if !filter.contains(&tvmaze_episode.season) {
                    continue;
                }
            }

            seasons_map
                .entry(tvmaze_episode.season)
                .or_insert_with(Vec::new)
                .push(Self::convert_episode(tvmaze_episode));
        }

        // Convert HashMap to Vec<Season>, sorted by season number
        let mut seasons: Vec<Season> = seasons_map
            .into_iter()
            .map(|(season_number, mut episodes)| {
                // Sort episodes by episode number within each season
                episodes.sort_by_key(|e| e.episode_number);
                Season {
                    season_number,
                    episodes,
                }
            })
            .collect();

        // Sort seasons by season number
        seasons.sort_by_key(|s| s.season_number);

        Ok(TVSeries {
            name: tvmaze_show.name,
            seasons,
        })
    }
}

impl MetadataProvider for TvMazeProvider {
    fn fetch_series(
        &self,
        series_name: &str,
        season_numbers: Option<Vec<usize>>,
    ) -> Result<TVSeries, MetadataRetrievalError> {
        // Build the API URL
        let url = format!("{}/singlesearch/shows", self.base_url);

        // Make the HTTP request with query parameters
        let response = self
            .client
            .get(&url)
            .query(&[("q", series_name), ("embed", "episodes")])
            .send()
            .map_err(|e| MetadataRetrievalError::RequestError(e.to_string()))?;

        // Check if the series was found
        if response.status() == 404 {
            return Err(MetadataRetrievalError::SeriesNotFound(
                series_name.to_string(),
            ));
        }

        // Ensure request was successful
        if !response.status().is_success() {
            return Err(MetadataRetrievalError::RequestError(format!(
                "HTTP {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        // Parse the JSON response
        let tvmaze_show: TvMazeShow = response
            .json()
            .map_err(|e| MetadataRetrievalError::ParseError(e.to_string()))?;

        // Convert to our internal structures
        Self::convert_to_series(tvmaze_show, season_numbers)
    }
}
