//! End-to-end comparison of four batch-routing strategies on the real
//! fixtures (titles.json, paragraphs.json). The prior `batch_threshold_sweep.rs`
//! benchmarked routing choices in isolation; this one actually wires each
//! strategy into the detection path and measures wall-clock per call.
//!
//! Strategies:
//!   A — fixed  thr=4 (original default)
//!   B — fixed  thr=8 (conservative single-threshold choice)
//!   C — dynamic: N ≥ 2 AND approx_tokens ≥ 50 (shipped routing)
//!   D — always parallel (lower bound for "best case if rayon always helps")
//!
//! Usage:
//!   cargo run --release --example batch_strategy_bench --features all-langs

use std::fs;
use std::time::Instant;

use papagan::{Detector, Lang};

const ITERS: usize = 7;

fn main() {
    let paragraphs = load("bench/paragraphs.json");
    let titles = load("bench/titles.json");

    // Warm the rayon pool + caches.
    let warm = Detector::new();
    for s in paragraphs.iter().take(20) {
        let _ = warm.detect(s);
    }

    for (label, docs) in [("paragraphs", &paragraphs), ("titles", &titles)] {
        println!("## {label} — {} items", docs.len());
        println!("measuring 4 strategies × batch sizes {{1, 4, 8, 32, 128, full}}");
        println!();
        println!("| N | serial (ms) | A: thr=4 | B: thr=8 | C: dynamic | D: always-par |");
        println!("|---:|---:|---:|---:|---:|---:|");
        for &n in &[1usize, 4, 8, 32, 128, docs.len()] {
            if n == 0 || n > docs.len() {
                continue;
            }
            let sample: Vec<&str> = docs.iter().take(n).map(String::as_str).collect();

            let serial_ms = bench_serial(&sample);
            let a_ms = bench_strategy(&sample, Strategy::Fixed(4));
            let b_ms = bench_strategy(&sample, Strategy::Fixed(8));
            let c_ms = bench_strategy(&sample, Strategy::Dynamic);
            let d_ms = bench_strategy(&sample, Strategy::AlwaysParallel);

            println!(
                "| {n} | {serial_ms:.2} | {a_ms:.2} ({:.2}×) | {b_ms:.2} ({:.2}×) | {c_ms:.2} ({:.2}×) | {d_ms:.2} ({:.2}×) |",
                a_ms / serial_ms,
                b_ms / serial_ms,
                c_ms / serial_ms,
                d_ms / serial_ms,
            );
        }
        println!();
    }
}

#[derive(Clone, Copy)]
enum Strategy {
    Fixed(usize),
    Dynamic,
    AlwaysParallel,
}

fn bench_serial(sample: &[&str]) -> f64 {
    let d = Detector::new();
    median_ms(ITERS, || {
        for s in sample {
            let _ = d.detect(std::hint::black_box(s));
        }
    })
}

fn bench_strategy(sample: &[&str], strat: Strategy) -> f64 {
    let d = Detector::builder()
        .only(Lang::all_enabled().iter().copied())
        .build();
    let d_serial_intra = Detector::builder()
        .only(Lang::all_enabled().iter().copied())
        .parallel_threshold(usize::MAX)
        .build();

    median_ms(ITERS, || {
        let go_parallel = match strat {
            Strategy::Fixed(thr) => sample.len() >= thr,
            Strategy::Dynamic => {
                let n = sample.len();
                let tokens: usize = sample.iter().map(|s| s.split_whitespace().count()).sum();
                n >= 2 && tokens >= 50
            }
            Strategy::AlwaysParallel => true,
        };
        if !go_parallel {
            for s in sample {
                let _ = d.detect(std::hint::black_box(s));
            }
        } else {
            use rayon::prelude::*;
            let _: Vec<_> = sample
                .par_iter()
                .map(|s| d_serial_intra.detect(std::hint::black_box(s)))
                .collect();
        }
    })
}

fn median_ms(iters: usize, mut f: impl FnMut()) -> f64 {
    let mut samples: Vec<u128> = Vec::with_capacity(iters);
    for _ in 0..iters {
        let start = Instant::now();
        f();
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort();
    samples[samples.len() / 2] as f64 / 1_000_000.0
}

fn load(path: &str) -> Vec<String> {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        std::process::exit(1);
    });
    serde_json::from_str(&raw).unwrap()
}
