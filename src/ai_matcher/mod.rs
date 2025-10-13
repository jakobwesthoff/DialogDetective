//! AI-based matching module
//!
//! This module provides functionality to match video files with TV series episodes
//! using AI/LLM-based analysis. It generates prompts for language models to help solve
//! the mystery of which episode a video file belongs to.

mod claude_code;
mod gemini_cli;

pub(crate) use claude_code::ClaudeCodeMatcher;
pub(crate) use gemini_cli::GeminiCliMatcher;

use crate::metadata_retrieval::{Episode, TVSeries};
use crate::speech_to_text::Transcript;
use thiserror::Error;

/// Errors that can occur during episode matching
#[derive(Debug, Error)]
pub enum EpisodeMatchingError {
    /// Failed to communicate with AI service
    #[error("AI service error: {0}")]
    ServiceError(String),

    /// Failed to parse the AI's response
    #[error("Failed to parse AI response: {reason}\n\nFull LLM response:\n{response}")]
    ParseError { reason: String, response: String },

    /// No matching episode could be determined
    #[error("No matching episode found in the series\n\nFull LLM response:\n{response}")]
    NoMatchFound { response: String },
}

/// Trait for matching transcripts to episodes using AI/LLM analysis
///
/// Implementors of this trait orchestrate the complete matching process:
/// generating prompts, sending them to LLMs, parsing responses, and
/// identifying which episode a transcript belongs to.
pub(crate) trait EpisodeMatcher {
    /// Matches a transcript to an episode from the given series
    ///
    /// This method uses AI/LLM analysis to determine which episode
    /// best matches the provided transcript by analyzing dialogue content.
    ///
    /// # Arguments
    ///
    /// * `transcript` - The audio transcript from the video file
    /// * `series` - The TV series with all candidate episodes
    ///
    /// # Returns
    ///
    /// The episode that best matches the transcript
    ///
    /// # Errors
    ///
    /// Returns an error if the AI service fails, the response cannot be parsed,
    /// or no suitable match can be found.
    fn match_episode(
        &self,
        transcript: &Transcript,
        series: &TVSeries,
    ) -> Result<Episode, EpisodeMatchingError>;
}

/// Trait for generating prompts for LLM-based episode matching
///
/// Implementors of this trait take transcript data and episode metadata
/// to construct effective prompts that help the LLM solve the mystery
/// of which episode the video belongs to.
pub(crate) trait SinglePromptGenerator {
    /// Generates a prompt for matching a transcript against episodes in a series
    ///
    /// This prompt asks the LLM to analyze the transcript and identify which
    /// episode from the series it most likely belongs to.
    ///
    /// # Arguments
    ///
    /// * `transcript` - The audio transcript from the video file
    /// * `series` - The complete TV series with all episodes
    ///
    /// # Returns
    ///
    /// A formatted prompt string ready to send to an LLM
    fn generate_single_prompt(&self, transcript: &Transcript, series: &TVSeries) -> String;
}

/// A naive prompt generator implementation
///
/// This generator creates straightforward prompts that instruct the LLM
/// to match transcripts to episodes and return results in JSON format.
pub(crate) struct NaivePromptGenerator;

impl Default for NaivePromptGenerator {
    fn default() -> Self {
        Self
    }
}

impl SinglePromptGenerator for NaivePromptGenerator {
    fn generate_single_prompt(&self, transcript: &Transcript, series: &TVSeries) -> String {
        let mut prompt = String::new();

        // Add JSON format instructions
        prompt.push_str("IMPORTANT: Your output to the following MUST be JSON in the FORMAT ");
        prompt.push_str(r#"{"season": XX, "episode": YY}. "#);
        prompt
            .push_str("NOTHING ELSE IS TO BE RETURNED. ONLY EVER ANSWER WITH THIS JSON Structure.");
        prompt.push_str("The JSON is to be encapsulated in a markdown jsonblock ```json\n\n");

        // Add task description
        prompt.push_str("Using this structure answer the following question:\n");
        prompt.push_str("Based on the given Transcript of a tv series episode as well as a List of possible episode candidates ");
        prompt.push_str(
            "identified by their Season number, Episode number, title and short summary, ",
        );
        prompt.push_str("match the transcript to the best fitting short summary, to identify which episode the given transcript belongs to.\n\n");

        // Add reflection instruction
        prompt.push_str("Ultrathink about this and reflect on your reasoning, before providing ONLY THE REQUESTED ANSWER FORMAT.\n\n");

        // Add data header
        prompt.push_str("Here follows the mentioned data:\n\n");

        // Add transcript section
        prompt.push_str("=== TRANSCRIPT ===\n");
        prompt.push_str(&format!("Language: {}\n\n", transcript.language));
        prompt.push_str(&transcript.text);
        prompt.push_str("\n\n");

        // Add episode candidates section
        prompt.push_str(&format!(
            "=== EPISODE CANDIDATES FOR '{}' ===\n\n",
            series.name
        ));

        for season in &series.seasons {
            prompt.push_str(&format!("--- SEASON {} ---\n", season.season_number));

            for episode in &season.episodes {
                prompt.push_str(&format!(
                    "Season: {}, Episode: {} - {}\n",
                    episode.season_number, episode.episode_number, episode.name
                ));
                prompt.push_str(&format!("Summary: {}\n\n", episode.summary));
            }
        }

        prompt
    }
}
