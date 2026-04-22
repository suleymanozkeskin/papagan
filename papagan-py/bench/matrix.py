"""Python-side rows of the binding × API × workload performance matrix.

Uses the two open fixtures: Tatoeba short sentences (accuracy_large.tsv,
regenerate via `cargo xtask fetch-eval`) and Leipzig news paragraphs
(bench/paragraphs.json, regenerate via `cargo xtask fetch-leipzig`).

Outputs rows for **papagan** plus — if installed — rows for each Python
competitor (py3langid, langdetect, lingua-language-detector). Competitors
only measure the detect() loop since they don't have batch APIs.

Usage:
    python papagan-py/bench/matrix.py

Optional competitor install:
    pip install py3langid langdetect lingua-language-detector

Companion harnesses: papagan/examples/bench_matrix.rs,
papagan-node/examples/bench-matrix.js. Orchestrator: scripts/bench-matrix.sh.
"""
import json
import sys
import time
import statistics
from pathlib import Path

from papagan import Detector

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
TATOEBA_PATH = REPO_ROOT / "papagan" / "tests" / "fixtures" / "accuracy_large.tsv"
PARAGRAPHS_PATH = REPO_ROOT / "bench" / "paragraphs.json"

ITERS = 7


def load_tsv(path: Path) -> list[str]:
    try:
        text = path.read_text()
    except FileNotFoundError:
        sys.stderr.write(
            f"{path}: missing\n"
            "hint: regenerate with `cargo xtask fetch-eval`.\n"
        )
        sys.exit(1)
    out: list[str] = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t", 1)
        if len(parts) == 2:
            out.append(parts[1])
    return out


def load_json(path: Path) -> list[str]:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        sys.stderr.write(
            f"{path}: missing\n"
            "hint: regenerate with `cargo xtask fetch-leipzig`.\n"
        )
        sys.exit(1)


def bench(fn) -> float:
    samples = []
    for _ in range(ITERS):
        t = time.perf_counter_ns()
        fn()
        samples.append(time.perf_counter_ns() - t)
    return statistics.median(samples) / 1e6  # → ms


def fixture_stats(items: list[str]) -> tuple[int, str, str]:
    tokens = sum(len(s.split()) for s in items)
    byt = sum(len(s.encode("utf-8")) for s in items)
    tok_str = f"{(tokens + 500) // 1000}k" if tokens >= 1000 else str(tokens)
    kb_str = f"{(byt + 500) // 1000} KB"
    return tokens, tok_str, kb_str


def ns_per_token(ms: float, tokens: int) -> str:
    if tokens == 0:
        return "—"
    return f"{round(ms * 1_000_000 / tokens)}"


def try_import(name: str):
    try:
        return __import__(name)
    except ImportError:
        return None


def row(lib: str, tokens: int, tok_str: str, kb_str: str,
        loop_ms, batch_ms, async_ms) -> str:
    """Emit a markdown row with wall time (ms) + derived ns/token per API."""
    loop_fmt = f"{loop_ms:.2f}" if isinstance(loop_ms, float) else str(loop_ms)
    loop_ns = ns_per_token(loop_ms, tokens) if isinstance(loop_ms, float) else "—"
    batch_fmt = f"{batch_ms:.2f}" if isinstance(batch_ms, float) else str(batch_ms)
    batch_ns = ns_per_token(batch_ms, tokens) if isinstance(batch_ms, float) else "—"
    async_fmt = f"{async_ms:.2f}" if isinstance(async_ms, float) else str(async_ms)
    return (
        f"| Python | {lib} | {tok_str} | {kb_str} | "
        f"{loop_fmt} | {loop_ns} | {batch_fmt} | {batch_ns} | {async_fmt} |"
    )


def bench_papagan(tatoeba, paragraphs, tat_stats, par_stats) -> list[str]:
    d = Detector()
    for t in tatoeba[:20]:
        _ = d.detect(t)
    for p in paragraphs[:20]:
        _ = d.detect(p)
    tl = bench(lambda: [d.detect(t) for t in tatoeba])
    tb = bench(lambda: d.detect_batch(tatoeba))
    pl = bench(lambda: [d.detect(p) for p in paragraphs])
    pb = bench(lambda: d.detect_batch(paragraphs))
    return [
        row("papagan", *tat_stats, tl, tb, "—"),
        row("papagan", *par_stats, pl, pb, "—"),
    ]


def bench_py3langid(tatoeba, paragraphs, tat_stats, par_stats) -> list[str]:
    mod = try_import("py3langid")
    if mod is None:
        return []
    classify = mod.classify
    for t in tatoeba[:20]:
        _ = classify(t)
    tl = bench(lambda: [classify(t) for t in tatoeba])
    pl = bench(lambda: [classify(p) for p in paragraphs])
    return [
        row("py3langid", *tat_stats, tl, "—", "—"),
        row("py3langid", *par_stats, pl, "—", "—"),
    ]


def bench_langdetect(tatoeba, paragraphs, tat_stats, par_stats) -> list[str]:
    mod = try_import("langdetect")
    if mod is None:
        return []
    from langdetect import DetectorFactory, detect
    DetectorFactory.seed = 0
    for t in tatoeba[:20]:
        try:
            _ = detect(t)
        except Exception:
            pass

    def safe_detect_all(items):
        out = []
        for item in items:
            try:
                out.append(detect(item))
            except Exception:
                out.append("?")
        return out

    tl = bench(lambda: safe_detect_all(tatoeba))
    pl = bench(lambda: safe_detect_all(paragraphs))
    return [
        row("langdetect", *tat_stats, tl, "—", "—"),
        row("langdetect", *par_stats, pl, "—", "—"),
    ]


def bench_lingua(tatoeba, paragraphs, tat_stats, par_stats) -> list[str]:
    mod = try_import("lingua")
    if mod is None:
        return []
    from lingua import LanguageDetectorBuilder
    detector = LanguageDetectorBuilder.from_all_languages().build()
    for t in tatoeba[:20]:
        _ = detector.detect_language_of(t)
    tl = bench(lambda: [detector.detect_language_of(t) for t in tatoeba])
    pl = bench(lambda: [detector.detect_language_of(p) for p in paragraphs])
    return [
        row("lingua", *tat_stats, tl, "—", "—"),
        row("lingua", *par_stats, pl, "—", "—"),
    ]


def main() -> None:
    tatoeba = load_tsv(TATOEBA_PATH)
    paragraphs = load_json(PARAGRAPHS_PATH)

    # Each stats tuple = (tokens_int, tokens_str, bytes_str).
    tat_stats = fixture_stats(tatoeba)
    par_stats = fixture_stats(paragraphs)

    for r in bench_papagan(tatoeba, paragraphs, tat_stats, par_stats):
        print(r)
    for bench_fn in (bench_py3langid, bench_langdetect, bench_lingua):
        for r in bench_fn(tatoeba, paragraphs, tat_stats, par_stats):
            print(r)


if __name__ == "__main__":
    main()
