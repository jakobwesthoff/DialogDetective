use clap::{Parser, ValueEnum};
use dialog_detective::{
    DialogDetectiveError, MatcherType, ProgressEvent, SeriesCandidate, execute_copy,
    execute_rename, investigate_case, model_downloader, plan_operations,
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
    #[arg(required_unless_present = "list_models")]
    video_dir: Option<PathBuf>,

    /// Name of the TV series (e.g., "Breaking Bad")
    #[arg(required_unless_present = "list_models")]
    show_name: Option<String>,

    /// List all available Whisper models and exit
    #[arg(long)]
    list_models: bool,

    /// Select Whisper model by name (auto-downloads if needed)
    ///
    /// By default, the 'base' model is used. Use this flag to select a different
    /// model from the supported list. Use --list-models to see all available models.
    ///
    /// Examples: tiny, base, small, medium, large-v3-turbo, base-q8_0
    #[arg(long, value_name = "NAME", conflicts_with = "model_path")]
    model: Option<String>,

    /// Override with custom model file path (advanced)
    ///
    /// Use this flag to specify a custom model file path instead of using
    /// the auto-download feature. This is for advanced users with custom models.
    #[arg(long, value_name = "PATH", conflicts_with = "model")]
    model_path: Option<PathBuf>,

    /// Filter to specific season(s) - can be repeated (RECOMMENDED)
    ///
    /// Using season filtering speeds up matching, reduces token usage,
    /// and improves accuracy by providing more focused context to the AI.
    #[arg(short, long = "season", value_name = "N")]
    seasons: Vec<usize>,

    /// AI backend to use for episode matching
    #[arg(short = 'm', long, value_enum, default_value_t = Matcher::GeminiFlash)]
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
    /// Gemini CLI (requires 'gemini' in PATH)
    Gemini,
    /// Gemini CLI with gemini-2.5-flash model (default, requires 'gemini' in PATH)
    GeminiFlash,
    /// Claude Code CLI (requires 'claude' in PATH)
    Claude,
}

impl From<Matcher> for MatcherType {
    fn from(m: Matcher) -> Self {
        match m {
            Matcher::Gemini => MatcherType::Gemini,
            Matcher::GeminiFlash => MatcherType::GeminiFlash,
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
            println!();
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
    }
}

/// Displays all available Whisper models with download status and exits
fn display_model_list_and_exit() {
    use std::collections::HashMap;

    println!("🔍 Available Whisper Models");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Get cache directory
    let cache_dir = match model_downloader::get_cache_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("❌ Error: Failed to access cache directory: {}", e);
            process::exit(1);
        }
    };

    println!("📁 Cache directory: {}", cache_dir.display());
    println!();

    // Get list of cached models
    let cached_models = match model_downloader::list_cached_models() {
        Ok(models) => models,
        Err(e) => {
            eprintln!("⚠️  Warning: Failed to read cached models: {}", e);
            Vec::new()
        }
    };

    // Create a map for quick lookup
    let cached_map: HashMap<String, &model_downloader::CachedModelInfo> = cached_models
        .iter()
        .map(|m| (m.model_name.clone(), m))
        .collect();

    // Display all models
    let all_models = model_downloader::supported_models();

    println!("Available Models:");
    for model in all_models.iter() {
        if let Some(info) = cached_map.get(*model) {
            println!("  ✓ {:<30} ({})", model, info.size_human_readable());
        } else {
            println!("  ○ {:<30} (not downloaded)", model);
        }
    }

    println!();
    println!("💡 Tips:");
    println!("  - Use --model <NAME> to select a model (e.g., --model tiny)");
    println!("  - Models are downloaded automatically on first use");
    println!("  - Smaller models are faster but less accurate");
    println!("  - Quantized models have -q suffix (smaller size, slightly lower quality)");
    println!();

    if !cached_models.is_empty() {
        let total_size: u64 = cached_models.iter().map(|m| m.size_bytes).sum();
        println!(
            "📊 Total cached: {} models, {} used",
            cached_models.len(),
            humansize::format_size(total_size, humansize::BINARY)
        );
    }

    process::exit(0);
}

/// Presents an interactive series selection prompt using `dialoguer::Select`.
///
/// Builds display labels with year disambiguation: if two candidates share
/// the same name, both get a "(year)" suffix to tell them apart.
fn select_series_interactive(
    candidates: &[SeriesCandidate],
) -> Result<usize, DialogDetectiveError> {
    use std::collections::HashMap;

    // Count how many times each name appears so we know when to show the year
    let mut name_counts: HashMap<&str, usize> = HashMap::new();
    for candidate in candidates {
        *name_counts.entry(&candidate.name).or_default() += 1;
    }

    // Build display labels with year disambiguation
    let display_items: Vec<String> = candidates
        .iter()
        .map(|c| {
            if name_counts.get(c.name.as_str()).copied().unwrap_or(0) > 1 {
                match c.year {
                    Some(year) => format!("{} ({})", c.name, year),
                    None => format!("{} (unknown year)", c.name),
                }
            } else {
                c.name.clone()
            }
        })
        .collect();

    println!();

    let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("🔎 Multiple matches found — select the correct series")
        .items(&display_items)
        .default(0)
        .interact_opt()
        .map_err(|e| DialogDetectiveError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    match selection {
        Some(index) => Ok(index),
        None => Err(DialogDetectiveError::SelectionCancelled),
    }
}

fn main() {
    let cli = Cli::parse();

    // Handle --list-models flag
    if cli.list_models {
        display_model_list_and_exit();
    }

    // Unwrap required arguments (safe because of required_unless_present)
    let video_dir = cli.video_dir.expect("video_dir should be present");
    let show_name = cli.show_name.expect("show_name should be present");

    // Validate arguments
    if !video_dir.exists() {
        eprintln!(
            "❌ Error: Directory does not exist: {}",
            video_dir.display()
        );
        process::exit(1);
    }

    if !video_dir.is_dir() {
        eprintln!("❌ Error: Path is not a directory: {}", video_dir.display());
        process::exit(1);
    }

    // Resolve model path: custom path, selected model, or default 'base'
    let model_path = if let Some(custom_path) = cli.model_path {
        // Custom model path provided - validate it exists
        if !custom_path.exists() {
            eprintln!(
                "❌ Error: Model file does not exist: {}",
                custom_path.display()
            );
            process::exit(1);
        }

        if !custom_path.is_file() {
            eprintln!(
                "❌ Error: Model path is not a file: {}",
                custom_path.display()
            );
            process::exit(1);
        }

        custom_path
    } else {
        // Determine which model to use
        let model_name = cli.model.as_deref().unwrap_or("base");

        // Validate model name against supported list
        let supported = model_downloader::supported_models();
        if !supported.contains(&model_name) {
            eprintln!("❌ Error: Unsupported model '{}'", model_name);
            eprintln!();
            eprintln!("Supported models:");
            for (i, model) in supported.iter().enumerate() {
                eprint!("  {}", model);
                if (i + 1) % 4 == 0 {
                    eprintln!();
                } else {
                    eprint!("  ");
                }
            }
            if supported.len() % 4 != 0 {
                eprintln!();
            }
            eprintln!();
            eprintln!("💡 Tip: Use --list-models to see all available models with details");
            process::exit(1);
        }

        // Download model if needed
        match model_downloader::ensure_model_available(model_name) {
            Ok(path) => path,
            Err(e) => {
                eprintln!(
                    "❌ Error: Failed to download Whisper model '{}': {}",
                    model_name, e
                );
                eprintln!("💡 Tip: You can manually specify a model path with --model-path");
                process::exit(1);
            }
        }
    };

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
        &video_dir,
        &model_path,
        &show_name,
        season_filter,
        cli.matcher.into(),
        handle_progress_event,
        select_series_interactive,
    ) {
        Ok(matches) => {
            if matches.is_empty() {
                println!("❌ Case closed: No matches found");
                return;
            }

            // Plan file operations
            let output_dir = cli.output_dir.as_deref();
            let operations = match plan_operations(&matches, &show_name, &cli.format, output_dir) {
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
