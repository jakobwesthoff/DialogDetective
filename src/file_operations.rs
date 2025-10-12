use crate::{Episode, MatchResult};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during file operations
#[derive(Debug, Error)]
pub enum FileOperationError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid format string: {0}")]
    InvalidFormat(String),

    #[error("Missing file extension for: {0}")]
    MissingExtension(String),
}

/// Represents a planned file operation (rename or copy)
#[derive(Debug, Clone)]
pub struct PlannedOperation {
    /// Source file path
    pub source: PathBuf,
    /// Destination file path (relative to output dir for copy, or absolute for rename)
    pub destination: PathBuf,
    /// Original episode matched (for display)
    pub episode: Episode,
    /// Duplicate suffix applied (if any)
    pub duplicate_suffix: Option<usize>,
}

/// Sanitizes a string for use in filenames by replacing problematic characters
///
/// Replaces characters that are invalid or problematic in filenames across platforms:
/// - Path separators: / \
/// - Reserved characters: : * ? " < > |
/// - Control characters
/// - Trim leading/trailing whitespace and dots
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();

    // Trim whitespace and dots from start/end
    sanitized.trim_matches(|c: char| c.is_whitespace() || c == '.').to_string()
}

/// Formats a filename based on a format string and episode information
///
/// Supported placeholders:
/// - `{show}` - Series name
/// - `{season}` or `{season:NN}` - Season number with optional zero-padding
/// - `{episode}` or `{episode:NN}` - Episode number with optional zero-padding
/// - `{title}` - Episode title (sanitized)
/// - `{ext}` - File extension (without dot)
///
/// # Examples
///
/// ```
/// let result = format_filename(
///     "{show} - S{season:02}E{episode:02} - {title}.{ext}",
///     "Breaking Bad",
///     1,
///     2,
///     "Cat's in the Bag...",
///     "mp4"
/// );
/// assert_eq!(result, "Breaking Bad - S01E02 - Cat's in the Bag....mp4");
/// ```
pub fn format_filename(
    format: &str,
    show_name: &str,
    season: usize,
    episode: usize,
    title: &str,
    extension: &str,
) -> String {
    let sanitized_title = sanitize_filename(title);
    let sanitized_show = sanitize_filename(show_name);

    let mut result = format.to_string();

    // Replace {show}
    result = result.replace("{show}", &sanitized_show);

    // Replace {season} and {season:NN}
    result = replace_with_padding(&result, "season", season);

    // Replace {episode} and {episode:NN}
    result = replace_with_padding(&result, "episode", episode);

    // Replace {title}
    result = result.replace("{title}", &sanitized_title);

    // Replace {ext}
    result = result.replace("{ext}", extension);

    result
}

/// Helper function to replace placeholders with optional zero-padding
///
/// Handles both `{name}` and `{name:NN}` where NN is the padding width
fn replace_with_padding(text: &str, name: &str, value: usize) -> String {
    let mut result = text.to_string();

    // First, handle padded versions like {season:02}
    let pattern_start = format!("{{{name}:");
    while let Some(start) = result.find(&pattern_start) {
        if let Some(end) = result[start..].find('}') {
            let full_pattern = &result[start..start + end + 1];
            let padding_str = &full_pattern[pattern_start.len()..full_pattern.len() - 1];

            if let Ok(width) = padding_str.parse::<usize>() {
                let formatted = format!("{:0width$}", value, width = width);
                result = result.replace(full_pattern, &formatted);
            }
        } else {
            break;
        }
    }

    // Then handle unpadded version like {season}
    let simple_pattern = format!("{{{name}}}");
    result = result.replace(&simple_pattern, &value.to_string());

    result
}

/// Groups match results by episode and detects duplicates
///
/// Returns a HashMap where keys are (season, episode) tuples and values are
/// vectors of match results for that episode.
pub fn detect_duplicates(matches: &[MatchResult]) -> HashMap<(usize, usize), Vec<MatchResult>> {
    let mut groups: HashMap<(usize, usize), Vec<MatchResult>> = HashMap::new();

    for match_result in matches {
        let key = (
            match_result.episode.season_number,
            match_result.episode.episode_number,
        );
        groups.entry(key).or_insert_with(Vec::new).push(match_result.clone());
    }

    groups
}

/// Plans file operations with duplicate handling via suffix strategy
///
/// For duplicate episodes, adds numeric suffix starting from 2:
/// - First occurrence: `name.ext`
/// - Second occurrence: `name (2).ext`
/// - Third occurrence: `name (3).ext`
pub fn plan_operations(
    matches: &[MatchResult],
    show_name: &str,
    format: &str,
    output_dir: Option<&Path>,
) -> Result<Vec<PlannedOperation>, FileOperationError> {
    let groups = detect_duplicates(matches);
    let mut operations = Vec::new();

    for match_result in matches {
        let key = (
            match_result.episode.season_number,
            match_result.episode.episode_number,
        );

        // Get the extension from the source file
        let extension = match_result
            .video
            .path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| {
                FileOperationError::MissingExtension(
                    match_result.video.path.display().to_string(),
                )
            })?;

        // Generate base filename
        let base_name = format_filename(
            format,
            show_name,
            match_result.episode.season_number,
            match_result.episode.episode_number,
            &match_result.episode.name,
            extension,
        );

        // Determine if this is a duplicate and which occurrence
        let group = &groups[&key];
        let (final_name, suffix) = if group.len() > 1 {
            // Find which occurrence this is
            let occurrence = group
                .iter()
                .position(|m| m.video.path == match_result.video.path)
                .unwrap_or(0);

            if occurrence == 0 {
                // First occurrence, no suffix
                (base_name.clone(), None)
            } else {
                // Add suffix (2), (3), etc.
                let suffix_num = occurrence + 1;
                let name_without_ext = base_name
                    .strip_suffix(&format!(".{}", extension))
                    .unwrap_or(&base_name);
                let suffixed = format!("{} ({}).{}", name_without_ext, suffix_num, extension);
                (suffixed, Some(suffix_num))
            }
        } else {
            // Not a duplicate
            (base_name, None)
        };

        // Determine destination path
        let destination = if let Some(output) = output_dir {
            output.join(&final_name)
        } else {
            // For rename mode, destination is in same directory as source
            match_result
                .video
                .path
                .parent()
                .map(|p| p.join(&final_name))
                .unwrap_or_else(|| PathBuf::from(&final_name))
        };

        operations.push(PlannedOperation {
            source: match_result.video.path.clone(),
            destination,
            episode: match_result.episode.clone(),
            duplicate_suffix: suffix,
        });
    }

    Ok(operations)
}

/// Executes rename operations in place
pub fn execute_rename(operations: &[PlannedOperation]) -> Result<Vec<io::Error>, FileOperationError> {
    let mut errors = Vec::new();

    for op in operations {
        if let Err(e) = fs::rename(&op.source, &op.destination) {
            errors.push(e);
        }
    }

    Ok(errors)
}

/// Executes copy operations to output directory
///
/// Creates the output directory if it doesn't exist.
pub fn execute_copy(
    operations: &[PlannedOperation],
    output_dir: &Path,
) -> Result<Vec<io::Error>, FileOperationError> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let mut errors = Vec::new();

    for op in operations {
        if let Err(e) = fs::copy(&op.source, &op.destination) {
            errors.push(e);
        }
    }

    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Normal Title"), "Normal Title");
        assert_eq!(sanitize_filename("Title: With Colon"), "Title- With Colon");
        assert_eq!(sanitize_filename("Path/With\\Slashes"), "Path-With-Slashes");
        assert_eq!(sanitize_filename("  Spaces  "), "Spaces");
        assert_eq!(sanitize_filename("...dots..."), "dots");
    }

    #[test]
    fn test_format_filename() {
        let result = format_filename(
            "{show} - S{season:02}E{episode:02} - {title}.{ext}",
            "Breaking Bad",
            1,
            2,
            "Cat's in the Bag...",
            "mp4",
        );
        // Trailing dots are trimmed by sanitize_filename
        assert_eq!(result, "Breaking Bad - S01E02 - Cat's in the Bag.mp4");

        let result2 = format_filename(
            "{show} S{season}E{episode} {title}.{ext}",
            "Game of Thrones",
            3,
            9,
            "The Rains of Castamere",
            "mkv",
        );
        assert_eq!(result2, "Game of Thrones S3E9 The Rains of Castamere.mkv");
    }

    #[test]
    fn test_replace_with_padding() {
        assert_eq!(replace_with_padding("S{season:02}E{episode:02}", "season", 1), "S01E{episode:02}");
        assert_eq!(replace_with_padding("S01E{episode:02}", "episode", 2), "S01E02");
        assert_eq!(replace_with_padding("Season {season}", "season", 5), "Season 5");
    }
}
