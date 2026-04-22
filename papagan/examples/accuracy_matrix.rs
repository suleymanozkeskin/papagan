//! Generate a consolidated accuracy matrix across our two accuracy fixtures
//! (Tatoeba `accuracy_large.tsv` + FLORES-200 `accuracy_flores.tsv`).
//!
//! Output is a single markdown table with one row per supported language,
//! showing precision/recall on each fixture plus the most common
//! misclassification direction, so users can answer "how does papagan do
//! on my target language?" at a glance.
//!
//! Usage:
//!   cargo run --release --example accuracy_matrix --features all-langs
//!
//! Optional overrides:
//!   --tatoeba <path>    default papagan/tests/fixtures/accuracy_large.tsv
//!   --flores <path>     default papagan/tests/fixtures/accuracy_flores.tsv

use std::collections::BTreeMap;
use std::fs;

use papagan::Detector;

struct PerLang {
    total: u32,
    correct: u32,
    confusion: BTreeMap<String, u32>, // predicted → count when wrong
}

impl PerLang {
    fn new() -> Self {
        Self {
            total: 0,
            correct: 0,
            confusion: BTreeMap::new(),
        }
    }
    fn record(&mut self, predicted: &str, expected: &str) {
        self.total += 1;
        if predicted == expected {
            self.correct += 1;
        } else {
            *self.confusion.entry(predicted.to_string()).or_insert(0) += 1;
        }
    }
    fn accuracy(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            100.0 * self.correct as f32 / self.total as f32
        }
    }
}

fn run_fixture(path: &str) -> (usize, BTreeMap<String, PerLang>) {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });
    let detector = Detector::new();
    let mut per_lang: BTreeMap<String, PerLang> = BTreeMap::new();
    let mut total = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((iso, text)) = line.split_once('\t') else {
            continue;
        };
        let predicted = detector.detect(text).top().0.iso_639_1().to_string();
        per_lang
            .entry(iso.to_string())
            .or_insert_with(PerLang::new)
            .record(&predicted, iso);
        total += 1;
    }
    (total, per_lang)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut tatoeba = "papagan/tests/fixtures/accuracy_large.tsv".to_string();
    let mut flores = "papagan/tests/fixtures/accuracy_flores.tsv".to_string();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--tatoeba" => tatoeba = args.next().expect("--tatoeba needs a path"),
            "--flores" => flores = args.next().expect("--flores needs a path"),
            _ => {}
        }
    }

    let (tat_total, tat_per) = run_fixture(&tatoeba);
    let (flr_total, flr_per) = run_fixture(&flores);

    let tat_correct: u32 = tat_per.values().map(|p| p.correct).sum();
    let flr_correct: u32 = flr_per.values().map(|p| p.correct).sum();

    println!("## Accuracy by language — two independent corpora");
    println!();
    println!(
        "Tatoeba: {tat_correct}/{tat_total} ({:.2}% overall).  FLORES-200 devtest: {flr_correct}/{flr_total} ({:.2}% overall).",
        100.0 * tat_correct as f32 / tat_total as f32,
        100.0 * flr_correct as f32 / flr_total as f32,
    );
    println!();
    println!("| Lang | Tatoeba acc (n) | FLORES acc (n) | Common miss (predicted instead) |");
    println!("|---|---:|---:|---|");
    let mut all_langs: Vec<&String> = tat_per.keys().chain(flr_per.keys()).collect();
    all_langs.sort();
    all_langs.dedup();
    for lang in all_langs {
        let t = tat_per.get(lang);
        let f = flr_per.get(lang);
        let tacc = t
            .map(|p| format!("{:.1}% ({})", p.accuracy(), p.total))
            .unwrap_or_else(|| "—".to_string());
        let facc = f
            .map(|p| format!("{:.1}% ({})", p.accuracy(), p.total))
            .unwrap_or_else(|| "—".to_string());
        // Merge miss counts from both fixtures for the "common miss" column.
        let mut merged: BTreeMap<String, u32> = BTreeMap::new();
        for p in [t, f].into_iter().flatten() {
            for (k, v) in &p.confusion {
                *merged.entry(k.clone()).or_insert(0) += v;
            }
        }
        let miss = if merged.is_empty() {
            "—".to_string()
        } else {
            let mut v: Vec<(&String, &u32)> = merged.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1));
            let picks: Vec<String> = v
                .iter()
                .take(2)
                .map(|(l, n)| format!("{l} ({n})"))
                .collect();
            format!("→ {}", picks.join(", "))
        };
        println!("| {lang} | {tacc} | {facc} | {miss} |");
    }
}
