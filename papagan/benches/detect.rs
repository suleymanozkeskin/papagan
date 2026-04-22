use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use papagan::Detector;

fn load_titles() -> Vec<String> {
    let path = env::var("PAPAGAN_BENCH_TITLES")
        .map(PathBuf::from)
        .or_else(|_| {
            let repo_root = env!("CARGO_MANIFEST_DIR");
            let p = PathBuf::from(repo_root).join("../bench/titles.json");
            if p.exists() { Ok(p) } else { Err(()) }
        })
        .expect(
            "set PAPAGAN_BENCH_TITLES=/path/to/titles.json or place the file at ../bench/titles.json",
        );

    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|_| {
        raw.lines()
            .filter(|l| !l.is_empty())
            .map(|l| {
                l.trim_matches(|c: char| c == '"' || c == ',' || c.is_whitespace())
                    .to_string()
            })
            .filter(|l| !l.is_empty())
            .collect()
    })
}

// Long-form sweep. Loads `bench/paragraphs.json` (Leipzig-derived, built via
// `cargo xtask fetch-leipzig`). Missing file → bench group is skipped so
// developers without the fixture can still run the titles bench.
fn load_paragraphs() -> Option<Vec<String>> {
    let path = env::var("PAPAGAN_BENCH_PARAGRAPHS")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bench/paragraphs.json");
            p.exists().then_some(p)
        })?;
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn unique_preserving(titles: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for t in titles {
        if seen.insert(t.as_str().to_string()) {
            out.push(t.clone());
        }
    }
    out
}

fn bench_detector(c: &mut Criterion) {
    let titles = load_titles();
    let uniq = unique_preserving(&titles);
    let detector = Detector::new();

    // Warm up the dict + ngram tables (first call may pay cold-cache cost).
    for t in uniq.iter().take(20) {
        let _ = detector.detect(t);
    }

    let mut single = c.benchmark_group("detect_single");
    single.throughput(Throughput::Elements(1));
    // Median-ish title from the bench set.
    let representative = uniq
        .iter()
        .find(|t| t.len() > 20 && t.len() < 60)
        .cloned()
        .unwrap_or_else(|| uniq[0].clone());
    single.bench_function("representative", |b| {
        b.iter(|| detector.detect(std::hint::black_box(&representative)))
    });
    single.finish();

    let mut uniq_group = c.benchmark_group("detect_unique_sweep");
    uniq_group.throughput(Throughput::Elements(uniq.len() as u64));
    uniq_group.bench_function("uniq_239", |b| {
        b.iter_batched(
            || (),
            |_| {
                for t in &uniq {
                    let _ = detector.detect(std::hint::black_box(t));
                }
            },
            BatchSize::SmallInput,
        )
    });
    uniq_group.finish();

    let mut full = c.benchmark_group("detect_full_sweep");
    full.throughput(Throughput::Elements(titles.len() as u64));
    full.bench_function("full_1870", |b| {
        b.iter_batched(
            || (),
            |_| {
                for t in &titles {
                    let _ = detector.detect(std::hint::black_box(t));
                }
            },
            BatchSize::SmallInput,
        )
    });
    full.finish();

    // Long-form regime: paragraphs concatenated from Leipzig news sentences.
    // Exercises the aggregation path and tips into the parallel code path
    // for inputs with >64 words.
    if let Some(paragraphs) = load_paragraphs() {
        for p in paragraphs.iter().take(20) {
            let _ = detector.detect(p);
        }

        let mut para = c.benchmark_group("detect_paragraph_single");
        para.throughput(Throughput::Elements(1));
        let representative = paragraphs
            .iter()
            .find(|p| {
                let w = p.split_whitespace().count();
                (60..120).contains(&w)
            })
            .cloned()
            .unwrap_or_else(|| paragraphs[0].clone());
        para.bench_function("representative", |b| {
            b.iter(|| detector.detect(std::hint::black_box(&representative)))
        });
        para.finish();

        let label = format!("all_{}", paragraphs.len());
        let mut para_sweep = c.benchmark_group("detect_paragraph_sweep");
        para_sweep.throughput(Throughput::Elements(paragraphs.len() as u64));
        para_sweep.bench_function(&label, |b| {
            b.iter_batched(
                || (),
                |_| {
                    for p in &paragraphs {
                        let _ = detector.detect(std::hint::black_box(p));
                    }
                },
                BatchSize::SmallInput,
            )
        });
        para_sweep.finish();
    }
}

criterion_group!(benches, bench_detector);
criterion_main!(benches);
