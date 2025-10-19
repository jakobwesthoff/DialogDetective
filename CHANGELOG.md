# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 1.1.0 - 2025-10-19

### Added
- New `gemini-flash` matcher option for using Gemini CLI with the `gemini-2.5-flash` model
- Configurable model parameter support for Gemini CLI matcher

### Changed
- Default matcher changed from `gemini` to `gemini-flash` for faster, more cost-effective episode matching
- `GeminiCliMatcher` now accepts an optional model parameter
- Cache keys now differentiate between different Gemini models

## 1.0.0 - 2025-10-13

### Added
- Initial release with video file identification through audio analysis
- Speech-to-text transcription using Whisper (29 models, GPU support)
- AI-based episode matching via Gemini CLI and Claude Code CLI
- TVMaze metadata integration with caching
- File operations: dry-run, rename, and copy modes
- Season filtering and customizable filename templates
