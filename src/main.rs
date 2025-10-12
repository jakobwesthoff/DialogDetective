use dialog_detective::{investigate_case, ProgressEvent};
use std::env;
use std::path::Path;
use std::process;

/// Handles progress events and prints formatted output to stdout
fn handle_progress_event(event: ProgressEvent) {
    match event {
        ProgressEvent::Started { directory, show_name } => {
            println!(
                "DialogDetective reporting: Starting investigation in {} for {}...",
                directory.display(),
                show_name
            );
        }
        ProgressEvent::FetchingMetadata { show_name } => {
            println!("\n=== Fetching Episode Metadata ===");
            println!("Retrieving episode information for '{}'...", show_name);
        }
        ProgressEvent::MetadataFetched {
            series_name,
            season_count,
        } => {
            println!("Found {} season(s) for '{}'\n", season_count, series_name);
        }
        ProgressEvent::ScanningVideos => {
            println!("\nScanning for video files...");
        }
        ProgressEvent::VideosFound { count } => {
            if count == 0 {
                println!("No video files found.");
            } else {
                println!("Found {} video file(s)\n", count);
            }
        }
        ProgressEvent::ProcessingVideo {
            index,
            total,
            video_path,
        } => {
            println!("[{}/{}] Processing: {}", index + 1, total, video_path.display());
        }
        ProgressEvent::ExtractingAudio { .. } => {
            println!("  Extracting audio...");
        }
        ProgressEvent::TranscribingAudio { .. } => {
            println!("  Transcribing audio...");
        }
        ProgressEvent::TranscriptionComplete {
            language, text, ..
        } => {
            println!("  Language: {}", language);
            println!("  Transcript:\n{}\n", text);
        }
        ProgressEvent::TranscriptionPhaseComplete { video_count } => {
            println!(
                "\nTranscription complete! Processed {} video(s).",
                video_count
            );
        }
        ProgressEvent::MatchingEpisodes => {
            println!("\n=== Matching Episodes ===");
        }
        ProgressEvent::MatchingVideo {
            index,
            total,
            video_path,
        } => {
            println!("[{}/{}] Matching: {}", index + 1, total, video_path.display());
        }
        ProgressEvent::Complete { match_count } => {
            println!("\nInvestigation complete! Matched {} video(s).", match_count);
        }
    }
}

fn main() {
    // Get arguments from command line
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: {} <directory> <model_path> <show_name>", args[0]);
        eprintln!(
            "\nExample: {} /path/to/videos models/ggml-base.bin \"Breaking Bad\"",
            args[0]
        );
        process::exit(1);
    }

    let directory = Path::new(&args[1]);
    let model_path = Path::new(&args[2]);
    let show_name = &args[3];

    // Check if directory exists
    if !directory.exists() {
        eprintln!("Error: Directory does not exist: {}", directory.display());
        process::exit(1);
    }

    if !directory.is_dir() {
        eprintln!("Error: Path is not a directory: {}", directory.display());
        process::exit(1);
    }

    // Check if model file exists
    if !model_path.exists() {
        eprintln!("Error: Model file does not exist: {}", model_path.display());
        process::exit(1);
    }

    if !model_path.is_file() {
        eprintln!("Error: Model path is not a file: {}", model_path.display());
        process::exit(1);
    }

    // Run the investigation with progress callback
    match investigate_case(directory, model_path, show_name, handle_progress_event) {
        Ok(matches) => {
            // Print results
            println!("\n=== Match Results ===\n");

            if matches.is_empty() {
                println!("No matches found.");
                return;
            }

            for (index, match_result) in matches.iter().enumerate() {
                println!("Match #{}", index + 1);
                println!("  Video: {}", match_result.video.path.display());
                println!(
                    "  Episode: S{:02}E{:02} - {}",
                    match_result.episode.season_number,
                    match_result.episode.episode_number,
                    match_result.episode.name
                );
                println!("  Summary: {}", match_result.episode.summary);
                println!();
            }

            println!("Successfully matched {} video(s)!", matches.len());
        }
        Err(e) => {
            eprintln!("\nError during investigation: {}", e);
            process::exit(1);
        }
    }
}
