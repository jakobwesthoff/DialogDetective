# DialogDetective

Automatically identify and rename unknown tv series video files by letting AI listen to their dialogue.

<center>
  <a href="https://asciinema.org/a/41c1y7KXjdaZBwPMT09Nkyt7p" target="_blank"><img src="https://asciinema.org/a/41c1y7KXjdaZBwPMT09Nkyt7p.svg" /></a>
</center>

## Why I Built This

I sometimes rip TV series from my Blu-ray/DVD collection to have them available for easier binge watching. Unfortunately, the structure of those disc releases is often completely non-linear - you get files like `TITLE_01.mkv`, `TITLE_03.mkv`, `TITLE_07.mkv` with no clear indication which episode is which.

I didn't want to manually map these weird title IDs to actual season and episode numbers. That would require me to watch a bit of each file and guess based on episode summaries from TV databases. I thought modern LLMs should be able to do this for me. A quick prototype later, it turned out they can.

So I created DialogDetective to do this work automatically. If you have the same problem, this tool might help you too.

## Installation

```bash
cargo install dialog_detective
```

### Pre-built Binaries

Pre-built binaries are available on the [GitHub Releases](https://github.com/jakobwesthoff/DialogDetective/releases) page:

- **macOS** (Apple Silicon & Intel): Built with **Metal** GPU acceleration
- **Linux** (x86_64 & aarch64): Built with **CPU-only** Whisper
- **Windows** (x86_64): Built with **CPU-only** Whisper

### Prerequisites

- **FFmpeg** - Must be installed and available in your PATH for audio extraction
  - macOS: `brew install ffmpeg`
  - Ubuntu/Debian: `apt install ffmpeg`
  - Windows: Download from [ffmpeg.org](https://ffmpeg.org/download.html)
- **AI CLI**: [Gemini CLI](https://github.com/google-gemini/gemini-cli) (default) or [Claude Code](https://claude.ai/code)
  - Must be installed and authenticated before use
- **Rust toolchain** (only if building from source) - install from [rustup.rs](https://rustup.rs)

Whisper models are downloaded automatically on first run.

## Quick Start

```bash
# Dry run - see what would happen (recommended first step)
# It is encouraged to limit processing to specific seasons. See below for more information about this.
dialog_detective ./videos "The Flash" -s 1

# Rename files in place
dialog_detective ./videos "The Flash" --mode rename -s 1

# Copy files to organized directory
dialog_detective ./videos "The Flash" --mode copy -o ./organized -s 1

# Select different Whisper model (default: base)
dialog_detective ./videos "The Flash" --model large-v3-turbo -s 1

# See all available options
dialog_detective --help
```

<!-- docs:start -->
## Documentation

DialogDetective takes the guesswork out of organizing TV series rips. Point it at a directory of video files, tell it the show name, and let AI do the detective work.

The process is simple: search [TVMaze](https://www.tvmaze.com/) for the show, extract audio from each video using [FFmpeg](https://ffmpeg.org/), transcribe the dialogue using [Whisper](https://github.com/ggerganov/whisper.cpp), then use an LLM to match what was said to the correct episode. Finally, rename or copy the files with proper episode information.

If the show name matches multiple series (e.g. "Battlestar Galactica" returns both the 1978 and 2003 versions), you'll get an interactive selection prompt to pick the correct one. When titles are identical, the premiere year is shown to help distinguish them. A unique match is selected automatically.

### CLI Usage

```bash
dialog_detective <VIDEO_DIR> <SHOW_NAME> [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `<VIDEO_DIR>` | Required | Directory to scan for video files |
| `<SHOW_NAME>` | Required | TV series name for metadata lookup |
| `-s, --season <N>` | All | Filter to specific season(s), repeatable |
| `--model <NAME>` | base | Whisper model (tiny/base/small/medium/large) |
| `--model-path <PATH>` | - | Custom Whisper model file path |
| `-m, --matcher <BACKEND>` | gemini | AI backend: gemini or claude |
| `--mode <MODE>` | dry-run | Operation: dry-run, rename, or copy |
| `-o, --output-dir <DIR>` | - | Output directory (required for copy mode) |
| `--format <PATTERN>` | See below | Custom filename template |
| `--list-models` | - | List available Whisper models |

### Operation Modes

DialogDetective supports three operation modes, controlled by the `--mode` option:

| Mode | Description |
|------|-------------|
| `dry-run` | **Default.** Shows what would happen without modifying any files. Always run this first to verify the matches are correct. |
| `rename` | Renames files in place with proper episode information. |
| `copy` | Copies files to a new location (requires `--output-dir`). Original files remain untouched. |

```bash
# Preview changes (always do this first)
dialog_detective ./videos "Breaking Bad" -s 1

# Rename files in place
dialog_detective ./videos "Breaking Bad" -s 1 --mode rename

# Copy to organized directory
dialog_detective ./videos "Breaking Bad" -s 1 --mode copy -o ./organized
```

### Season Filtering

> [!TIP]
> **Highly Recommended**
>
> Always use `--season` when you know which season your files belong to:
> - Dramatically improves matching accuracy
> - Reduces LLM context size (fewer episodes to choose from)
> - Saves API tokens
> - Faster processing

Since you're typically processing a single season at a time when ripping discs, specifying the correct season makes the tool much more effective.

```bash
# Process only season 1 files
dialog_detective ./videos "Breaking Bad" -s 1

# Process multiple seasons
dialog_detective ./videos "Breaking Bad" -s 1 -s 2
```

> [!WARNING]
> The season filter **limits** the matching scope. If you specify `-s 1` and a video file is actually from season 2, it will likely be mismatched to a season 1 episode. Only use season filtering when you know all your video files belong to the specified season(s).

### Filename Templates

Use `--format` to customize output filenames. The default template is:

`{show} - S{season:02}E{episode:02} - {title}.{ext}`

#### Available Variables

| Variable | Description |
|----------|-------------|
| `{show}` | Series name |
| `{season}` / `{season:02}` | Season number (use `:02` for zero-padding, e.g., "01") |
| `{episode}` / `{episode:02}` | Episode number (use `:02` for zero-padding, e.g., "07") |
| `{title}` | Episode title |
| `{ext}` | Original file extension (without dot) |

```bash
# Custom format example
dialog_detective ./videos "The Flash" -s 1 \
  --format "{show} S{season:02}E{episode:02} {title}.{ext}"
```

### Whisper Models

DialogDetective uses [Whisper](https://github.com/ggerganov/whisper.cpp) for speech-to-text transcription. Models are automatically downloaded from [HuggingFace](https://huggingface.co/ggerganov/whisper.cpp) on first use.

#### Model Selection

Choose a model based on your needs:

| Model | Size | Speed | Accuracy | Notes |
|-------|------|-------|----------|-------|
| `tiny` | ~39MB | Fastest | Lower | Good for testing |
| `base` | ~142MB | Fast | Good | **Default.** Best balance of speed and accuracy |
| `small` | ~466MB | Medium | Better | Good for non-English content |
| `medium` | ~1.5GB | Slower | High | Recommended if base struggles |
| `large-v3` | ~2.9GB | Slowest | Highest | Best accuracy, requires more RAM |
| `large-v3-turbo` | ~809MB | Medium | High | Good compromise for large model quality |

English-only variants (`tiny.en`, `base.en`, etc.) are slightly more accurate for English content. Quantized variants (`-q5_0`, `-q5_1`, `-q8_0`) are smaller but slightly less accurate.

```bash
# List all available models and see which are cached
dialog_detective --list-models

# Use a specific model
dialog_detective ./videos "Show" -s 1 --model large-v3-turbo
```

### GPU Acceleration

DialogDetective uses [whisper-rs](https://github.com/tazz4843/whisper-rs) for speech-to-text, which supports various GPU backends for faster transcription.

#### Pre-built Binaries

| Platform | GPU Backend | Notes |
|----------|-------------|-------|
| **macOS** | Metal | Apple GPU acceleration enabled by default |
| **Linux** | CPU-only | Build from source for GPU support |
| **Windows** | CPU-only | Build from source for GPU support |

#### Building with GPU Support

If you have the required GPU frameworks installed on Linux or Windows, you can build with GPU acceleration:

```bash
# NVIDIA CUDA (requires CUDA toolkit)
cargo build --release --features cuda

# Vulkan (requires Vulkan SDK)
cargo build --release --features vulkan

# AMD ROCm/hipBLAS (requires ROCm)
cargo build --release --features hipblas
```

See the [whisper-rs documentation](https://github.com/tazz4843/whisper-rs#features) for detailed requirements for each GPU backend.

### AI Backend

DialogDetective uses external CLI tools for LLM access. You must have one of the following installed and authenticated:

- [Gemini CLI](https://github.com/google-gemini/gemini-cli) (default) - Google's Gemini models
- [Claude Code](https://claude.ai/code) - Anthropic's Claude models

The CLI must be working independently before DialogDetective can use it. Test with `gemini` or `claude` in your terminal.

```bash
# Use Gemini (default)
dialog_detective ./videos "Show" -s 1

# Use Claude
dialog_detective ./videos "Show" -s 1 --matcher claude
```

The interface is abstracted to easily support direct API access in the future. Contributions welcome!

### Cache & Storage

DialogDetective caches various data to avoid redundant processing and speed up repeated runs.

#### Cache Location

All cached data is stored in a platform-specific cache directory:

- **macOS:** `~/Library/Caches/de.westhoffswelt.dialogdetective/`
- **Linux:** `~/.cache/dialogdetective/`
- **Windows:** `%LOCALAPPDATA%\dialogdetective\`

#### What Gets Cached

| Data | Directory | TTL | Why Cached |
|------|-----------|-----|------------|
| **Whisper Models** | `models/` | Permanent | Models are large (39MB - 2.9GB) and don't change. Downloaded once from HuggingFace on first use. |
| **Search Results** | `search/` | 24 hours | TVMaze search results for show name queries. Avoids re-hitting the search API on repeated runs. |
| **Series Metadata** | `metadata/` | 24 hours | Episode lists from TVMaze rarely change. Cached per show ID and season filter. |
| **Transcripts** | `transcripts/` | 24 hours | Whisper transcription is CPU/GPU intensive. Caching by video file hash means re-running on the same files skips transcription entirely. |
| **Match Results** | `matching/` | 24 hours | LLM matching costs tokens and time. Results are cached by a composite key (video hash + show + seasons + matcher), so identical queries return instantly. |

The 24-hour TTL balances freshness with efficiency. If you need to force a refresh (e.g., after TVMaze updates episode data), simply delete the relevant cache subdirectory.

#### Temporary Files

During processing, DialogDetective extracts audio to temporary WAV files in your system's temp directory (`/tmp`, `/var/folders/...`, or `%TEMP%`). These files are automatically cleaned up when processing completes or if the program is interrupted.

#### Managing Cache

To clear all cached data:
```bash
# macOS
rm -rf ~/Library/Caches/de.westhoffswelt.dialogdetective/

# Linux
rm -rf ~/.cache/dialogdetective/
```

To clear only models (to free disk space):
```bash
# macOS
rm -rf ~/Library/Caches/de.westhoffswelt.dialogdetective/models/

# Linux
rm -rf ~/.cache/dialogdetective/models/
```

Use `dialog_detective --list-models` to see which models are currently cached and their sizes.

<!-- docs:end -->

## Contributing

Contributions are welcome! Feel free to submit issues and pull requests.

## License

MIT License - see [LICENSE](LICENSE) file for details.
