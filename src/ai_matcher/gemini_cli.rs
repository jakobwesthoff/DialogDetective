//! Gemini CLI-based episode matcher
//!
//! This module provides an implementation of the EpisodeMatcher trait that uses
//! the Gemini CLI to match transcripts to episodes.

use super::{EpisodeMatcher, EpisodeMatchingError, SinglePromptGenerator};
use crate::metadata_retrieval::{Episode, TVSeries};
use crate::speech_to_text::Transcript;
use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};

/// JSON response format expected from Gemini CLI
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    season: usize,
    episode: usize,
}

/// Episode matcher using Gemini CLI
///
/// This matcher generates prompts using a SinglePromptGenerator and sends them
/// to the Gemini CLI for analysis. It parses the JSON response to identify
/// the matching episode.
pub(crate) struct GeminiCliMatcher<G: SinglePromptGenerator> {
    /// The prompt generator to use for creating prompts
    generator: G,
}

impl<G: SinglePromptGenerator> GeminiCliMatcher<G> {
    /// Creates a new GeminiCliMatcher with the given prompt generator
    pub fn new(generator: G) -> Self {
        Self { generator }
    }

    /// Checks if the gemini CLI is installed and available
    fn is_gemini_installed() -> bool {
        Command::new("gemini")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Sends a prompt to Gemini CLI and returns the response
    fn call_gemini(prompt: &str) -> Result<String, EpisodeMatchingError> {
        // Check if gemini is installed
        if !Self::is_gemini_installed() {
            return Err(EpisodeMatchingError::ServiceError(
                "Gemini CLI not found. Please install it first.".to_string(),
            ));
        }

        // Spawn gemini process with stdin
        let mut child = Command::new("gemini")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                EpisodeMatchingError::ServiceError(format!("Failed to spawn gemini CLI: {}", e))
            })?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).map_err(|e| {
                EpisodeMatchingError::ServiceError(format!(
                    "Failed to write to gemini stdin: {}",
                    e
                ))
            })?;
        }

        // Wait for completion and capture output
        let output = child.wait_with_output().map_err(|e| {
            EpisodeMatchingError::ServiceError(format!("Failed to read gemini output: {}", e))
        })?;

        // Check exit code
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EpisodeMatchingError::ServiceError(format!(
                "Gemini CLI failed with exit code {:?}: {}",
                output.status.code(),
                stderr
            )));
        }

        // Convert stdout to string
        String::from_utf8(output.stdout).map_err(|e| {
            EpisodeMatchingError::ParseError(format!("Invalid UTF-8 in gemini response: {}", e))
        })
    }

    /// Extracts JSON from markdown code fence (```json ... ```)
    fn extract_json_block(response: &str) -> Result<String, EpisodeMatchingError> {
        // Look for ```json ... ``` block
        let start_marker = "```json";
        let end_marker = "```";

        if let Some(start_pos) = response.find(start_marker) {
            let json_start = start_pos + start_marker.len();
            let remaining = &response[json_start..];

            if let Some(end_pos) = remaining.find(end_marker) {
                let json_str = remaining[..end_pos].trim();
                return Ok(json_str.to_string());
            }
        }

        Err(EpisodeMatchingError::ParseError(
            "No JSON code block found in response".to_string(),
        ))
    }

    /// Finds an episode in the series by season and episode number
    fn find_episode(
        series: &TVSeries,
        season_num: usize,
        episode_num: usize,
    ) -> Result<Episode, EpisodeMatchingError> {
        for season in &series.seasons {
            if season.season_number == season_num {
                for episode in &season.episodes {
                    if episode.episode_number == episode_num {
                        return Ok(episode.clone());
                    }
                }
            }
        }

        Err(EpisodeMatchingError::NoMatchFound)
    }
}

impl<G: SinglePromptGenerator> EpisodeMatcher for GeminiCliMatcher<G> {
    fn match_episode(
        &self,
        transcript: &Transcript,
        series: &TVSeries,
    ) -> Result<Episode, EpisodeMatchingError> {
        // Generate the prompt
        let prompt = self.generator.generate_single_prompt(transcript, series);

        // Call Gemini CLI
        let response = Self::call_gemini(&prompt)?;

        // Extract JSON block
        let json_str = Self::extract_json_block(&response)?;

        // Parse JSON
        let gemini_response: GeminiResponse = serde_json::from_str(&json_str).map_err(|e| {
            EpisodeMatchingError::ParseError(format!("Failed to parse JSON response: {}", e))
        })?;

        // Find matching episode
        Self::find_episode(series, gemini_response.season, gemini_response.episode)
    }
}
