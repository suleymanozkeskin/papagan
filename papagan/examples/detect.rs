//! Minimal CLI demo — prints the language distribution for a string.
//!
//! Usage:
//!   cargo run --example detect --release -- "Die Katze sitzt auf der Matte"

use papagan::Detector;

fn main() {
    let input: String = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    if input.trim().is_empty() {
        eprintln!("usage: detect <text>");
        std::process::exit(1);
    }

    let d = Detector::new();
    let detailed = d.detect_detailed(&input);
    let (top_lang, top_score) = detailed.aggregate.top();
    println!("top: {:?} ({:.3})", top_lang, top_score);

    println!("distribution:");
    for (lang, score) in detailed.aggregate.distribution() {
        println!("  {:>4}: {:.3}", lang.iso_639_1(), score);
    }

    println!("per-word:");
    for w in &detailed.words {
        let top = w
            .scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(l, s)| format!("{:?} {:.2}", l, s))
            .unwrap_or_else(|| "—".into());
        println!("  {:<20} [{:?}] top={}", w.token.as_ref(), w.source, top);
    }
}
