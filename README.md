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

Two axes matter for a language detector: how well it classifies each supported language (**accuracy**) and how fast it runs under each binding + API combination (**performance**). Both are benched on the same machine in one session and fully reproducible via the commands at the bottom of this section.


### Performance matrix — binding × library × workload

Measured on Darwin arm64, 2026-04-22, all-langs release build, 8 cores. Median of 7 full-sweep iterations per cell.

Both workloads come from open corpora: Tatoeba sentences (CC-BY 2.0 FR; `cargo xtask fetch-eval`) and Leipzig news paragraphs (CC-BY 4.0; `cargo xtask fetch-leipzig`). `Tokens` / `Bytes` anchor the absolute amount of work per row; `ns/tok` is the derived per-token rate — the cleanest cross-library comparison since it normalizes out workload size.

| Binding | Library | Tokens | Bytes | Loop (ms) | Loop (ns/tok) | Batch (ms) | Batch (ns/tok) | Async (ms) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| Rust | papagan | 35k | 222 KB | **31.25** | **900** | **7.98** | **230** | — |
| Rust | papagan | 87k | 620 KB | **77.82** | **898** | **23.04** | **266** | — |
| Python | papagan | 35k | 222 KB | **32.94** | **949** | **9.67** | **279** | — |
| Python | papagan | 87k | 620 KB | **79.94** | **923** | **23.74** | **274** | — |
| Python | py3langid | 35k | 222 KB | 223.59 | 6 442 | — | — | — |
| Python | py3langid | 87k | 620 KB | 120.81 | 1 395 | — | — | — |
| Python | langdetect | 35k | 222 KB | 6 959.25 | 200 526 | — | — | — |
| Python | langdetect | 87k | 620 KB | 1 348.72 | 15 570 | — | — | — |
| Python | lingua (all langs) | 35k | 222 KB | 3 700.01 | 106 613 | — | — | — |
| Python | lingua (all langs) | 87k | 620 KB | 2 675.98 | 30 893 | — | — | — |
| Node | papagan | 35k | 222 KB | **49.90** | **1 438** | **30.06** | **866** | 27.17 |
| Node | papagan | 87k | 620 KB | **91.58** | **1 057** | **30.65** | **354** | 28.21 |

`Loop` is a serial `for ... detect()` over the whole fixture. `Batch` submits it in one call with rayon fan-out inside. `Async` runs on libuv's thread pool (Node only) — wall time is ~same as sync but the V8 event loop stays responsive (see `papagan-node/examples/event-loop-latency.js`). Competitor libraries don't have batch APIs, so their batch columns are empty; the comparison is on `Loop (ns/tok)`.

**Reading the ns/token column:**

- **papagan is 900–950 ns/token on loop, ~270 ns/token on batch** — throughput is consistent across workload size (flat from 35k to 87k tokens), so the rate is a real throughput number.
- **py3langid is 6 442 ns/tok on short sentences, 1 395 on paragraphs** — huge fixed per-call overhead that amortizes as inputs grow.
- **langdetect is 200 526 ns/tok on short sentences** — catastrophically slow on short inputs (re-seeded Java-ported detector per call), recovers to 15 570 on longer documents.
- **lingua (all langs) is 106 613 → 30 893 ns/tok** — loads a vast model; throughput scales with input length but never catches the lightweight detectors.

Performance is essentially flat across natural languages — the tokenize fast-path for pure-ASCII inputs (en/es/it/nl/fr/pt) is marginally faster than the unicode path (de with umlauts, pl, ru, tr), but within ±20 % at worst.


### Accuracy by language — two independent corpora

Measured on Darwin arm64, 2026-04-22.

Tatoeba: **4,971 / 5,000 (99.42 %)**.  FLORES-200 devtest: **10,106 / 10,120 (99.86 %)**.

| Lang | Tatoeba acc (n) | FLORES acc (n) | Common miss (predicted instead) |
|---|---:|---:|---|
| de | 100.0 % (500) | 99.9 % (1012) | → nl (1) |
| en | 99.2 % (500) | 99.9 % (1012) | → it (2), fr (1) |
| es | 98.0 % (500) | 99.6 % (1012) | → pt (6), it (5) |
| fr | 99.4 % (500) | 99.7 % (1012) | → en (2), it (2) |
| it | 99.6 % (500) | 99.9 % (1012) | → tr (2), nl (1) |
| nl | 99.8 % (500) | 99.9 % (1012) | → en (1), es (1) |
| pl | 99.6 % (500) | 100.0 % (1012) | → it (1), pt (1) |
| pt | 98.6 % (500) | 99.7 % (1012) | → it (4), es (3) |
| ru | 100.0 % (500) | 100.0 % (1012) | — |
| tr | 100.0 % (500) | 100.0 % (1012) | — |

Tatoeba is the training-distribution-adjacent eval (subtitle-shaped, close to the OpenSubtitles corpus the FrequencyWords model was trained on); FLORES-200 devtest is the OOD robustness eval (Wikipedia/news translations, unseen during training). papagan clears 98 % on every supported language on both fixtures; the weakest cell is Spanish/Portuguese on Tatoeba due to Romance cluster overlap.

### How to reproduce benchmarks

Two commands regenerate every number in this section. Commit the outputs when you release.

```bash
# Accuracy (both fixtures):
cargo run --release --example accuracy_matrix --features all-langs

# Performance matrix — Rust + Python (including competitors, if installed) + Node:
./scripts/bench-matrix.sh
```

Prereqs for the orchestrator:
- `papagan/tests/fixtures/accuracy_large.tsv` — regenerate via `cargo xtask fetch-eval`
- `bench/paragraphs.json` — regenerate via `cargo xtask fetch-leipzig`
- `papagan-py/.venv` with `maturin develop --release` (for the Python rows)
- `papagan-node/` built via `npm run build:platform` (for the Node rows)

To include Python competitor rows in the matrix:

```bash
papagan-py/.venv/bin/pip install py3langid langdetect lingua-language-detector
```

Accuracy fixtures: `accuracy_flores.tsv` is committed; `accuracy_large.tsv` (Tatoeba) is gitignored and regenerated via `cargo xtask fetch-eval`.

## How it works

1. **Tokenize** the input with Unicode-aware segmentation + NFKC normalization + default lowercase (preserves Turkish `İ`/`ı`/`I`/`i` as distinct).
2. **Stage 1** — for each token, binary-search a sorted table of `(word, lang, rank)` triples pulled from the top-N frequency list of each enabled language. Hits yield rank-weighted priors: `P(lang|word) ∝ 1/(rank + k)`.
3. **Stage 2** (fallback for tokens that missed) — score against per-language character-trigram models (PHF perfect-hash at compile time, smoothed log-probabilities), softmax across enabled languages.
4. **Aggregate** — confidence-weighted combination of per-word distributions into a document-level distribution. High-confidence tokens (peaky distributions) dominate; uniform distributions contribute little.

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.
