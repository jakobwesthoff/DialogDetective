use dialog_detective::investigate_case;
use std::env;
use std::path::Path;
use std::process;

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

    // Run the investigation
    match investigate_case(directory, model_path, show_name) {
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
