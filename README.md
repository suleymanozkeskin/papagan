# papagan

Fast, small-binary language detection for **Rust**, **Python**, and **Node.js**.

Same core detector everywhere — a Rust crate with thin native bindings for Python (via PyO3) and Node.js (via napi-rs). Opt-in languages at compile time, weighted per-word output, zero-dependency once built.

```
The quick brown fox jumps over the lazy dog.         → en  0.998
Die Katze sitzt auf der warmen Matte.                → de  0.996
El gato está sentado tranquilamente en la alfombra.  → es  0.991
```

## Features

- **10 supported languages** — English, German, Turkish, Russian, French, Spanish, Italian, Dutch, Portuguese, Polish (ISO 639-1 codes: `en`, `de`, `tr`, `ru`, `fr`, `es`, `it`, `nl`, `pt`, `pl`).
- **Two-stage detection** — fast dictionary lookup (top-frequency words, binary search) then character-trigram fallback for unknown words. Script detection is free — Cyrillic, Latin, etc. fall out of the trigram model.
- **Weighted per-word output** — see exactly which languages contributed to the document-level classification, with probabilities per token. Great for mixed-language text.
- **Opt-in language packs** — include only the languages you need. Binary size scales linearly with enabled languages; the default is English-only.
- **Opt-in dictionary size tiers** — default 3k words/language (fast, small), opt into 5k or 10k for higher recall on rare words.
- **Batch detection** — `detect_batch([...])` fans out across cores via rayon for multi-document workloads (3–5× on an 8-core M-series for 1000-paragraph batches). Python callers get the GIL released around the call.
- **Automatic intra-document parallelism** for long inputs above a configurable word threshold (default 32; below that, serial).

## Install

<table>
<tr><td><b>Rust</b></td><td><code>cargo add papagan --features all-langs</code></td></tr>
<tr><td><b>Python</b></td><td><code>uv add papagan</code> or <code>pip install papagan</code></td></tr>
<tr><td><b>Node.js</b></td><td><code>bun add papagan</code> or <code>npm install papagan</code></td></tr>
</table>

## Quick start

### Rust

```rust
use papagan::Detector;

let detector = Detector::new();
let output = detector.detect("Die Katze sitzt auf der Matte");
let (lang, confidence) = output.top();
println!("{}: {:.3}", lang.iso_639_1(), confidence);
// de: 0.996
```

### Python

```python
from papagan import Detector

detector = Detector()
output = detector.detect("Die Katze sitzt auf der Matte")
lang, confidence = output.top()
print(f"{lang}: {confidence:.3f}")
# de: 0.996
```

### Node.js

```js
const { Detector } = require('papagan')

const detector = new Detector()
const output = detector.detect('Die Katze sitzt auf der Matte')
const [lang, confidence] = output.top()
console.log(`${lang}: ${confidence.toFixed(3)}`)
// de: 0.996
```

### Per-word breakdown

All three bindings support `detect_detailed(text)` which returns the language distribution **per word** alongside the document aggregate. Useful for mixed-language input, debugging, or showing confidence.

```python
d = detector.detect_detailed("The cat is black. Die Katze ist schwarz.")
for word in d.words:
    top_lang, top_score = max(word.scores, key=lambda x: x[1])
    print(f"  {word.token:<10} [{word.source}]  {top_lang} ({top_score:.2f})")
# the        [dict]   en (0.85)
# cat        [ngram]  en (0.99)
# is         [dict]   en (0.87)
# black      [ngram]  en (0.83)
# die        [dict]   de (0.65)
# katze      [ngram]  de (1.00)
# ist        [dict]   de (1.00)
# schwarz    [ngram]  de (1.00)
```

### Batch detection

When you have many documents to classify at once, `detect_batch` fans out across cores — near-linear speedup on M-series laptops with 4–8 cores:

```rust
let results: Vec<Output> = detector.detect_batch(&docs);  // Vec<String> / &[&str] / &[String]
let detailed: Vec<Detailed> = detector.detect_detailed_batch(&docs);
```

```python
results = detector.detect_batch(docs)                 # releases the GIL while running
detailed = detector.detect_detailed_batch(docs)
```

```js
const results = detector.detectBatch(docs)            // blocks the V8 thread; offload to a Worker for large batches
const detailed = detector.detectDetailedBatch(docs)
```

The batch path activates at ≥ 4 inputs. Below that, `detect_batch` falls back through the normal per-call path so it never regresses small-batch latency. Setting `parallel_threshold(usize::MAX)` opts out of rayon entirely (including batch-level parallelism).

## Configuration

### Restrict to a subset of languages (runtime)

If your input only contains a few languages, restrict the detector to those — faster and more confident.

```rust
use papagan::{Detector, Lang};
let d = Detector::builder().only([Lang::En, Lang::De]).build();
```

```python
d = Detector(only=["en", "de"])
```

```js
const d = Detector.builder().only(['en', 'de']).build()
```

### Dictionary tiers (compile time)

Controls how many top-frequency words get baked into the binary. The raw Rust crate defaults to 3k, and the Python / Node wrappers in this repo currently build against that same default unless packaging overrides `PAPAGAN_DICT_SIZE`.

| Tier | Flag | Binary (all-langs) | Accuracy (Tatoeba, 5k sentences) | Accuracy (FLORES-200 devtest, 10k sentences) |
|---|---|---|---|---|
| 1k | `PAPAGAN_DICT_SIZE=1000` | 3.3 MB | 99.1% | — |
| **3k** (default) | none | **4.9 MB** | **99.4%** | **99.9%** |
| 5k | `--features dict-5k` | 6.5 MB | 99.4% | — |
| 10k | `--features dict-10k` | 10.5 MB | 99.7% | — |

Tatoeba captures the short, subtitle-shaped regime (20–200 chars, informal); FLORES-200 devtest captures long-form, Wikipedia/news prose. Accuracy is higher on FLORES because longer inputs give the aggregation more evidence per document. Both fixtures are regenerated via `cargo xtask fetch-eval` and `cargo xtask fetch-flores`.

### Parallelism

Two levels of rayon parallelism, independently controllable:

- **Intra-document** — per-word scoring parallelizes automatically above 32 words (default). Tuned empirically on Leipzig paragraphs (median 84 words) and short titles (median 8 words); see `examples/parallel_sweep.rs` for the sweep that set this.
- **Batch-level** — `detect_batch([...])` fans out across documents when the batch is ≥ 4. Intra-document parallelism is forced serial inside the batch loop to avoid nesting rayon work.

```rust
let d = Detector::builder()
    .parallel_threshold(128)        // only parallelize per-word work at 128+ tokens
    .build();

let d = Detector::builder()
    .parallel_threshold(usize::MAX) // disable rayon entirely (both levels)
    .build();
```

Disable rayon at compile time by building without the `parallel` feature for embedded/wasm/minimal binaries.

## Supported languages

| Code | Language | Code | Language |
|---|---|---|---|
| `de` | German | `it` | Italian |
| `en` | English | `nl` | Dutch |
| `es` | Spanish | `pl` | Polish |
| `fr` | French | `pt` | Portuguese |
| `ru` | Russian | `tr` | Turkish |

Training corpus: [hermitdave/FrequencyWords](https://github.com/hermitdave/FrequencyWords) (OpenSubtitles 2018, MIT licensed).

## Benchmarks

Two regimes matter for language detection — short, query-shaped inputs (search boxes, titles, social) and long-form documents (news articles, Wikipedia paragraphs). papagan is benched against both.

### Short inputs — cross-library comparison

Python wrapper, 1,870 real short titles (239 unique, mostly English with de/fr/es mixed in), M-series Mac.

| Library | Install size | Median | Mean | P95 | Full sweep (1,870) |
|---|---:|---:|---:|---:|---:|
| **papagan** | **3.3 MiB** | **9.3 µs** | **11.3 µs** | **28 µs** | **16.7 ms** |
| py3langid | 740 KiB | 48 µs | 50 µs | 67 µs | 84 ms |
| langdetect | 2.3 MiB | 975 µs | 1,243 µs | 3,292 µs | 2,186 ms |
| lingua (all langs) | 294 MiB | 1,349 µs | 1,816 µs | 5,251 µs | 1,768 ms |

Top-language agreement is essentially identical across all four — papagan matches the feature-rich options on output while being ~5× faster than the next fastest and ~100× faster than lingua/langdetect at a fraction of the disk footprint.

### Long-form and batch — internal regimes

1000 Leipzig news paragraphs (median 84 words, range 31–166) and the 1,870 titles above. M-series, 8 cores, all-langs release build.

| Regime | Serial `for detect()` | `detect_batch([...])` | Speedup |
|---|---:|---:|---:|
| Paragraphs (1000 × ~84 words) | 84 ms / 84 µs per call | 26 ms / 26 µs per call | **3.2×** |
| Titles (1870 × ~8 words) | 16 ms / 8.4 µs per call | 3.3 ms / 1.8 µs per call | **4.8×** |

Titles win bigger because they're dict-hit heavy (near-zero per-call work, so rayon setup amortizes well); paragraphs are ngram-heavy and hit memory-bandwidth limits on the shared PHF trigram table when all cores run concurrently. Scales top out around 3.9× on 8 cores.

Reproduce with `cargo run --release --example batch_sweep --features all-langs` (requires `bench/paragraphs.json` — see `cargo xtask fetch-leipzig`).

## How it works

1. **Tokenize** the input with Unicode-aware segmentation + NFKC normalization + default lowercase (preserves Turkish `İ`/`ı`/`I`/`i` as distinct).
2. **Stage 1** — for each token, binary-search a sorted table of `(word, lang, rank)` triples pulled from the top-N frequency list of each enabled language. Hits yield rank-weighted priors: `P(lang|word) ∝ 1/(rank + k)`.
3. **Stage 2** (fallback for tokens that missed) — score against per-language character-trigram models (PHF perfect-hash at compile time, smoothed log-probabilities), softmax across enabled languages.
4. **Aggregate** — confidence-weighted combination of per-word distributions into a document-level distribution. High-confidence tokens (peaky distributions) dominate; uniform distributions contribute little.

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.
