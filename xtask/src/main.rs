//! Training pipeline for lang-detect. See ../DESIGN.md §12.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use unicode_normalization::UnicodeNormalization;

#[derive(Parser)]
#[command(name = "xtask", about = "lang-detect offline tooling")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Fetch frequency lists and emit data/<lang>/{words.txt,trigrams.txt}.
    BuildData {
        /// Comma-separated ISO 639-1 language codes, e.g. "en,de,tr".
        #[arg(long, value_delimiter = ',', required = true)]
        langs: Vec<String>,

        /// Top-N words from the raw frequency list used for trigram training.
        #[arg(long, default_value_t = 10_000)]
        top_n: usize,

        /// Words written to words.txt. Keep headroom here — the runtime
        /// crate selects a subset via its `dict-5k` / `dict-10k` features
        /// or the `LANG_DETECT_DICT_SIZE` env var.
        #[arg(long, default_value_t = 10_000)]
        dict_size: usize,

        /// Top-K trigrams kept per language (trigrams.txt).
        #[arg(long, default_value_t = 3_000)]
        trigram_k: usize,

        /// Add-α Laplace smoothing constant.
        #[arg(long, default_value_t = 0.5)]
        smoothing_alpha: f64,

        /// Output directory (relative to workspace root).
        #[arg(long, default_value = "data")]
        out: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::BuildData {
            langs,
            top_n,
            dict_size,
            trigram_k,
            smoothing_alpha,
            out,
        } => {
            for lang in langs {
                build_lang(&lang, top_n, dict_size, trigram_k, smoothing_alpha, &out)
                    .with_context(|| format!("building data for {lang}"))?;
            }
            Ok(())
        }
    }
}

fn build_lang(
    lang: &str,
    top_n: usize,
    dict_size: usize,
    trigram_k: usize,
    alpha: f64,
    out_dir: &str,
) -> Result<()> {
    let url = format!(
        "https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/{lang}/{lang}_50k.txt"
    );
    println!("[{lang}] fetching {url}");
    let body = ureq::get(&url)
        .call()
        .with_context(|| format!("HTTP GET {url}"))?
        .into_string()
        .context("reading body as UTF-8")?;

    // Parse + normalize into (word, freq) pairs, top_n unique.
    let mut seen = HashSet::new();
    let mut entries: Vec<(String, u64)> = Vec::with_capacity(top_n);
    for line in body.lines() {
        let mut parts = line.split_whitespace();
        let Some(raw_word) = parts.next() else { continue };
        let Some(freq_s) = parts.next() else { continue };
        let Ok(freq) = freq_s.parse::<u64>() else { continue };
        if !raw_word.chars().any(|c| c.is_alphabetic()) {
            continue;
        }
        let normalized: String = raw_word.nfkc().flat_map(char::to_lowercase).collect();
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        entries.push((normalized, freq));
        if entries.len() >= top_n {
            break;
        }
    }

    let lang_dir = Path::new(out_dir).join(lang);
    fs::create_dir_all(&lang_dir)
        .with_context(|| format!("creating {}", lang_dir.display()))?;

    write_words(lang, &entries, dict_size, &lang_dir)?;
    write_trigrams(lang, &entries, trigram_k, alpha, &lang_dir)?;

    Ok(())
}

fn write_words(lang: &str, entries: &[(String, u64)], dict_size: usize, dir: &Path) -> Result<()> {
    let words: Vec<&str> = entries
        .iter()
        .take(dict_size)
        .map(|(w, _)| w.as_str())
        .collect();
    let path = dir.join("words.txt");
    let mut content = words.join("\n");
    content.push('\n');
    fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
    println!("[{lang}] wrote {} ({} words)", path.display(), words.len());
    Ok(())
}

fn write_trigrams(
    lang: &str,
    entries: &[(String, u64)],
    k: usize,
    alpha: f64,
    dir: &Path,
) -> Result<()> {
    // Accumulate trigram counts weighted by word frequency.
    let mut counts: HashMap<String, u64> = HashMap::new();
    for (word, freq) in entries {
        for tri in extract_trigrams(word) {
            *counts.entry(tri).or_insert(0) += *freq;
        }
    }

    let total: u64 = counts.values().sum();
    let vocab = counts.len() as f64;
    let denom = total as f64 + alpha * vocab;

    // Keep top-K by count, then sort alphabetically for stable diffs.
    let mut scored: Vec<(String, f32)> = counts
        .into_iter()
        .map(|(t, c)| {
            let p = (c as f64 + alpha) / denom;
            (t, p.ln() as f32)
        })
        .collect();
    // Higher logprob = higher count under the same smoothing denom.
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    scored.sort_by(|a, b| a.0.cmp(&b.0));

    let floor = (alpha / denom).ln() as f32;

    let mut content = String::new();
    // Sentinel for the floor value. "FLOOR" cannot appear as a real trigram
    // because we lowercase input before extraction.
    content.push_str(&format!("FLOOR\t{floor}\n"));
    for (tri, logp) in &scored {
        content.push_str(&format!("{tri}\t{logp}\n"));
    }

    let path = dir.join("trigrams.txt");
    fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
    println!("[{lang}] wrote {} ({} trigrams)", path.display(), scored.len());
    Ok(())
}

fn extract_trigrams(word: &str) -> Vec<String> {
    // Must match lang-detect's runtime extraction byte-for-byte.
    let mut padded = String::with_capacity(word.len() + 4);
    padded.push_str("^^");
    padded.push_str(word);
    padded.push_str("$$");
    let chars: Vec<char> = padded.chars().collect();
    let mut out = Vec::with_capacity(chars.len().saturating_sub(2));
    for w in chars.windows(3) {
        let mut s = String::with_capacity(12);
        s.extend(w);
        out.push(s);
    }
    out
}
