# DialogDetective

Automatically identify and rename unknown tv series video files by letting AI listen to their dialogue.

<center>
  <a href="https://asciinema.org/a/41c1y7KXjdaZBwPMT09Nkyt7p" target="_blank"><img src="https://asciinema.org/a/41c1y7KXjdaZBwPMT09Nkyt7p.svg" /></a>
</center>

## Why I Built This

I sometimes rip TV series from my Blu-ray/DVD collection to have them available for easier binge watching. Unfortunately, the structure of those disc releases is often completely non-linear - you get files like `TITLE_01.mkv`, `TITLE_03.mkv`, `TITLE_07.mkv` with no clear indication which episode is which.

I didn't want to manually map these weird title IDs to actual season and episode numbers. That would require me to watch a bit of each file and guess based on episode summaries from TV databases. I thought modern LLMs should be able to do this for me. A quick prototype later, it turned out they can.

So I created DialogDetective to do this work automatically. If you have the same problem, this tool might help you too.

## How It Works

DialogDetective extracts audio from your video files, transcribes the dialogue using Whisper (with GPU acceleration), fetches episode metadata from TVMaze, and uses an LLM (Gemini or Claude) to match the transcript to the correct episode. Then it renames or copies the files with proper episode information.

## Installation

```bash
cargo install --path .
```

### Prerequisites

- **Rust toolchain** (install from [rustup.rs](https://rustup.rs))
- **FFmpeg** - Must be installed and available in your PATH for audio extraction
  - macOS: `brew install ffmpeg`
  - Ubuntu/Debian: `apt install ffmpeg`
  - Windows: Download from [ffmpeg.org](https://ffmpeg.org/download.html)
- **AI CLI**: [Gemini CLI](https://ai.google.dev/) (default) or [Claude Code](https://claude.com/code)
  - Must be installed and authenticated before use

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

### Season Filtering (Highly Encouraged!)

DialogDetective can work on full series without season filtering, but **using `-s` or `--season` is highly encouraged** for several important reasons:

- **Reduces LLM context size** - Only sends relevant episodes to the AI instead of the entire series
- **Improves matching accuracy** - Fewer episodes means less confusion and better identification
- **Saves tokens** - Significantly reduces API costs, especially for long-running series
- **Faster processing** - Less data to send and analyze

**Important:** The season filter **limits** the matching scope. If you specify `-s 1` and a video file is actually from season 2, it will likely be mismatched to a season 1 episode. Only use season filtering when you know all your video files belong to the specified season(s).

Since you're typically processing a single season at a time when ripping discs, specifying the correct season makes the tool much more effective: `-s 1` or `--season 2`

## Usage

Run `dialog_detective --help` for complete usage information.

Important options:
- `-s` / `--season` - Filter to specific season (highly encouraged, can be repeated)
- `--model` - Select Whisper model
- `--matcher` - AI backend: gemini (default) or claude
- `--mode` - Operation: dry-run (default), rename, or copy
- `--list-models` - Show all available Whisper models

## Filename Templates

Use `--format` to customize output filenames.

**Default:** `{show} - S{season:02}E{episode:02} - {title}.{ext}`

**Available variables:**
- `{show}` - Series name
- `{season}` - Season number (use `{season:02}` for zero-padding)
- `{episode}` - Episode number (use `{episode:02}` for zero-padding)
- `{title}` - Episode title
- `{ext}` - Original file extension

**Example:**
```bash
dialog_detective ./videos "The Flash" -s 1 \
  --format "{show} S{season:02}E{episode:02} {title}.{ext}"
```

## AI Backend Integration

DialogDetective currently uses CLI tools for LLM access ([Gemini CLI](https://ai.google.dev/) and [Claude Code](https://claude.com/code)). This was the easiest way for me to quickly support LLMs, as I already had both tools installed and authenticated on my system.

The interface is abstracted enough to easily add direct API access via API keys (OpenAI, Anthropic, etc.) if there's demand for it. If you need direct API support, feel free to reach out or submit a PR - contributions are highly welcome!

## License

MIT License - see [LICENSE](LICENSE) file for details.
