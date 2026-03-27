# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 2.0.0 - 2026-03-27

### Added
- Interactive series selection when a show name matches multiple results on TVMaze
- Year disambiguation in selection prompt when multiple candidates share the same title
- Search results cache (`search/` directory, 24h TTL) to avoid redundant TVMaze API calls
- `SeriesCandidate` public type for representing search results
- `SelectionCancelled` error variant for when the user aborts selection
- `dialoguer` dependency for interactive terminal selection UI

### Changed
- **Breaking:** `investigate_case` now requires an additional `select_series` callback parameter
- **Breaking:** `MetadataProvider` trait split into `search_series` and `fetch_series` methods
- TVMaze provider now uses `/search/shows` endpoint (returns multiple candidates) instead of `/singlesearch/shows` (single result)
- Episode metadata cache keys now use TVMaze show ID instead of show name
- `CachedMetadataProvider::new` now takes separate search and metadata cache instances

## 1.1.1 - 2025-02-03

### Changed

- Use native rustls instead of openssl TLS layer to allow for static linking with musl libc

### Fixed

- Cross-compilation for Apple Silicon (aarch64-apple-darwin) on GitHub Actions by targeting ARMv8.5-a to avoid i8mm intrinsics unsupported on M1
- Build for ARM64 Linux (aarch64-unknown-linux-musl) by using cargo-zigbuild instead of cross-rs with outdated GCC

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
