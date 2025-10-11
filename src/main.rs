use dialog_detective::investigate_case;
use std::env;
use std::path::Path;
use std::process;

fn main() {
    // Get directory path from command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <directory>", args[0]);
        eprintln!("\nExample: {} /path/to/videos", args[0]);
        process::exit(1);
    }

    let directory = Path::new(&args[1]);

    // Check if directory exists
    if !directory.exists() {
        eprintln!("Error: Directory does not exist: {}", directory.display());
        process::exit(1);
    }

    if !directory.is_dir() {
        eprintln!("Error: Path is not a directory: {}", directory.display());
        process::exit(1);
    }

    // Run the investigation
    if let Err(e) = investigate_case(directory) {
        eprintln!("\nError during investigation: {}", e);
        process::exit(1);
    }
}
