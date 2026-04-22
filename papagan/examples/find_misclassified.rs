//! Print every sentence in a TSV accuracy fixture where the detector
//! disagrees with the label. Dumps the top prediction, full distribution,
//! and the first 30 per-word scores for each miss — useful when a single
//! miss shows up in a regression and you need to see why.
//!
//! Usage:
//!   cargo run --release --example find_misclassified --features all-langs \
//!       -- papagan/tests/fixtures/accuracy_flores.tsv [filter_lang]
//!
//! If `filter_lang` is provided (e.g. "nl"), only misses for that label are
//! printed.

use std::fs;

use papagan::{Detector, Lang, MatchSource};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "papagan/tests/fixtures/accuracy_flores.tsv".to_string());
    let filter = args.next();

    let content = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });

    let d = Detector::new();
    let mut total = 0usize;
    let mut misses = 0usize;

    for (line_no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((iso, text)) = line.split_once('\t') else {
            continue;
        };
        if let Some(ref f) = filter {
            if f != iso {
                continue;
            }
        }
        total += 1;

        let detailed = d.detect_detailed(text);
        let (top_lang, top_score) = detailed.aggregate.top();
        let pred = top_lang.iso_639_1();
        if pred == iso {
            continue;
        }
        misses += 1;

        println!("--- line {} — expected={} predicted={} ({:.3}) ---",
            line_no + 1, iso, pred, top_score);
        println!("TEXT: {text}");
        print!("DIST:");
        for (lang, score) in detailed.aggregate.distribution() {
            print!("  {}={:.3}", lang.iso_639_1(), score);
        }
        println!();
        println!("PER-WORD (first 30):");
        for w in detailed.words.iter().take(30) {
            let top = w
                .scores
                .iter()
                .fold((Lang::Unknown, 0.0_f32), |acc, (l, s)| {
                    if *s > acc.1 { (*l, *s) } else { acc }
                });
            let src = match w.source {
                MatchSource::Dict => "dict ",
                MatchSource::Ngram => "ngram",
                MatchSource::Unknown => "unk  ",
            };
            println!("  {src}  {:<20}  {} ({:.2})", w.token, top.0.iso_639_1(), top.1);
        }
        println!();
    }
    eprintln!(
        "\n{misses}/{total} misses in {path}{}",
        filter.map(|f| format!(" (filter: {f})")).unwrap_or_default()
    );
}
