# VHS Demo Recording

This directory contains files for recording the DialogDetective demo video using [VHS](https://github.com/charmbracelet/vhs).

## Files

- `demo.tape` - VHS tape file defining recording settings, theme, and commands
- `mock_dialog_detective.sh` - Mock script simulating DialogDetective output with realistic timing
- `record.sh` - Helper script to set up the environment and run the recording

## Prerequisites

- [VHS](https://github.com/charmbracelet/vhs) installed (`brew install vhs` on macOS)
- FFmpeg (installed automatically with VHS)

## Recording

Run from this directory:

```bash
./record.sh
```

This will:
1. Create a temp environment at `/tmp/dialogdetective-demo`
2. Copy the mock script
3. Run VHS to record the demo
4. Move outputs (`demo.webm`, `demo.mp4`) to `../pages/assets/`
5. Clean up temp files

## Output

The recorded videos are saved to `docs/pages/assets/` for use on the project website.

## Customization

- **Timing**: Adjust `sleep` durations in `mock_dialog_detective.sh`
- **Theme**: Modify the color scheme in `demo.tape`
- **Content**: Edit the mock script to show different files or output
