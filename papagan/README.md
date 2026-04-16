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

### Builder

```rust
use papagan::{Detector, Lang};

let detector = Detector::builder()
    .only([Lang::En, Lang::De])       // restrict to a subset
    .unknown_threshold(0.25)           // below this => Lang::Unknown
    .parallel_threshold(128)           // parallelize at 128+ words
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

How many top-frequency words get baked into the binary per language. Default is 3k — the empirical accuracy knee on a 5000-sentence Tatoeba eval.

| Setting | Binary (all-langs) | Accuracy |
|---|---|---|
| `PAPAGAN_DICT_SIZE=1000` env var | 3.3 MB | 99.1% |
| **default (3k)** | **4.9 MB** | **99.4%** |
| `features = ["dict-5k"]` | 6.5 MB | 99.4% |
| `features = ["dict-10k"]` | 10.5 MB | 99.7% |

Env var overrides feature choice. Larger dict = fewer trigram fallbacks + better classification of rare words.

### Parallelism

- `parallel` (default, behind `dep:rayon`) — per-word scoring runs on a rayon thread pool for inputs at or above `parallel_threshold` (default 64 words).
- Opt out with `default-features = false, features = ["en"]` to get a serial, no-rayon build for embedded/wasm/minimal binaries.

## API

```rust
pub struct Detector { /* ... */ }
pub struct DetectorBuilder { /* ... */ }

pub struct Output      { /* top(), distribution() */ }
pub struct Detailed    { pub words: Vec<WordScore>, pub aggregate: Output }
pub struct WordScore   { pub token: Box<str>, pub scores: SmallVec<[(Lang, f32); 8]>, pub source: MatchSource }

pub enum Lang          { De, En, Tr, Ru, Fr, Es, It, Nl, Pt, Pl, Unknown /* cfg-gated per feature */ }
pub enum MatchSource   { Dict, Ngram, Unknown }
```

See [`docs.rs/papagan`](https://docs.rs/papagan) for full signatures.

## How it works

1. **Tokenize** input with `unicode-segmentation` + NFKC + Unicode default lowercase. Preserves Turkish `İ`/`I`/`ı`/`i` distinctions without needing locale-aware casing at detection time.
2. **Stage 1** — binary-search a sorted `(word, lang, rank)` table baked at compile time. Hits yield `P(lang|word) ∝ 1/(rank + 10)`, so stopword-like signals (high-frequency function words) dominate — which is exactly the signal we want for language detection.
3. **Stage 2** — character trigrams with `^^word$$` boundary padding. Per-language PHF maps compiled via `phf_codegen`; missing trigrams fall to a smoothed floor. Softmax per-word, then aggregate across tokens confidence-weighted (`token_weight = max(scores)`).

Training pipeline (`cargo xtask`) fetches frequency lists from hermitdave/FrequencyWords and emits the `data/` artifacts consumed by `build.rs`.

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.
