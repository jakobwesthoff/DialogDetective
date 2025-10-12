//! Claude Code CLI-based episode matcher
//!
//! This module provides an implementation of the EpisodeMatcher trait that uses
//! the Claude Code CLI to match transcripts to episodes.

use super::{EpisodeMatcher, EpisodeMatchingError, SinglePromptGenerator};
use crate::metadata_retrieval::{Episode, TVSeries};
use crate::speech_to_text::Transcript;
use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};

/// JSON response format expected from Claude Code CLI
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    season: usize,
    episode: usize,
}

/// Episode matcher using Claude Code CLI
///
/// This matcher generates prompts using a SinglePromptGenerator and sends them
/// to the Claude Code CLI for analysis. It parses the JSON response to identify
/// the matching episode.
pub(crate) struct ClaudeCodeMatcher<G: SinglePromptGenerator> {
    /// The prompt generator to use for creating prompts
    generator: G,
}

impl<G: SinglePromptGenerator> ClaudeCodeMatcher<G> {
    /// Creates a new ClaudeCodeMatcher with the given prompt generator
    pub fn new(generator: G) -> Self {
        Self { generator }
    }

    /// Checks if the claude CLI is installed and available
    fn is_claude_installed() -> bool {
        Command::new("claude")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Sends a prompt to Claude Code CLI and returns the response
    fn call_claude(prompt: &str) -> Result<String, EpisodeMatchingError> {
        // Check if claude is installed
        if !Self::is_claude_installed() {
            return Err(EpisodeMatchingError::ServiceError(
                "Claude CLI not found. Please install it first.".to_string(),
            ));
        }

        // Spawn claude process with stdin
        let mut child = Command::new("claude")
            .arg("-p")
            .arg("--output-format")
            .arg("text")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                EpisodeMatchingError::ServiceError(format!("Failed to spawn claude CLI: {}", e))
            })?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).map_err(|e| {
                EpisodeMatchingError::ServiceError(format!(
                    "Failed to write to claude stdin: {}",
                    e
                ))
            })?;
        }

        // Wait for completion and capture output
        let output = child.wait_with_output().map_err(|e| {
            EpisodeMatchingError::ServiceError(format!("Failed to read claude output: {}", e))
        })?;

        // Check exit code
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EpisodeMatchingError::ServiceError(format!(
                "Claude CLI failed with exit code {:?}: {}",
                output.status.code(),
                stderr
            )));
        }

        // Convert stdout to string
        String::from_utf8(output.stdout).map_err(|e| {
            EpisodeMatchingError::ParseError(format!("Invalid UTF-8 in claude response: {}", e))
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

impl<G: SinglePromptGenerator> EpisodeMatcher for ClaudeCodeMatcher<G> {
    fn match_episode(
        &self,
        transcript: &Transcript,
        series: &TVSeries,
    ) -> Result<Episode, EpisodeMatchingError> {
        // Generate the prompt
        let prompt = self.generator.generate_single_prompt(transcript, series);

        // Call Claude CLI
        let response = Self::call_claude(&prompt)?;

        // Extract JSON block
        let json_str = Self::extract_json_block(&response)?;

        // Parse JSON
        let claude_response: ClaudeResponse = serde_json::from_str(&json_str).map_err(|e| {
            EpisodeMatchingError::ParseError(format!("Failed to parse JSON response: {}", e))
        })?;

        // Find matching episode
        Self::find_episode(series, claude_response.season, claude_response.episode)
    }
}
