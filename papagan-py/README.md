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
    parallel_threshold=128,         # parallelize at 128+ words
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

## Accuracy

~99.4% on a 5000-sentence Tatoeba evaluation across the 10 supported languages. Per-language precision/recall is best on isolated scripts (Russian, Turkish — perfect) and slightly weaker on the close Iberian pair (Spanish/Portuguese).

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.

## Related

- [Rust crate](https://crates.io/crates/papagan) — the core library
- [Node.js package](https://www.npmjs.com/package/papagan) — Node.js bindings
- [GitHub](https://github.com/suleymanozkeskin/papagan) — source, issues, development
