//! Two-axis sweep for batch-parallelism routing.
//!
//! Axes:
//!   - batch cardinality N
//!   - per-input token count (short / medium / long)
//!
//! For each (N, length) cell, measures:
//!   - serial `for` loop calling `detect()`
//!   - rayon `par_iter()` with intra-doc serial forced
//!   - rayon `par_iter().with_min_len(2)` (pair-chunked)
//!
//! Output:
//!   grid of ratios (parallel / serial). <1 means parallel wins.
//!   Also prints a flat table sorted by total tokens so we can see
//!   whether the crossover is a clean function of total-work or not.
//!
//! Usage:
//!   cargo run --release --example batch_routing_sweep --features all-langs

use std::fs;
use std::time::Instant;

use papagan::{Detector, Lang};

const ITERS: usize = 7;

struct Bucket {
    label: &'static str,
    token_target: usize,
    samples: Vec<String>,
}

fn main() {
    let raw = fs::read_to_string("bench/paragraphs.json").unwrap_or_else(|e| {
        eprintln!("paragraphs.json: {e}");
        std::process::exit(1);
    });
    let paragraphs: Vec<String> = serde_json::from_str(&raw).unwrap();

    // Build three length buckets by truncating paragraphs to target word
    // counts. Using the same source text across buckets keeps language mix
    // constant — only per-input length varies.
    let buckets = vec![
        Bucket {
            label: "short  (7 words)",
            token_target: 7,
            samples: truncate_all(&paragraphs, 7),
        },
        Bucket {
            label: "medium (30 words)",
            token_target: 30,
            samples: truncate_all(&paragraphs, 30),
        },
        Bucket {
            label: "long   (85 words)",
            token_target: 85,
            samples: truncate_all(&paragraphs, 85),
        },
    ];

    let d = Detector::builder()
        .only(Lang::all_enabled().iter().copied())
        .build();
    let d_serial_intra = Detector::builder()
        .only(Lang::all_enabled().iter().copied())
        .parallel_threshold(usize::MAX)
        .build();

    // Warm.
    for b in &buckets {
        for s in b.samples.iter().take(20) {
            let _ = d.detect(s);
        }
    }

    let sizes: &[usize] = &[1, 2, 3, 4, 6, 8, 12, 16, 32, 64];

    // Flat results table.
    struct Cell {
        bucket_label: &'static str,
        n: usize,
        total_tokens: usize,
        serial_us_per_call: f64,
        par_us_per_call: f64,
        par_minlen2_us_per_call: f64,
    }
    let mut cells: Vec<Cell> = Vec::new();

    for b in &buckets {
        let refs: Vec<&str> = b.samples.iter().map(String::as_str).collect();
        for &n in sizes {
            if n > refs.len() {
                continue;
            }
            let sample = &refs[..n];

            let serial_us = median_us(ITERS, || {
                for s in sample {
                    let _ = d.detect(std::hint::black_box(s));
                }
            }) / n as f64;

            let par_us = median_us(ITERS, || {
                use rayon::prelude::*;
                let _: Vec<_> = sample
                    .par_iter()
                    .map(|s| d_serial_intra.detect(std::hint::black_box(s)))
                    .collect();
            }) / n as f64;

            let par_minlen2_us = median_us(ITERS, || {
                use rayon::prelude::*;
                let _: Vec<_> = sample
                    .par_iter()
                    .with_min_len(2)
                    .map(|s| d_serial_intra.detect(std::hint::black_box(s)))
                    .collect();
            }) / n as f64;

            cells.push(Cell {
                bucket_label: b.label,
                n,
                total_tokens: b.token_target * n,
                serial_us_per_call: serial_us,
                par_us_per_call: par_us,
                par_minlen2_us_per_call: par_minlen2_us,
            });
        }
    }

    // Grid view — one table per bucket.
    for b in &buckets {
        println!("## {}", b.label);
        println!("| N | total_tokens | serial | par | par/serial | par_minlen2 | minlen2/serial |");
        println!("|---:|---:|---:|---:|---:|---:|---:|");
        for c in cells.iter().filter(|c| c.bucket_label == b.label) {
            let ratio = c.par_us_per_call / c.serial_us_per_call;
            let ratio2 = c.par_minlen2_us_per_call / c.serial_us_per_call;
            let win = if ratio < 0.95 {
                "✓"
            } else if ratio > 1.05 {
                " "
            } else {
                "≈"
            };
            let win2 = if ratio2 < 0.95 {
                "✓"
            } else if ratio2 > 1.05 {
                " "
            } else {
                "≈"
            };
            println!(
                "| {} | {} | {:.2} | {:.2} | {:.2}× {} | {:.2} | {:.2}× {} |",
                c.n,
                c.total_tokens,
                c.serial_us_per_call,
                c.par_us_per_call,
                ratio,
                win,
                c.par_minlen2_us_per_call,
                ratio2,
                win2,
            );
        }
        println!();
    }

    // Flat view sorted by total_tokens — does the crossover line up?
    println!("## Crossover view — all cells sorted by total_tokens");
    println!("| total_tokens | bucket | N | par/serial | minlen2/serial |");
    println!("|---:|---|---:|---:|---:|");
    let mut sorted: Vec<&Cell> = cells.iter().collect();
    sorted.sort_by_key(|c| c.total_tokens);
    for c in sorted {
        let ratio = c.par_us_per_call / c.serial_us_per_call;
        let ratio2 = c.par_minlen2_us_per_call / c.serial_us_per_call;
        let mark = if ratio < 0.95 {
            "✓"
        } else if ratio > 1.05 {
            " "
        } else {
            "≈"
        };
        println!(
            "| {:>5} | {} | {:>3} | {:.2}× {} | {:.2}× |",
            c.total_tokens, c.bucket_label, c.n, ratio, mark, ratio2
        );
    }
}

fn truncate_all(docs: &[String], target_words: usize) -> Vec<String> {
    docs.iter()
        .filter_map(|d| {
            let words: Vec<&str> = d.split_whitespace().collect();
            if words.len() < target_words {
                return None;
            }
            Some(words[..target_words].join(" "))
        })
        .collect()
}

fn median_us(iters: usize, mut f: impl FnMut()) -> f64 {
    let mut samples: Vec<u128> = Vec::with_capacity(iters);
    for _ in 0..iters {
        let start = Instant::now();
        f();
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort();
    samples[samples.len() / 2] as f64 / 1000.0
}
