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
- **Parallel per-word scoring** via rayon, with a word-count threshold so short inputs stay serial.

## Install

<table>
<tr><td><b>Rust</b></td><td><code>cargo add papagan --features all-langs</code></td></tr>
<tr><td><b>Python</b></td><td><code>pip install papagan</code></td></tr>
<tr><td><b>Node.js</b></td><td><code>npm install papagan</code></td></tr>
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

### Dictionary tiers (compile time, Rust crate only)

Controls how many top-frequency words get baked into the binary. The PyPI and npm distributions ship with `dict-5k` baked in by the packager; the raw Rust crate defaults to 3k.

| Tier | Flag | Binary (all-langs) | Accuracy on 5000-sentence eval |
|---|---|---|---|
| 1k | `PAPAGAN_DICT_SIZE=1000` | 3.3 MB | 99.1% |
| **3k** (default) | none | **4.9 MB** | **99.4%** |
| 5k | `--features dict-5k` | 6.5 MB | 99.4% |
| 10k | `--features dict-10k` | 10.5 MB | 99.7% |

### Parallelism

Parallel per-word scoring (rayon) fires automatically above 64 words of input. Tune or disable:

```rust
let d = Detector::builder().parallel_threshold(128).build();  // only parallelize 128+
```

Disable rayon entirely by building without the `parallel` feature.

## Supported languages

| Code | Language | Code | Language |
|---|---|---|---|
| `de` | German | `it` | Italian |
| `en` | English | `nl` | Dutch |
| `es` | Spanish | `pl` | Polish |
| `fr` | French | `pt` | Portuguese |
| `ru` | Russian | `tr` | Turkish |

Training corpus: [hermitdave/FrequencyWords](https://github.com/hermitdave/FrequencyWords) (OpenSubtitles 2018, MIT licensed).

## How it works

1. **Tokenize** the input with Unicode-aware segmentation + NFKC normalization + default lowercase (preserves Turkish `İ`/`ı`/`I`/`i` as distinct).
2. **Stage 1** — for each token, binary-search a sorted table of `(word, lang, rank)` triples pulled from the top-N frequency list of each enabled language. Hits yield rank-weighted priors: `P(lang|word) ∝ 1/(rank + k)`.
3. **Stage 2** (fallback for tokens that missed) — score against per-language character-trigram models (PHF perfect-hash at compile time, smoothed log-probabilities), softmax across enabled languages.
4. **Aggregate** — confidence-weighted combination of per-word distributions into a document-level distribution. High-confidence tokens (peaky distributions) dominate; uniform distributions contribute little.

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.
