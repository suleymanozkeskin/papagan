# papagan

[![crates.io](https://img.shields.io/crates/v/papagan.svg)](https://crates.io/crates/papagan)
[![docs.rs](https://img.shields.io/docsrs/papagan)](https://docs.rs/papagan)

Fast, small-binary language detection. Two-stage pipeline: sorted-table dictionary lookup for top-frequency words, character-trigram fallback for the rest. Opt-in language packs at compile time. Weighted per-word output.

## Install

```toml
[dependencies]
papagan = { version = "0.1", features = ["all-langs"] }
```

The default features ship only English. Enable the languages you need:

```toml
papagan = { version = "0.1", features = ["en", "de", "tr"] }
```

Or take everything:

```toml
papagan = { version = "0.1", features = ["all-langs"] }
```

## Quick start

```rust
use papagan::{Detector, Lang};

let detector = Detector::new();
let output = detector.detect("Die Katze sitzt auf der Matte");
let (lang, confidence) = output.top();
assert_eq!(lang, Lang::De);
println!("{}: {:.3}", lang.iso_639_1(), confidence);
```

### Per-word detail

```rust
let detailed = detector.detect_detailed("The cat is black. Die Katze ist schwarz.");
for word in &detailed.words {
    let (top_lang, top_score) = word.scores.iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .copied()
        .unwrap_or((Lang::Unknown, 0.0));
    println!("  {:<10} [{:?}] {:?} ({:.2})",
             word.token, word.source, top_lang, top_score);
}

// Document-level aggregate:
let (top_lang, top_score) = detailed.aggregate.top();
```

### Batch detection

For multi-document workloads, `detect_batch` fans out across cores via rayon — ~3× on 1000 Leipzig paragraphs, ~5× on 1870 short titles, on an 8-core M-series.

```rust
let docs = vec!["the cat sat", "die katze sitzt", "le chat est assis", "el gato está sentado"];
let results = detector.detect_batch(&docs);                 // Vec<Output>
let detailed = detector.detect_detailed_batch(&docs);       // Vec<Detailed>
```

Works with `&[&str]`, `Vec<String>`, `&[String]` (anything `AsRef<str> + Sync`). Batches of fewer than 4 inputs fall back through the per-call path, preserving intra-document parallelism. Setting `parallel_threshold(usize::MAX)` on the builder opts out of rayon at both levels.

### Builder

```rust
use papagan::{Detector, Lang};

let detector = Detector::builder()
    .only([Lang::En, Lang::De])       // restrict to a subset
    .unknown_threshold(0.25)           // below this => Lang::Unknown
    .parallel_threshold(128)           // per-word parallelism kicks in at 128+
    .build();
```

## Feature flags

### Languages

Each language is an independent feature — binary size scales linearly with enabled languages.

| Feature | Language | Feature | Language |
|---|---|---|---|
| `de` | German | `it` | Italian |
| `en` | English (default) | `nl` | Dutch |
| `es` | Spanish | `pl` | Polish |
| `fr` | French | `pt` | Portuguese |
| `ru` | Russian | `tr` | Turkish |
| `all-langs` | — enables all of the above — | | |

### Dictionary size tiers

How many top-frequency words get baked into the binary per language. Default is 3k — the empirical accuracy knee across two independent evals: Tatoeba (short, subtitle-style) and FLORES-200 devtest (long-form, Wikipedia/news).

| Setting | Binary (all-langs) | Tatoeba 5k | FLORES-200 devtest 10k |
|---|---|---|---|
| `PAPAGAN_DICT_SIZE=1000` env var | 3.3 MB | 99.1% | — |
| **default (3k)** | **4.9 MB** | **99.4%** | **99.9%** |
| `features = ["dict-5k"]` | 6.5 MB | 99.4% | — |
| `features = ["dict-10k"]` | 10.5 MB | 99.7% | — |

Env var overrides feature choice. Larger dict = fewer trigram fallbacks + better classification of rare words. Both fixtures are regeneratable via `cargo xtask fetch-eval` / `cargo xtask fetch-flores`.

### Parallelism

Two levels of rayon, both controlled by the `parallel` feature (default on, behind `dep:rayon`):

- **Intra-document** — per-word scoring parallelizes for inputs at or above `parallel_threshold` (default 32, tuned via `examples/parallel_sweep.rs` on Leipzig paragraphs and short titles).
- **Batch-level** — `detect_batch` / `detect_detailed_batch` fan out across documents when the batch is ≥ 4. Intra-document parallelism is forced serial inside the batch loop to avoid nested rayon.

Setting `parallel_threshold(usize::MAX)` on the builder disables rayon at both levels. Compile without the `parallel` feature (`default-features = false, features = ["en"]`) for a serial, no-rayon build for embedded/wasm/minimal binaries.

## API

```rust
pub struct Detector { /* ... */ }
impl Detector {
    pub fn detect(&self, input: &str) -> Output;
    pub fn detect_detailed(&self, input: &str) -> Detailed;
    pub fn detect_batch<S: AsRef<str> + Sync>(&self, inputs: &[S]) -> Vec<Output>;
    pub fn detect_detailed_batch<S: AsRef<str> + Sync>(&self, inputs: &[S]) -> Vec<Detailed>;
}
pub struct DetectorBuilder { /* only, unknown_threshold, parallel_threshold */ }

pub struct Output      { /* top(), distribution() */ }
pub struct Detailed    { pub words: Vec<WordScore>, pub aggregate: Output }
pub struct WordScore   { pub token: Box<str>, pub scores: SmallVec<[(Lang, f32); 8]>, pub source: MatchSource }

pub enum Lang          { De, En, Tr, Ru, Fr, Es, It, Nl, Pt, Pl, Unknown /* cfg-gated per feature */ }
pub enum MatchSource   { Dict, Ngram, Unknown }
```

See [`docs.rs/papagan`](https://docs.rs/papagan) for full signatures.

## Benchmarks

Measured on Darwin arm64, 2026-04-22, all-langs release build. Open fixtures: Tatoeba sentences (CC-BY 2.0 FR) and Leipzig news paragraphs (CC-BY 4.0). `ns/tok` is the per-token rate. Full cross-binding matrix (including Python competitor comparison) and reproduction commands in the [repository README](https://github.com/suleymanozkeskin/papagan#benchmarks).

| Tokens | Bytes | Loop (ms) | Loop (ns/tok) | Batch (ms) | Batch (ns/tok) |
|---:|---:|---:|---:|---:|---:|
| 35k | 222 KB | **31.25** | **900** | **7.98** | **230** |
| 87k | 620 KB | **77.82** | **898** | **23.04** | **266** |

~900 ns/token on loop and ~250 ns/token with `detect_batch` — flat throughput across workload size, ~4× speedup under rayon fan-out on 8 cores.

**Accuracy**: 99.42 % on Tatoeba (5,000 sentences) and 99.86 % on FLORES-200 devtest (10,120 sentences). Per-language table in the top-level README.

## How it works

1. **Tokenize** input with `unicode-segmentation` + NFKC + Unicode default lowercase. Preserves Turkish `İ`/`I`/`ı`/`i` distinctions without needing locale-aware casing at detection time.
2. **Stage 1** — binary-search a sorted `(word, lang, rank)` table baked at compile time. Hits yield `P(lang|word) ∝ 1/(rank + 10)`, so stopword-like signals (high-frequency function words) dominate — which is exactly the signal we want for language detection.
3. **Stage 2** — character trigrams with `^^word$$` boundary padding. Per-language PHF maps compiled via `phf_codegen`; missing trigrams fall to a smoothed floor. Softmax per-word, then aggregate across tokens confidence-weighted (`token_weight = max(scores)`).

Training pipeline (`cargo xtask`) fetches frequency lists from hermitdave/FrequencyWords and emits the `data/` artifacts consumed by `build.rs`.

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.
