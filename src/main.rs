use dialog_detective::investigate_case;
use std::env;
use std::path::Path;
use std::process;

fn main() {
    // Get arguments from command line
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <directory> <model_path>", args[0]);
        eprintln!("\nExample: {} /path/to/videos models/ggml-base.bin", args[0]);
        process::exit(1);
    }

    let directory = Path::new(&args[1]);
    let model_path = Path::new(&args[2]);

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
    if let Err(e) = investigate_case(directory, model_path) {
        eprintln!("\nError during investigation: {}", e);
        process::exit(1);
    }
}
