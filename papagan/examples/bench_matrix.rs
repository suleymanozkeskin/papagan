//! Rust-side rows of the binding × API × workload performance matrix.
//!
//! Uses the two **open** fixtures: Tatoeba short sentences
//! (`papagan/tests/fixtures/accuracy_large.tsv`, regenerate via
//! `cargo xtask fetch-eval`) and Leipzig news paragraphs
//! (`bench/paragraphs.json`, regenerate via `cargo xtask fetch-leipzig`).
//!
//! Companion harnesses: `papagan-py/bench/matrix.py`,
//! `papagan-node/examples/bench-matrix.js`. Orchestrator:
//! `scripts/bench-matrix.sh`.
//!
//! Usage:
//!   cargo run --release --example bench_matrix --features all-langs

use std::fs;
use std::time::Instant;

use papagan::Detector;

// Matches the Python/Node harnesses: median of 7 full-sweep iterations.
const ITERS: usize = 7;

fn main() {
    let tatoeba = load_tsv("papagan/tests/fixtures/accuracy_large.tsv");
    let paragraphs = load_json("bench/paragraphs.json");

    let d = Detector::new();
    // Warm the rayon pool + dict/ngram tables on both shapes.
    for t in tatoeba.iter().take(20) {
        let _ = d.detect(t);
    }
    for p in paragraphs.iter().take(20) {
        let _ = d.detect(p);
    }

    let tat_refs: Vec<&str> = tatoeba.iter().map(String::as_str).collect();
    let para_refs: Vec<&str> = paragraphs.iter().map(String::as_str).collect();

    let tat_loop = bench(|| {
        for s in &tat_refs {
            let _ = d.detect(std::hint::black_box(s));
        }
    });
    let tat_batch = bench(|| {
        let _ = d.detect_batch(std::hint::black_box(&tat_refs));
    });
    let para_loop = bench(|| {
        for s in &para_refs {
            let _ = d.detect(std::hint::black_box(s));
        }
    });
    let para_batch = bench(|| {
        let _ = d.detect_batch(std::hint::black_box(&para_refs));
    });

    let (tat_tokens, tat_bytes) = fixture_stats(&tatoeba);
    let (para_tokens, para_bytes) = fixture_stats(&paragraphs);

    println!(
        "| Rust | papagan | {} | {} | {tat_loop:.2} | {} | {tat_batch:.2} | {} | — |",
        fmt_count(tat_tokens),
        fmt_kb(tat_bytes),
        ns_per_token(tat_loop, tat_tokens),
        ns_per_token(tat_batch, tat_tokens),
    );
    println!(
        "| Rust | papagan | {} | {} | {para_loop:.2} | {} | {para_batch:.2} | {} | — |",
        fmt_count(para_tokens),
        fmt_kb(para_bytes),
        ns_per_token(para_loop, para_tokens),
        ns_per_token(para_batch, para_tokens),
    );
}

// Average ns per token for a full-sweep timing.
fn ns_per_token(ms: f64, tokens: usize) -> String {
    if tokens == 0 {
        return "—".to_string();
    }
    format!("{:.0}", ms * 1_000_000.0 / tokens as f64)
}

fn fixture_stats(items: &[String]) -> (usize, usize) {
    let tokens = items.iter().map(|s| s.split_whitespace().count()).sum();
    let bytes = items.iter().map(|s| s.len()).sum();
    (tokens, bytes)
}

fn fmt_count(n: usize) -> String {
    if n >= 1_000 {
        format!("{}k", (n + 500) / 1_000)
    } else {
        n.to_string()
    }
}

fn fmt_kb(bytes: usize) -> String {
    format!("{} KB", (bytes + 500) / 1_000)
}

fn bench(mut f: impl FnMut()) -> f64 {
    let mut samples: Vec<u128> = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let start = Instant::now();
        f();
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort();
    samples[samples.len() / 2] as f64 / 1_000_000.0
}

fn load_json(path: &str) -> Vec<String> {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}\nhint: regenerate with `cargo xtask fetch-leipzig`.");
        std::process::exit(1);
    });
    serde_json::from_str(&raw).unwrap()
}

// Tatoeba TSV format: "<lang>\t<sentence>" per line. Drop labels, keep text.
fn load_tsv(path: &str) -> Vec<String> {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}\nhint: regenerate with `cargo xtask fetch-eval`.");
        std::process::exit(1);
    });
    raw.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t').map(|(_lang, text)| text.to_string()))
        .collect()
}
