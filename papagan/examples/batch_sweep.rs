//! Compare serial for-loop vs `detect_batch` on both short-title and
//! long-form paragraph inputs. Measures speedup and scaling vs the number
//! of physical cores available.
//!
//! Usage:
//!   cargo run --release --example batch_sweep --features all-langs

use std::fs;
use std::time::Instant;

use papagan::Detector;

const ITERS: usize = 5;

fn main() {
    let paragraphs = load_json("bench/paragraphs.json");
    let titles = load_json("bench/titles.json");

    eprintln!(
        "loaded {} paragraphs (avg {:.0} chars) and {} titles (avg {:.0} chars)",
        paragraphs.len(),
        avg_chars(&paragraphs),
        titles.len(),
        avg_chars(&titles),
    );
    eprintln!("available parallelism: {:?}", std::thread::available_parallelism());
    eprintln!();

    let detector = Detector::new();
    let para_refs: Vec<&str> = paragraphs.iter().map(String::as_str).collect();
    let title_refs: Vec<&str> = titles.iter().map(String::as_str).collect();

    // Warm up — load tables, spin up rayon thread pool, let caches settle.
    let _ = detector.detect_batch(&para_refs[..20]);
    let _ = detector.detect_batch(&title_refs[..20]);

    println!("## Paragraphs ({} items, median ~84 words)", paragraphs.len());
    println!();
    report(&detector, &para_refs);

    println!();
    println!("## Titles ({} items, median ~8 words)", titles.len());
    println!();
    report(&detector, &title_refs);

    println!();
    println!("## Batch-size sweep (paragraphs)");
    println!();
    println!("| Batch size | Serial loop (ms) | detect_batch (ms) | Speedup |");
    println!("|---:|---:|---:|---:|");
    for &n in &[1usize, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1000] {
        if n > para_refs.len() {
            continue;
        }
        let sample = &para_refs[..n];
        let serial_ms = median_ms(ITERS, || {
            for s in sample {
                let _ = detector.detect(std::hint::black_box(s));
            }
        });
        let batch_ms = median_ms(ITERS, || {
            let _ = detector.detect_batch(std::hint::black_box(sample));
        });
        let speedup = serial_ms / batch_ms;
        println!("| {n} | {serial_ms:.3} | {batch_ms:.3} | {speedup:.2}× |");
    }
}

fn report(detector: &Detector, inputs: &[&str]) {
    let n = inputs.len();
    let serial_ms = median_ms(ITERS, || {
        for s in inputs {
            let _ = detector.detect(std::hint::black_box(s));
        }
    });
    let batch_ms = median_ms(ITERS, || {
        let _ = detector.detect_batch(std::hint::black_box(inputs));
    });

    println!("| Path | Total (ms) | Per call (µs) | Throughput (K/s) |");
    println!("|---|---:|---:|---:|");
    println!(
        "| Serial `for` loop calling `detect()` | {:.2} | {:.2} | {:.1} |",
        serial_ms,
        serial_ms * 1000.0 / n as f64,
        n as f64 / (serial_ms / 1000.0) / 1000.0,
    );
    println!(
        "| `detect_batch(..)` | {:.2} | {:.2} | {:.1} |",
        batch_ms,
        batch_ms * 1000.0 / n as f64,
        n as f64 / (batch_ms / 1000.0) / 1000.0,
    );
    println!("| **Speedup** | | | **{:.2}×** |", serial_ms / batch_ms);
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

fn load_json(path: &str) -> Vec<String> {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });
    serde_json::from_str(&raw).unwrap_or_else(|e| {
        eprintln!("failed to parse {path}: {e}");
        std::process::exit(1);
    })
}

fn avg_chars(xs: &[String]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().map(|s| s.len()).sum::<usize>() as f64 / xs.len() as f64
    }
}
