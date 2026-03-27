/// TVMaze metadata provider implementation.
///
/// Uses the search endpoint to find candidates, then fetches episodes
/// for the selected show in a separate request.
use super::tvmaze_types::{TvMazeEpisode, TvMazeSearchResult};
use super::{Episode, MetadataProvider, MetadataRetrievalError, Season, SeriesCandidate, TVSeries};
use std::collections::HashMap;

/// Maximum number of search results to return as candidates.
const MAX_CANDIDATES: usize = 10;

/// Metadata provider for the TVMaze API.
///
/// This provider fetches TV series information from https://api.tvmaze.com
/// using the search endpoint for candidate discovery and the episodes
/// endpoint for full metadata retrieval.
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

    /// Groups a flat list of episodes into sorted seasons, optionally filtered.
    fn group_into_seasons(
        episodes: Vec<TvMazeEpisode>,
        season_filter: Option<Vec<usize>>,
    ) -> Vec<Season> {
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
                .or_default()
                .push(Self::convert_episode(tvmaze_episode));
        }

        // Convert HashMap to Vec<Season>, sorted by season number
        let mut seasons: Vec<Season> = seasons_map
            .into_iter()
            .map(|(season_number, mut episodes)| {
                episodes.sort_by_key(|e| e.episode_number);
                Season {
                    season_number,
                    episodes,
                }
            })
            .collect();

        seasons.sort_by_key(|s| s.season_number);
        seasons
    }

    /// Extracts a four-digit year from an ISO date string like "2008-01-20".
    fn extract_year(premiered: &str) -> Option<u16> {
        premiered
            .split('-')
            .next()
            .and_then(|year_str| year_str.parse().ok())
    }
}

impl MetadataProvider for TvMazeProvider {
    fn search_series(
        &self,
        series_name: &str,
    ) -> Result<Vec<SeriesCandidate>, MetadataRetrievalError> {
        let url = format!("{}/search/shows", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("q", series_name)])
            .send()
            .map_err(|e| MetadataRetrievalError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MetadataRetrievalError::RequestError(format!(
                "HTTP {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let results: Vec<TvMazeSearchResult> = response
            .json()
            .map_err(|e| MetadataRetrievalError::ParseError(e.to_string()))?;

        // The search endpoint returns results sorted by score descending.
        // Take only the top N candidates.
        let candidates: Vec<SeriesCandidate> = results
            .into_iter()
            .take(MAX_CANDIDATES)
            .map(|result| SeriesCandidate {
                id: result.show.id,
                name: result.show.name,
                year: result.show.premiered.as_deref().and_then(Self::extract_year),
            })
            .collect();

        if candidates.is_empty() {
            return Err(MetadataRetrievalError::SeriesNotFound(
                series_name.to_string(),
            ));
        }

        Ok(candidates)
    }

    fn fetch_series(
        &self,
        candidate: &SeriesCandidate,
        season_numbers: Option<Vec<usize>>,
    ) -> Result<TVSeries, MetadataRetrievalError> {
        let url = format!("{}/shows/{}/episodes", self.base_url, candidate.id);

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| MetadataRetrievalError::RequestError(e.to_string()))?;

        if response.status() == 404 {
            return Err(MetadataRetrievalError::SeriesNotFound(
                candidate.name.clone(),
            ));
        }

        if !response.status().is_success() {
            return Err(MetadataRetrievalError::RequestError(format!(
                "HTTP {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let episodes: Vec<TvMazeEpisode> = response
            .json()
            .map_err(|e| MetadataRetrievalError::ParseError(e.to_string()))?;

        let seasons = Self::group_into_seasons(episodes, season_numbers);

        Ok(TVSeries {
            name: candidate.name.clone(),
            seasons,
        })
    }
}
