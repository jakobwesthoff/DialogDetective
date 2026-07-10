# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

It deliberately records only what the code does not make obvious. Module layout,
type definitions, the public API surface and the CLI flags are all discoverable by
reading `src/` or running `--help`, and are therefore not duplicated here.

## Overview

DialogDetective identifies unknown TV episode video files by their dialogue:
audio extraction → speech-to-text → LLM matching against episode summaries →
rename or copy. It is a single crate exposing both a library (`src/lib.rs`) and a
CLI (`src/main.rs`).

## Traps

Things that have bitten before, or that a reader will get wrong by assuming the
obvious.

### The transcript cache is not keyed on the Whisper model

`transcript_cache` is keyed on the blake3 hash of the video file and nothing
else. Passing a different `--model` therefore reuses the transcript produced by
whichever model ran first. Anything that changes transcription output must become
part of that key, or the cache must be cleared by hand.

By contrast `compute_matching_cache_key` folds in the show name, season filter and
`MatcherType`. Note it folds in the *variant*, not the pinned model string, so two
variants that resolve to the same CLI must not share a key.

### `whisper-rs` is declared twice and must move as a unit

It appears under both `[target.'cfg(target_os = "macos")'.dependencies]` (with
`metal`) and `[target.'cfg(not(target_os = "macos"))'.dependencies]`.
`whisper-rs-sys` declares `links = "whisper"`, so two versions cannot coexist in
one dependency graph. Bumping only one entry produces an unresolvable graph rather
than a useful error.

### `sanitize_filename` trims leading and trailing dots

`"Cat's in the Bag..."` becomes `"Cat's in the Bag"`, so the formatted name is
`... - Cat's in the Bag.mp4` and not `... - Cat's in the Bag....mp4`. Episode
titles ending in ellipses are common.

### Video files are detected by content, not extension

`file_resolver` sniffs the first 8 KB with `infer`. A file named `.mp4` that is not
video will be skipped, and an extensionless video will be picked up.

### `compute_video_hash` reads the whole file

It is a blake3 hash over the entire video via `update_mmap_rayon`, not a sample of
it. Cost scales with file size, and it runs once per video before any cache lookup.

### Progress indices are 0-based

`ProgressEvent` carries 0-based indices to match `enumerate()`. `main.rs` converts
to 1-based purely for display.

## Contracts

### LLM matcher response

Matchers pipe a prompt to a CLI over stdin and require a ` ```json ` fenced block
containing `{"season": X, "episode": Y}`, which is then looked up in the fetched
`TVSeries`. A model that answers in prose, or fences with plain ` ``` `, fails to
parse. `NaivePromptGenerator` builds this instruction.

### Matcher backends and their models

Each `MatcherType` selects a CLI *and* a model, passed through that CLI's `--model`
flag. `Gemini` leaves the flag unset and inherits the CLI's own default; the other
three pin explicitly. Leaving a backend unpinned means results are not reproducible
across machines.

| `MatcherType` | CLI | `--model` |
|---|---|---|
| `Gemini` | `gemini` | *(unset)* |
| `GeminiFlash` | `gemini` | `gemini-2.5-flash` |
| `Claude` | `claude` | `sonnet` |
| `ClaudeHaiku` | `claude` | `haiku` |

### Caches

Four `CacheStorage` instances under the platform cache dir for
`de.westhoffswelt.dialogdetective`, each opened with a 24h TTL: `search`,
`metadata`, `transcripts`, `matching`. Only `transcripts` and `matching` have
`clean()` called on them at the start of an investigation; expired `search` and
`metadata` entries are evicted lazily on load.

### GPU acceleration is compile-time

`metal` is enabled by the macOS target section. `cuda`, `vulkan` and `hipblas` are
opt-in crate features. There is no runtime backend selection; `use_gpu(true)` only
toggles use of whatever was compiled in.

## Conventions

- **Dependencies:** always `cargo add <crate>`, never hand-edit `Cargo.toml`.
- **Errors:** `thiserror`, one error type per module, all funnelled into
  `DialogDetectiveError`. Add variants incrementally, only when a call site needs one.
- **Visibility:** modules are `pub(crate)` by default. `model_downloader` is
  deliberately public. Export from `lib.rs` only what the CLI or a library consumer
  genuinely needs.
- **Progress:** the library reports through an `FnMut(ProgressEvent)` callback and
  never prints. Keep it UI-agnostic; all formatting belongs in `main.rs`.
- **Theme:** detective terminology throughout ("investigating", "cases", "clues",
  "solving"). Maintain the metaphor when adding features.

## Design decisions

- **Callback over returned progress stream:** UI-agnostic, zero overhead when the
  caller ignores events.
- **Sequential per-video processing:** bounded memory and immediate feedback,
  rather than batching all transcripts before matching.
- **CLI-backed matchers rather than HTTP APIs:** no key management, and the same
  prompt/parse path extends to direct API matchers later.
- **Season filtering:** shrinks the candidate list sent to the LLM, and is folded
  into the metadata and matching cache keys.
- **Duplicate suffixes** (`name (2).ext`): predictable, needs no configuration.

## Known limitations

- Whisper holds the entire decoded track in memory. 16 kHz mono is ~32 KB/s as
  `i16` (~115 MB/hour), transiently doubled while converting to `f32`. Long videos
  need proportional RAM; there is no streaming.
- GPU acceleration requires both compatible hardware and the matching compile-time
  feature.
- The chosen LLM CLI must be installed and authenticated.
- Processing is sequential; no parallelism across videos.
