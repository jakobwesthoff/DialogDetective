use dialog_detective::{ProgressEvent, investigate_case};
use std::env;
use std::path::Path;
use std::process;

/// Handles progress events and prints formatted output to stdout
fn handle_progress_event(event: ProgressEvent) {
    match event {
        ProgressEvent::Started { show_name, .. } => {
            println!("🔍 DialogDetective");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("📺 Investigating: {}", show_name);
        }
        ProgressEvent::FetchingMetadata { .. } => {
            print!("📡 Fetching metadata... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::MetadataFetched { season_count, .. } => {
            println!("✓ ({} seasons)", season_count);
        }
        ProgressEvent::ScanningVideos => {
            print!("🔎 Scanning directory... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::VideosFound { count } => {
            if count == 0 {
                println!("✗ No videos found");
            } else {
                println!("✓ ({} files)", count);
                println!();
            }
        }
        ProgressEvent::ProcessingVideo {
            index,
            total,
            video_path,
        } => {
            let filename = video_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            println!("🎬 [{}/{}] {}", index + 1, total, filename);
        }
        ProgressEvent::ExtractingAudio { .. } => {
            print!("   ├─ Extracting audio... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::TranscribingAudio { .. } => {
            println!("✓");
            print!("   ├─ Transcribing... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::TranscriptionComplete { language, .. } => {
            println!("✓ ({})", language);
        }
        ProgressEvent::MatchingVideo { .. } => {
            print!("   └─ Matching episode... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::Complete { .. } => {
            println!("✓\n");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
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
            if matches.is_empty() {
                println!("❌ Case closed: No matches found");
                return;
            }

            println!("📋 Results:");
            println!();

            for match_result in matches.iter() {
                let filename = match_result
                    .video
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                println!(
                    "  {} ➜ S{:02}E{:02} - {}",
                    filename,
                    match_result.episode.season_number,
                    match_result.episode.episode_number,
                    match_result.episode.name
                );
            }

            println!();
            println!("✅ Case solved: {} video(s) identified", matches.len());
        }
        Err(e) => {
            eprintln!("\n❌ Investigation failed: {}", e);
            process::exit(1);
        }
    }
}
