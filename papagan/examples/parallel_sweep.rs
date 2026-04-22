//! Sweep `parallel_threshold` on the long-form paragraph bench to see where
//! (if anywhere) rayon actually pays for itself.
//!
//! Usage:
//!   cargo run --release --example parallel_sweep --features all-langs
//!
//! Optional positional arg: path to a JSON array of strings (defaults to
//! bench/paragraphs.json).

use std::fs;
use std::time::Instant;

use papagan::{Detector, Lang};

const ITERS: usize = 7;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "bench/paragraphs.json".to_string());
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });
    let paragraphs: Vec<String> = serde_json::from_str(&raw).unwrap_or_else(|e| {
        eprintln!("invalid JSON array at {path}: {e}");
        std::process::exit(1);
    });
    let word_counts: Vec<usize> = paragraphs
        .iter()
        .map(|p| p.split_whitespace().count())
        .collect();
    let mean_words = word_counts.iter().sum::<usize>() as f64 / word_counts.len().max(1) as f64;
    let median_words = median(&word_counts);
    eprintln!(
        "loaded {} paragraphs: mean={mean_words:.0} words, median={median_words} words",
        paragraphs.len()
    );

    // Always-serial: a practically-infinite threshold means rayon never fires.
    // Always-parallel: threshold 0.
    let cases: &[(&str, usize)] = &[
        ("always serial (no rayon)", usize::MAX),
        ("thr=1024 (rarely parallel)", 1024),
        ("thr=256", 256),
        ("thr=128", 128),
        ("thr=64 (prior default)", 64),
        ("thr=32 (current default)", 32),
        ("thr=16", 16),
        ("always parallel (thr=0)", 0),
    ];

    println!("| Threshold | Mean / call (µs) | Median / call (µs) | Full sweep (ms) |");
    println!("|---|---:|---:|---:|");

    for (label, thr) in cases {
        let d = Detector::builder()
            .only(Lang::all_enabled().iter().copied())
            .parallel_threshold(*thr)
            .build();
        // Warmup to prime caches + rayon worker pool.
        for p in paragraphs.iter().take(20) {
            let _ = d.detect(p);
        }

        let mut sweep_nanos: Vec<u128> = Vec::with_capacity(ITERS);
        for _ in 0..ITERS {
            let start = Instant::now();
            for p in &paragraphs {
                let _ = d.detect(std::hint::black_box(p));
            }
            sweep_nanos.push(start.elapsed().as_nanos());
        }
        sweep_nanos.sort();
        let median_sweep_nanos = sweep_nanos[sweep_nanos.len() / 2];
        let sweep_ms = median_sweep_nanos as f64 / 1_000_000.0;
        let per_call_us = median_sweep_nanos as f64 / 1000.0 / paragraphs.len() as f64;

        // Also measure a per-call median by timing each call individually.
        // Less noise-proof than the sweep but gives us the distribution.
        let mut per_call_nanos: Vec<u128> = Vec::with_capacity(paragraphs.len());
        for p in &paragraphs {
            let start = Instant::now();
            let _ = d.detect(std::hint::black_box(p));
            per_call_nanos.push(start.elapsed().as_nanos());
        }
        per_call_nanos.sort();
        let median_per_call_us = per_call_nanos[per_call_nanos.len() / 2] as f64 / 1000.0;

        println!("| {label} | {per_call_us:.2} | {median_per_call_us:.2} | {sweep_ms:.2} |");
    }
}

fn median(xs: &[usize]) -> usize {
    if xs.is_empty() {
        return 0;
    }
    let mut sorted = xs.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}
