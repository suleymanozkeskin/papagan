# Changelog

All notable changes to papagan are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

_(nothing yet)_

## [0.1.7] — 2026-04-22

### Added

- **Batch detection API** across Rust, Python, and Node:
  `detect_batch(&[S])` and `detect_detailed_batch(&[S])`. Fans out across
  cores via rayon for multi-document workloads. Measured **~3.5× speedup**
  on 1000 paragraphs and **~4.8×** on 1870 short titles (8-core M-series,
  vs a serial `for detect()` loop).
- **Async batch for Node**: `detectBatchAsync` / `detectDetailedBatchAsync`
  return `Promise<Output[]>` / `Promise<Detailed[]>`. Work runs on libuv's
  thread pool so the V8 event loop stays responsive. Measured max
  event-loop stall on a 1000-paragraph batch: **35 ms → 11 ms** vs the sync
  variant.
- **Python GIL release** around batch calls via `py.detach()` so concurrent
  Python threads (ThreadPoolExecutor) can progress during the Rust fan-out.
- **FLORES-200 accuracy fixture** (`cargo xtask fetch-flores`, committed at
  `papagan/tests/fixtures/accuracy_flores.tsv` — 10,120 sentences,
  1,012/lang, CC-BY-SA 4.0). Complements the Tatoeba fixture with
  long-form, Wikipedia/news-style prose. Result at default dict: **99.9 %**.
- **Leipzig long-form speed-bench fixture** (`cargo xtask fetch-leipzig`,
  writes `bench/paragraphs.json` — 1,000 paragraphs, avg 87 words, range
  31–166). Exercises the aggregation and parallel code paths. Gitignored;
  regenerate locally.
- **Criterion paragraph-bench groups**: `detect_paragraph_single` and
  `detect_paragraph_sweep`, added to `papagan/benches/detect.rs`. Skip
  cleanly if `bench/paragraphs.json` is missing.
- `DetectorBuilder::parallel_threshold(usize::MAX)` is the documented
  single opt-out for all rayon use — including batch-level parallelism,
  not just intra-document.

### Changed

- **`DEFAULT_PARALLEL_THRESHOLD` lowered from 64 to 32** (intra-document
  per-word parallelism). Tuned via sweep over `{∞, 1024, 256, 128, 64, 32,
  16, 0}` on paragraphs (median 84 words) and titles (median 8 words, p95
  = 13). At 32, titles stay fully serial; paragraphs win ~7–9% vs 64. See
  `examples/parallel_sweep.rs`.
- **Batch parallelism uses dynamic routing**, not a fixed batch-size
  threshold. Parallel iff `batch_size >= 2 AND approx_tokens >= 50` (where
  approx_tokens is a `split_whitespace().count()` sum across inputs, ~500
  ns per paragraph — negligible overhead). This avoids the
  titles-at-N=4 regression that a fixed `BATCH_PARALLEL_THRESHOLD = 4`
  caused, while capturing the N=4-paragraphs win that `=8` would miss.
  See `examples/batch_routing_sweep.rs` and `batch_strategy_bench.rs`.
- **Tokenizer drops numeric-only tokens** (e.g. `802`, `2024`, `1920`).
  Matches the existing xtask training-pipeline filter, so runtime now
  aligns with the distribution the ngram model was trained on. Measured
  ~12 % faster on paragraph content containing numerics; titles unaffected.
- Per-call `detect()` and `detect_detailed()` — no API change, only the
  above threshold and tokenizer tweaks affect behavior.

### Performance

End-to-end, measured vs v0.1.6 installed from PyPI/npm on 8-core M-series:

| Path | Fixture | v0.1.6 | master | Δ |
|---|---|---:|---:|---:|
| Python `detect()` loop | 1000 paragraphs | 92.5 ms | 78.5 ms | **−15 %** |
| Python `detect()` loop | 1870 titles | 16.3 ms | 16.4 ms | +1 %  |
| Python `detect_batch()` | 1000 paragraphs | — (new) | **22.7 ms** | 3.5× vs loop |
| Node `detect()` loop | 1000 paragraphs | 102 ms | 92 ms | **−10 %** |
| Node `detect()` loop | 1870 titles | 23.1 ms | 22.8 ms | −1 %  |
| Node `detectBatchAsync` | 1000 paragraphs | — (new) | **31 ms** | non-blocking |
| Rust `detect_paragraph_sweep` | 1000 paragraphs | 94.0 ms | 83.1 ms | **−12 %** |
| Rust `detect_full_sweep` | 1870 titles | 15.2 ms | 16.7 ms | +5 %  |

Title-path micro-benchmarks show a small criterion-reported regression at
20 samples that washes out under 10-run averaging. No user-visible titles
regression through the FFI layer.

### Accuracy

No change to the detection algorithm; numbers consistent with v0.1.6:

| Fixture | Dict tier | Accuracy |
|---|---|---:|
| Tatoeba (5,000 sentences) | default 3k | 99.4 % |
| **FLORES-200 devtest (10,120)** | default 3k | **99.9 %** (new eval) |

### Internal / Tooling

- Two-axis batch-routing sweep methodology (`batch_routing_sweep.rs`,
  `batch_strategy_bench.rs`) for future parallelism tuning.
- Node build workflow: separated auto-generated native types from the
  hand-crafted `index.d.ts` via `napi build --dts native.generated.d.ts
  --no-js`. The hand-crafted wrappers (Output/Detailed/Detector JS classes,
  camelCase+snake_case aliases) are no longer clobbered by `npm run build`.
- `find_misclassified` example — diagnostic tool for inspecting per-miss
  language distributions against any TSV accuracy fixture.

### Dependencies

- xtask only: `flate2 = "1"`, `tar = "0.4"` (for the FLORES tarball reader).
  No runtime dependency changes.

### Compatibility

- API-additive only. No breaking changes to the Rust, Python, or Node
  public surface. Existing `detect()` / `detect_detailed()` calls behave
  identically modulo the ~12 % paragraph speedup from the numeric-token
  filter.

---

## [0.1.6] — 2026-04

Initial tracked release baseline. See git history for full commit log.
