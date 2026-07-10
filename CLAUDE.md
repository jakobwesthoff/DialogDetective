# CLAUDE.md

Non-obvious facts only. Module layout, types, public API and CLI flags are
discoverable from `src/` and `--help`; do not restate them here.

DialogDetective identifies unknown TV episode video files by their dialogue:
audio extraction → speech-to-text → LLM matching against episode summaries →
rename or copy.

## Traps

- **`transcript_cache` is keyed on the video's blake3 hash alone.** A different
  `--model` reuses the transcript from whichever model ran first. Anything
  affecting transcription output must enter that key.
- **`compute_matching_cache_key` folds in the `MatcherType` variant, not the
  pinned model string.** Two variants resolving to the same CLI must not share a key.
- **`whisper-rs` is declared in two target sections** (`cfg(target_os = "macos")`
  with `metal`, and `cfg(not(...))`). `whisper-rs-sys` sets `links = "whisper"`, so
  versions cannot coexist. Bumping one entry yields an unresolvable graph, not a
  clear error.
- **`sanitize_filename` trims leading/trailing dots.** `"Cat's in the Bag..."` →
  `"Cat's in the Bag"`.
- **Videos are detected by content, not extension**: `file_resolver` sniffs 8 KB
  with `infer`.
- **`compute_video_hash` hashes the entire file** (`update_mmap_rayon`), once per
  video, before any cache lookup.
- **`ProgressEvent` indices are 0-based**; `main.rs` converts to 1-based for display.

## Contracts

**LLM response.** Matchers pipe the prompt to a CLI over stdin and require a
` ```json ` fenced `{"season": X, "episode": Y}`. Prose answers or bare ` ``` `
fences fail to parse. Built by `NaivePromptGenerator`.

**Matcher backends.** Each `MatcherType` selects a CLI and a model via that CLI's
`--model` flag. `Gemini` leaves it unset, inheriting the CLI default, so its
results are not reproducible across machines.

| `MatcherType` | CLI | `--model` |
|---|---|---|
| `Gemini` | `gemini` | *(unset)* |
| `GeminiFlash` | `gemini` | `gemini-2.5-flash` |
| `Claude` | `claude` | `sonnet` |
| `ClaudeHaiku` | `claude` | `haiku` |

**Caches.** Four `CacheStorage` instances under
`de.westhoffswelt.dialogdetective`, each with a 24h TTL: `search`, `metadata`,
`transcripts`, `matching`. Only `transcripts` and `matching` get `clean()` at
startup; the other two evict expired entries lazily in `load()`.

**GPU is compile-time.** `metal` comes from the macOS target section; `cuda`,
`vulkan`, `hipblas` are opt-in features. No runtime selection: `use_gpu(true)`
only enables whatever was compiled in.

**Memory.** Whisper holds the whole decoded track: 16 kHz mono is ~32 KB/s as
`i16` (~115 MB/hour), transiently doubled during `f32` conversion. No streaming.

## Conventions

- Add dependencies with `cargo add`, never by hand-editing `Cargo.toml`.
- `thiserror`, one error type per module, funnelled into `DialogDetectiveError`.
  Add variants only when a call site needs one.
- Modules are `pub(crate)`; `model_downloader` is deliberately public. Export from
  `lib.rs` only what the CLI or a consumer needs.
- The library never prints. It reports via an `FnMut(ProgressEvent)` callback; all
  formatting lives in `main.rs`.
- Detective terminology throughout ("investigating", "cases", "clues", "solving").
  Maintain it when adding features.

## Design decisions

- **Callback over returned progress stream:** UI-agnostic, zero overhead when
  events are ignored.
- **Sequential per-video processing:** bounded memory and immediate feedback,
  rather than batching all transcripts before matching.
- **CLI-backed matchers rather than HTTP APIs:** no key management; the same
  prompt/parse path extends to direct API matchers later.
- **Season filtering:** shrinks the LLM candidate list; folded into the metadata
  and matching cache keys.
- **Duplicate suffixes** (`name (2).ext`): predictable, needs no configuration.
