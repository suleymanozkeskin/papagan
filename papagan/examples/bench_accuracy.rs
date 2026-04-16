//! Accuracy harness. Runs the detector over `tests/fixtures/accuracy.tsv` and
//! emits a markdown report with per-language precision/recall/F1 plus a
//! confusion matrix.
//!
//! Run with all languages enabled:
//!   cargo run --release --example bench_accuracy --features all-langs
//!
//! Optional args:
//!   [fixtures-path]  default: papagan/tests/fixtures/accuracy.tsv
//!   [output-path]    default: stdout (prepend - to also print when writing)

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

use papagan::Detector;

fn main() {
    let mut args = std::env::args().skip(1);
    let fixtures_path = args
        .next()
        .unwrap_or_else(|| "papagan/tests/fixtures/accuracy.tsv".to_string());
    let output_path = args.next();

    let content = fs::read_to_string(&fixtures_path).unwrap_or_else(|e| {
        eprintln!("failed to read {fixtures_path}: {e}");
        std::process::exit(1);
    });

    let mut examples: Vec<(String, String)> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((iso, text)) = line.split_once('\t') else {
            continue;
        };
        examples.push((iso.to_string(), text.to_string()));
    }

    let detector = Detector::new();

    let mut tp: BTreeMap<String, u32> = BTreeMap::new();
    let mut fp: BTreeMap<String, u32> = BTreeMap::new();
    let mut fn_: BTreeMap<String, u32> = BTreeMap::new();
    let mut support: BTreeMap<String, u32> = BTreeMap::new();
    let mut confusion: BTreeMap<(String, String), u32> = BTreeMap::new();
    let mut correct = 0usize;

    for (expected, text) in &examples {
        let predicted = detector.detect(text).top().0.iso_639_1().to_string();
        *support.entry(expected.clone()).or_insert(0) += 1;
        *confusion
            .entry((expected.clone(), predicted.clone()))
            .or_insert(0) += 1;
        if predicted == *expected {
            correct += 1;
            *tp.entry(predicted).or_insert(0) += 1;
        } else {
            *fp.entry(predicted).or_insert(0) += 1;
            *fn_.entry(expected.clone()).or_insert(0) += 1;
        }
    }

    let report = build_report(
        examples.len(),
        correct,
        &support,
        &tp,
        &fp,
        &fn_,
        &confusion,
    );

    match output_path {
        Some(path) => {
            fs::write(&path, &report).expect("write report");
            eprintln!("wrote {path}");
        }
        None => print!("{report}"),
    }
}

fn build_report(
    total: usize,
    correct: usize,
    support: &BTreeMap<String, u32>,
    tp: &BTreeMap<String, u32>,
    fp: &BTreeMap<String, u32>,
    fn_: &BTreeMap<String, u32>,
    confusion: &BTreeMap<(String, String), u32>,
) -> String {
    let mut r = String::new();
    writeln!(r, "# papagan accuracy baseline").unwrap();
    writeln!(r).unwrap();
    writeln!(
        r,
        "- Sentences: **{total}**  |  Correct: **{correct}**  |  Overall accuracy: **{:.1}%**",
        100.0 * correct as f32 / total as f32
    )
    .unwrap();
    writeln!(r, "- Compiled with whatever features were active at build time. Set `PAPAGAN_DICT_SIZE` env var or enable `dict-5k`/`dict-10k` features to change dictionary tier.").unwrap();
    writeln!(r).unwrap();

    writeln!(r, "## Per-language metrics").unwrap();
    writeln!(r).unwrap();
    writeln!(r, "| Lang | Support | Precision | Recall | F1 |").unwrap();
    writeln!(r, "|------|--------:|----------:|-------:|---:|").unwrap();
    for (lang, n) in support {
        let tp_l = *tp.get(lang).unwrap_or(&0) as f32;
        let fp_l = *fp.get(lang).unwrap_or(&0) as f32;
        let fn_l = *fn_.get(lang).unwrap_or(&0) as f32;
        let precision = safe_div(tp_l, tp_l + fp_l);
        let recall = safe_div(tp_l, tp_l + fn_l);
        let f1 = safe_div(2.0 * precision * recall, precision + recall);
        writeln!(r, "| {lang} | {n} | {precision:.3} | {recall:.3} | {f1:.3} |").unwrap();
    }
    writeln!(r).unwrap();

    // Confusion matrix — actual (rows) × predicted (cols).
    let langs: Vec<String> = support.keys().cloned().collect();
    let mut all_predicted: std::collections::BTreeSet<String> =
        langs.iter().cloned().collect();
    for (_actual, predicted) in confusion.keys() {
        all_predicted.insert(predicted.clone());
    }
    let cols: Vec<String> = all_predicted.into_iter().collect();

    writeln!(r, "## Confusion matrix").unwrap();
    writeln!(r).unwrap();
    write!(r, "| actual \\ predicted |").unwrap();
    for c in &cols {
        write!(r, " {c} |").unwrap();
    }
    writeln!(r).unwrap();
    write!(r, "|---|").unwrap();
    for _ in &cols {
        write!(r, "---:|").unwrap();
    }
    writeln!(r).unwrap();
    for actual in &langs {
        write!(r, "| **{actual}** |").unwrap();
        for predicted in &cols {
            let n = confusion
                .get(&(actual.clone(), predicted.clone()))
                .copied()
                .unwrap_or(0);
            if n == 0 {
                write!(r, " . |").unwrap();
            } else if actual == predicted {
                write!(r, " **{n}** |").unwrap();
            } else {
                write!(r, " {n} |").unwrap();
            }
        }
        writeln!(r).unwrap();
    }
    writeln!(r).unwrap();

    r
}

fn safe_div(a: f32, b: f32) -> f32 {
    if b > 0.0 {
        a / b
    } else {
        0.0
    }
}
