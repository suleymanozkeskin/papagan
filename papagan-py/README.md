# papagan

[![PyPI](https://img.shields.io/pypi/v/papagan.svg)](https://pypi.org/project/papagan/)
[![Python versions](https://img.shields.io/pypi/pyversions/papagan.svg)](https://pypi.org/project/papagan/)

Fast language detection for Python, powered by Rust (via [PyO3](https://pyo3.rs) + [maturin](https://www.maturin.rs/)).

10 languages bundled, weighted per-word output, fully typed (PEP 561).

## Install

```bash
uv add papagan
# or
pip install papagan
```

Pre-built wheels ship for Linux (x86_64, aarch64), macOS (x86_64, arm64), and Windows (x86_64). Python 3.10+.

## Quick start

```python
from papagan import Detector

detector = Detector()

# Document-level detection
output = detector.detect("Die Katze sitzt auf der Matte")
lang, confidence = output.top()
print(f"{lang}: {confidence:.3f}")
# de: 0.996

# Full distribution
for lang, score in output.distribution():
    print(f"  {lang}: {score:.3f}")
```

### Per-word detail

Useful for mixed-language text or debugging:

```python
detailed = detector.detect_detailed("The cat is black. Die Katze ist schwarz.")

for word in detailed.words:
    top_lang, top_score = max(word.scores, key=lambda x: x[1])
    print(f"  {word.token:<10} [{word.source}]  {top_lang} ({top_score:.2f})")
# the        [dict]   en (0.85)
# cat        [ngram]  en (0.99)
# ...
# katze      [ngram]  de (1.00)

# The aggregate handles mixed input gracefully:
print(detailed.aggregate.distribution())
# [('de', 0.52), ('en', 0.48)]
```

### Batch detection

For multi-document workloads, `detect_batch` fans out across cores via rayon **and releases the GIL** while running — so concurrent Python threads can do other work and scale-out on ThreadPoolExecutor behaves as expected:

```python
docs = ["The cat sat", "Die Katze sitzt", "Le chat est assis", "El gato está sentado"]

results = detector.detect_batch(docs)              # list[Output]
detailed = detector.detect_detailed_batch(docs)    # list[Detailed]

for o in results:
    print(o.top())
```

On a 1000-paragraph batch (Leipzig news, avg 84 words each, 8-core M-series), `detect_batch` is **~3.5× faster** than calling `detect()` in a Python loop — 90 ms → 26 ms. On 1870 short titles it's **~5× faster** (16 ms → 3 ms) since rayon setup amortizes better over dict-hit-heavy tokens.

Batches smaller than 4 fall back through the normal per-call path so there's no small-batch regression.

### Restrict to specific languages

Faster and more confident when you know the input's language set in advance:

```python
detector = Detector(only=["en", "de"])
# or with the builder:
detector = Detector.builder().only(["en", "de"]).build()
```

### Configuration

```python
detector = Detector(
    only=["en", "de", "fr"],       # restrict to a subset
    unknown_threshold=0.25,         # below this => ("?", ...) aka Lang.Unknown
    parallel_threshold=32,          # parallelize per-word work at 32+ tokens (default)
    # set parallel_threshold to a very large number to opt out of rayon entirely
)
```

## Supported languages

| Code | Language | Code | Language |
|---|---|---|---|
| `de` | German | `it` | Italian |
| `en` | English | `nl` | Dutch |
| `es` | Spanish | `pl` | Polish |
| `fr` | French | `pt` | Portuguese |
| `ru` | Russian | `tr` | Turkish |

All 10 languages are bundled — no feature flags to set.

## Type hints

The package ships `.pyi` stubs and a `py.typed` marker (PEP 561):

```python
from papagan import Detector, Lang, Output, WordScore, LangCode, MatchSource

def classify(text: str) -> LangCode:
    lang, _score = Detector().detect(text).top()
    return lang  # typed as Literal["de", "en", ..., "?"]
```

Your type checker (mypy, pyright) will see full signatures for all classes, including the `LangCode` and `MatchSource` Literal types.

## Benchmarks

Measured on Darwin arm64, 2026-04-22. Open fixtures: Tatoeba sentences (CC-BY 2.0 FR) and Leipzig news paragraphs (CC-BY 4.0). `ns/tok` is the per-token rate — the cleanest cross-library comparison since it normalizes out workload size. Full cross-binding matrix and reproduction commands live in the [repository README](https://github.com/suleymanozkeskin/papagan#benchmarks).

| Library | Tokens | Bytes | Loop (ms) | Loop (ns/tok) | Batch (ms) | Batch (ns/tok) |
|---|---:|---:|---:|---:|---:|---:|
| **papagan** | 35k | 222 KB | **32.94** | **949** | **9.67** | **279** |
| **papagan** | 87k | 620 KB | **79.94** | **923** | **23.74** | **274** |
| py3langid | 35k | 222 KB | 223.59 | 6 442 | — | — |
| py3langid | 87k | 620 KB | 120.81 | 1 395 | — | — |
| langdetect | 35k | 222 KB | 6 959.25 | 200 526 | — | — |
| langdetect | 87k | 620 KB | 1 348.72 | 15 570 | — | — |
| lingua (all langs) | 35k | 222 KB | 3 700.01 | 106 613 | — | — |
| lingua (all langs) | 87k | 620 KB | 2 675.98 | 30 893 | — | — |

papagan chews through ~950 ns/token on loop and ~275 ns/token with `detect_batch` — flat across workload size, **~7× ahead of py3langid** on short sentences, **~110× ahead of lingua**, **~210× ahead of langdetect**. `detect_batch` releases the GIL so `ThreadPoolExecutor` scales as expected.

## Accuracy

**99.42 %** on Tatoeba (5,000 sentences) and **99.86 %** on FLORES-200 devtest (10,120 sentences) across the 10 supported languages. Per-language precision/recall is best on isolated scripts (Russian, Turkish, Polish — ~perfect) and slightly weaker on the close Romance cluster (Spanish/Portuguese/Italian); full per-language table in the [repository README](https://github.com/suleymanozkeskin/papagan#accuracy-by-language--two-independent-corpora).

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.

## Related

- [Rust crate](https://crates.io/crates/papagan) — the core library
- [Node.js package](https://www.npmjs.com/package/papagan) — Node.js bindings
- [GitHub](https://github.com/suleymanozkeskin/papagan) — source, issues, development
