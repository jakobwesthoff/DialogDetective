use clap::{Parser, ValueEnum};
use dialog_detective::{
    MatcherType, ProgressEvent, execute_copy, execute_rename, investigate_case, plan_operations,
};
use std::path::PathBuf;
use std::process;

/// DialogDetective - Automatically identify and rename unknown video files
///
/// This tool analyzes video files by extracting audio, transcribing speech,
/// and using AI to match the content to TV series episodes.
#[derive(Parser)]
#[command(name = "dialog_detective")]
#[command(version, about, long_about = None)]
#[command(
    after_help = "💡 TIP: Use --season to filter episodes for faster, cheaper, more accurate matching!"
)]
struct Cli {
    /// Directory containing video files to process
    video_dir: PathBuf,

    /// Path to Whisper model file (e.g., ggml-base.en.bin)
    model_path: PathBuf,

    /// Name of the TV series (e.g., "Breaking Bad")
    show_name: String,

    /// Filter to specific season(s) - can be repeated (RECOMMENDED)
    ///
    /// Using season filtering speeds up matching, reduces token usage,
    /// and improves accuracy by providing more focused context to the AI.
    #[arg(short, long = "season", value_name = "N")]
    seasons: Vec<usize>,

    /// AI backend to use for episode matching
    #[arg(short = 'm', long, value_enum, default_value_t = Matcher::Gemini)]
    matcher: Matcher,

    /// Operation mode: what to do after matching
    #[arg(long, value_enum, default_value_t = Mode::DryRun)]
    mode: Mode,

    /// Output directory for copy mode (required when mode=copy)
    #[arg(short = 'o', long, value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// File naming format
    ///
    /// Supported variables:
    ///   {show}    - Series name
    ///   {season}  - Season number (use {season:02} for zero-padding)
    ///   {episode} - Episode number (use {episode:02} for zero-padding)
    ///   {title}   - Episode title
    ///   {ext}     - Original file extension
    #[arg(
        long,
        default_value = "{show} - S{season:02}E{episode:02} - {title}.{ext}"
    )]
    format: String,
}

/// AI backend selection
#[derive(Clone, Copy, ValueEnum)]
enum Matcher {
    /// Gemini CLI (default, requires 'gemini' in PATH)
    Gemini,
    /// Claude Code CLI (requires 'claude' in PATH)
    Claude,
}

impl From<Matcher> for MatcherType {
    fn from(m: Matcher) -> Self {
        match m {
            Matcher::Gemini => MatcherType::Gemini,
            Matcher::Claude => MatcherType::Claude,
        }
    }
}

/// Operation mode
#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    /// Show what would happen without making changes (default)
    DryRun,
    /// Rename files in place
    Rename,
    /// Copy files to output directory with new names
    Copy,
}

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
        ProgressEvent::Hashing { .. } => {
            print!("   ├─ Computing hash... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::AudioExtraction { .. } => {
            print!("   ├─ Extracting audio... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::Transcription { .. } => {
            print!("   ├─ Transcribing... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::TranscriptionFinished { language, .. } => {
            println!("✓ ({})", language);
        }
        ProgressEvent::TranscriptCacheHit { language, .. } => {
            println!("   ├─ Transcript cached... ✓ ({})", language);
        }
        ProgressEvent::Matching { .. } => {
            print!("   └─ Matching episode... ");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
        ProgressEvent::MatchingCacheHit { episode, .. } => {
            println!(
                "   └─ Match cached... ✓ (S{:02}E{:02} - {})",
                episode.season_number, episode.episode_number, episode.name
            );
        }
        ProgressEvent::HashingFinished { .. }
        | ProgressEvent::AudioExtractionFinished { .. }
        | ProgressEvent::MatchingFinished { .. } => {
            println!("✓");
        }
        ProgressEvent::Complete { .. } => {
            println!("✓\n");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
    }
}

fn main() {
    let cli = Cli::parse();

    // Validate arguments
    if !cli.video_dir.exists() {
        eprintln!(
            "❌ Error: Directory does not exist: {}",
            cli.video_dir.display()
        );
        process::exit(1);
    }

    if !cli.video_dir.is_dir() {
        eprintln!(
            "❌ Error: Path is not a directory: {}",
            cli.video_dir.display()
        );
        process::exit(1);
    }

    if !cli.model_path.exists() {
        eprintln!(
            "❌ Error: Model file does not exist: {}",
            cli.model_path.display()
        );
        process::exit(1);
    }

    if !cli.model_path.is_file() {
        eprintln!(
            "❌ Error: Model path is not a file: {}",
            cli.model_path.display()
        );
        process::exit(1);
    }

    // Validate mode-specific requirements
    if matches!(cli.mode, Mode::Copy) && cli.output_dir.is_none() {
        eprintln!("❌ Error: --output-dir is required when using --mode copy");
        process::exit(1);
    }

    // Convert seasons filter
    let season_filter = if cli.seasons.is_empty() {
        None
    } else {
        Some(cli.seasons.clone())
    };

    // Run the investigation with progress callback
    match investigate_case(
        &cli.video_dir,
        &cli.model_path,
        &cli.show_name,
        season_filter,
        cli.matcher.into(),
        handle_progress_event,
    ) {
        Ok(matches) => {
            if matches.is_empty() {
                println!("❌ Case closed: No matches found");
                return;
            }

            // Plan file operations
            let output_dir = cli.output_dir.as_deref();
            let operations =
                match plan_operations(&matches, &cli.show_name, &cli.format, output_dir) {
                    Ok(ops) => ops,
                    Err(e) => {
                        eprintln!("\n❌ Failed to plan operations: {}", e);
                        process::exit(1);
                    }
                };

            // Display results based on mode
            match cli.mode {
                Mode::DryRun => {
                    println!("📋 Dry Run - No files will be modified:");
                    println!();

                    for op in &operations {
                        let source_name = op
                            .source
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");
                        let dest_name = op
                            .destination
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");

                        let operation_type = if output_dir.is_some() {
                            "COPY"
                        } else {
                            "RENAME"
                        };

                        if let Some(suffix) = op.duplicate_suffix {
                            println!(
                                "  [{}] {} → {} (duplicate #{})",
                                operation_type, source_name, dest_name, suffix
                            );
                        } else {
                            println!("  [{}] {} → {}", operation_type, source_name, dest_name);
                        }
                        println!(
                            "         S{:02}E{:02} - {}",
                            op.episode.season_number, op.episode.episode_number, op.episode.name
                        );
                        println!();
                    }

                    println!("💡 Use --mode rename or --mode copy to apply these changes");
                }

                Mode::Rename => {
                    println!("📝 Renaming files...");
                    println!();

                    match execute_rename(&operations) {
                        Ok(errors) if errors.is_empty() => {
                            for op in &operations {
                                let source_name = op
                                    .source
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");
                                let dest_name = op
                                    .destination
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");

                                println!("  ✓ {} → {}", source_name, dest_name);
                            }
                            println!();
                            println!("✅ Successfully renamed {} file(s)", operations.len());
                        }
                        Ok(errors) => {
                            let success_count = operations.len() - errors.len();

                            println!("⚠️  Operation completed with errors:");
                            println!();
                            println!("✅ Successfully renamed {} file(s)", success_count);
                            println!("❌ Failed to rename {} file(s):", errors.len());

                            for (op, error) in operations.iter().zip(errors.iter()) {
                                let source_name = op
                                    .source
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");
                                println!("  ✗ {} - {}", source_name, error);
                            }

                            process::exit(1);
                        }
                        Err(e) => {
                            eprintln!("\n❌ Rename operation failed: {}", e);
                            process::exit(1);
                        }
                    }
                }

                Mode::Copy => {
                    let output = cli.output_dir.as_ref().unwrap(); // Safe unwrap, validated earlier
                    println!("📦 Copying files to {}...", output.display());
                    println!();

                    match execute_copy(&operations, output) {
                        Ok(errors) if errors.is_empty() => {
                            for op in &operations {
                                let source_name = op
                                    .source
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");
                                let dest_name = op
                                    .destination
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");

                                println!("  ✓ {} → {}", source_name, dest_name);
                            }
                            println!();
                            println!(
                                "✅ Successfully copied {} file(s) to {}",
                                operations.len(),
                                output.display()
                            );
                        }
                        Ok(errors) => {
                            let success_count = operations.len() - errors.len();

                            println!("⚠️  Operation completed with errors:");
                            println!();
                            println!("✅ Successfully copied {} file(s)", success_count);
                            println!("❌ Failed to copy {} file(s):", errors.len());

                            for (op, error) in operations.iter().zip(errors.iter()) {
                                let source_name = op
                                    .source
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");
                                println!("  ✗ {} - {}", source_name, error);
                            }

                            process::exit(1);
                        }
                        Err(e) => {
                            eprintln!("\n❌ Copy operation failed: {}", e);
                            process::exit(1);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("\n❌ Investigation failed: {}", e);
            process::exit(1);
        }
    }
}
