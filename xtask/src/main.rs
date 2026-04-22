//! Training pipeline for papagan. See ../DESIGN.md §12.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use unicode_normalization::UnicodeNormalization;

#[derive(Parser)]
#[command(name = "xtask", about = "papagan offline tooling")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Fetch Tatoeba sentences and build an accuracy evaluation fixture.
    FetchEval {
        /// Comma-separated ISO 639-1 codes.
        #[arg(long, value_delimiter = ',', required = true)]
        langs: Vec<String>,

        /// Sentences to keep per language (reservoir-sampled).
        #[arg(long, default_value_t = 500)]
        samples: usize,

        /// Output TSV path (will be overwritten).
        #[arg(long, default_value = "papagan/tests/fixtures/accuracy_large.tsv")]
        out: String,

        /// Minimum character count to keep a sentence.
        #[arg(long, default_value_t = 20)]
        min_chars: usize,

        /// Maximum character count to keep a sentence.
        #[arg(long, default_value_t = 200)]
        max_chars: usize,

        /// Deterministic seed for reservoir sampling.
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },

    /// Fetch Leipzig Corpora Collection news sentences, glue them into
    /// paragraphs, and emit a JSON array suitable for the long-form speed
    /// bench. Complements `bench/titles.json` (short titles) with
    /// paragraph-length inputs that trip the parallel code path.
    FetchLeipzig {
        /// Comma-separated ISO 639-1 codes.
        #[arg(
            long,
            value_delimiter = ',',
            default_value = "en,de,tr,ru,fr,es,it,nl,pt,pl"
        )]
        langs: Vec<String>,

        /// Number of consecutive sentences to concatenate into each paragraph.
        #[arg(long, default_value_t = 5)]
        paragraph_size: usize,

        /// Paragraphs to keep per language after sampling.
        #[arg(long, default_value_t = 100)]
        samples: usize,

        /// Leipzig corpus size suffix — one of 10K, 30K, 100K, 300K, 1M.
        /// Smaller = faster download; plenty of headroom even at 10K for
        /// the default sample count.
        #[arg(long, default_value = "10K")]
        size: String,

        /// Output JSON path (array of strings, shape matches
        /// `bench/titles.json`).
        #[arg(long, default_value = "bench/paragraphs.json")]
        out: String,

        /// Deterministic seed for sentence sampling.
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },

    /// Fetch FLORES-200 sentences and build a second accuracy evaluation
    /// fixture — complements the Tatoeba fixture with more formal,
    /// Wikipedia/news-sourced prose (catches OOD regressions the subtitle-
    /// trained models miss).
    FetchFlores {
        /// Comma-separated ISO 639-1 codes.
        #[arg(
            long,
            value_delimiter = ',',
            default_value = "en,de,tr,ru,fr,es,it,nl,pt,pl"
        )]
        langs: Vec<String>,

        /// FLORES-200 split to pull from: `dev` (997 sentences/lang) or
        /// `devtest` (1012/lang). Devtest is the standard held-out eval split.
        #[arg(long, default_value = "devtest")]
        split: String,

        /// Output TSV path (will be overwritten).
        #[arg(long, default_value = "papagan/tests/fixtures/accuracy_flores.tsv")]
        out: String,

        /// Path to a pre-downloaded `flores200_dataset.tar.gz`. If set, skips
        /// the network download (useful for re-running offline).
        #[arg(long)]
        tarball: Option<String>,
    },

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
        /// or the `PAPAGAN_DICT_SIZE` env var.
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
        Cmd::FetchEval {
            langs,
            samples,
            out,
            min_chars,
            max_chars,
            seed,
        } => fetch_eval(&langs, samples, &out, min_chars, max_chars, seed),
        Cmd::FetchFlores {
            langs,
            split,
            out,
            tarball,
        } => fetch_flores(&langs, &split, &out, tarball.as_deref()),
        Cmd::FetchLeipzig {
            langs,
            paragraph_size,
            samples,
            size,
            out,
            seed,
        } => fetch_leipzig(&langs, paragraph_size, samples, &size, &out, seed),
    }
}

fn iso2_to_iso3(iso2: &str) -> Option<&'static str> {
    match iso2 {
        "en" => Some("eng"),
        "de" => Some("deu"),
        "tr" => Some("tur"),
        "ru" => Some("rus"),
        "fr" => Some("fra"),
        "es" => Some("spa"),
        "it" => Some("ita"),
        "nl" => Some("nld"),
        "pt" => Some("por"),
        "pl" => Some("pol"),
        _ => None,
    }
}

// Leipzig archives are named "<iso3>_news_<year>_<size>". The year varies
// per language because Leipzig publishes new packs on different cadences;
// these are the most recent available as of 2026-04 for each supported lang.
fn iso2_to_leipzig_corpus(iso2: &str, size: &str) -> Option<String> {
    let (iso3, year) = match iso2 {
        "en" => ("eng", "2024"),
        "de" => ("deu", "2024"),
        "tr" => ("tur", "2023"),
        "ru" => ("rus", "2023"),
        "fr" => ("fra", "2023"),
        "es" => ("spa", "2023"),
        "it" => ("ita", "2023"),
        "nl" => ("nld", "2023"),
        "pt" => ("por", "2023"),
        "pl" => ("pol", "2023"),
        _ => return None,
    };
    Some(format!("{iso3}_news_{year}_{size}"))
}

// FLORES-200 uses BCP-47-style codes with an explicit script subtag. We pin
// the specific script variant each of our supported languages ships in.
fn iso2_to_flores(iso2: &str) -> Option<&'static str> {
    match iso2 {
        "en" => Some("eng_Latn"),
        "de" => Some("deu_Latn"),
        "tr" => Some("tur_Latn"),
        "ru" => Some("rus_Cyrl"),
        "fr" => Some("fra_Latn"),
        "es" => Some("spa_Latn"),
        "it" => Some("ita_Latn"),
        "nl" => Some("nld_Latn"),
        "pt" => Some("por_Latn"),
        "pl" => Some("pol_Latn"),
        _ => None,
    }
}

fn fetch_eval(
    langs: &[String],
    samples: usize,
    out: &str,
    min_chars: usize,
    max_chars: usize,
    seed: u64,
) -> Result<()> {
    let mut content = String::new();
    writeln!(
        content,
        "# Accuracy-benchmark fixtures (large) — generated by `cargo xtask fetch-eval`."
    )?;
    writeln!(
        content,
        "# Source: Tatoeba.org — https://downloads.tatoeba.org/"
    )?;
    writeln!(
        content,
        "# License: CC-BY 2.0 FR (https://creativecommons.org/licenses/by/2.0/fr/)"
    )?;
    writeln!(
        content,
        "# Params: seed={seed}, samples={samples}, min_chars={min_chars}, max_chars={max_chars}"
    )?;
    writeln!(content)?;

    let mut total = 0usize;
    for iso2 in langs {
        let iso3 =
            iso2_to_iso3(iso2).ok_or_else(|| anyhow::anyhow!("unknown language code: {iso2}"))?;
        let url = format!(
            "https://downloads.tatoeba.org/exports/per_language/{iso3}/{iso3}_sentences.tsv.bz2"
        );
        println!("[{iso2}] fetching {url}");
        let sentences = fetch_and_sample(&url, iso3, samples, min_chars, max_chars, seed)
            .with_context(|| format!("fetching {iso2}"))?;
        println!("[{iso2}] sampled {} sentences", sentences.len());
        for sent in &sentences {
            let cleaned = sent.replace(['\t', '\n', '\r'], " ");
            writeln!(content, "{iso2}\t{cleaned}")?;
        }
        total += sentences.len();
    }

    fs::write(out, content).with_context(|| format!("writing {out}"))?;
    println!("wrote {out} ({total} total sentences)");
    Ok(())
}

fn fetch_and_sample(
    url: &str,
    iso3: &str,
    n: usize,
    min_chars: usize,
    max_chars: usize,
    seed: u64,
) -> Result<Vec<String>> {
    let resp = ureq::get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let reader = resp.into_reader();
    let decoder = bzip2::read::BzDecoder::new(reader);
    let buffered = BufReader::new(decoder);

    let mut rng = seed.wrapping_add(1);
    let mut reservoir: Vec<String> = Vec::with_capacity(n);
    let mut matches_seen = 0usize;

    for line in buffered.lines() {
        let Ok(line) = line else { continue };
        let mut parts = line.splitn(3, '\t');
        let Some(_id) = parts.next() else { continue };
        let Some(lang) = parts.next() else { continue };
        if lang != iso3 {
            continue;
        }
        let Some(text) = parts.next() else { continue };
        let text = text.trim();
        let char_count = text.chars().count();
        if char_count < min_chars || char_count > max_chars {
            continue;
        }

        if reservoir.len() < n {
            reservoir.push(text.to_string());
        } else {
            let r = (lcg_next(&mut rng) % (matches_seen as u64 + 1)) as usize;
            if r < n {
                reservoir[r] = text.to_string();
            }
        }
        matches_seen += 1;
    }
    Ok(reservoir)
}

fn fetch_flores(
    langs: &[String],
    split: &str,
    out: &str,
    tarball_path: Option<&str>,
) -> Result<()> {
    if split != "dev" && split != "devtest" {
        anyhow::bail!("--split must be 'dev' or 'devtest', got {split:?}");
    }

    // Build the set of archive paths we care about: `flores_code` → `iso2`.
    let mut want: HashMap<String, String> = HashMap::new();
    for iso2 in langs {
        let code = iso2_to_flores(iso2)
            .ok_or_else(|| anyhow::anyhow!("unsupported language code: {iso2}"))?;
        let path = format!("flores200_dataset/{split}/{code}.{split}");
        want.insert(path, iso2.clone());
    }

    // Open the archive stream — either from a cached local copy or streamed
    // straight from Meta's CDN.
    let url = "https://dl.fbaipublicfiles.com/nllb/flores200_dataset.tar.gz";
    let reader: Box<dyn Read> = if let Some(p) = tarball_path {
        println!("reading cached tarball from {p}");
        Box::new(fs::File::open(p).with_context(|| format!("opening {p}"))?)
    } else {
        println!("downloading {url} (~40 MB)");
        Box::new(
            ureq::get(url)
                .call()
                .with_context(|| format!("GET {url}"))?
                .into_reader(),
        )
    };
    let gz = flate2::read::GzDecoder::new(reader);
    let mut archive = tar::Archive::new(gz);

    let mut buckets: HashMap<String, Vec<String>> = HashMap::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let raw = entry.path()?.to_string_lossy().into_owned();
        // Tarball entries are prefixed with "./" — normalize before lookup.
        let path_str = raw.strip_prefix("./").unwrap_or(&raw).to_string();
        let Some(iso2) = want.get(&path_str) else {
            continue;
        };
        let mut contents = String::new();
        entry
            .read_to_string(&mut contents)
            .with_context(|| format!("reading {path_str}"))?;
        let sentences: Vec<String> = contents
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        println!("[{iso2}] {} sentences from {path_str}", sentences.len());
        buckets.insert(iso2.clone(), sentences);
    }

    for iso2 in langs {
        if !buckets.contains_key(iso2) {
            anyhow::bail!("no FLORES-200 entry found for {iso2} — archive layout may have changed");
        }
    }

    let mut content = String::new();
    writeln!(
        content,
        "# Accuracy-benchmark fixture (FLORES-200) — generated by `cargo xtask fetch-flores`."
    )?;
    writeln!(
        content,
        "# Source: FLORES-200 — https://github.com/facebookresearch/flores"
    )?;
    writeln!(
        content,
        "# License: CC-BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/)"
    )?;
    writeln!(content, "# Split: {split}")?;
    writeln!(content)?;

    let mut total = 0usize;
    for iso2 in langs {
        let sents = buckets.get(iso2).unwrap();
        for s in sents {
            let cleaned = s.replace(['\t', '\n', '\r'], " ");
            writeln!(content, "{iso2}\t{cleaned}")?;
        }
        total += sents.len();
    }

    fs::write(out, content).with_context(|| format!("writing {out}"))?;
    println!(
        "wrote {out} ({total} total sentences across {} langs)",
        langs.len()
    );
    Ok(())
}

fn fetch_leipzig(
    langs: &[String],
    paragraph_size: usize,
    samples: usize,
    size: &str,
    out: &str,
    seed: u64,
) -> Result<()> {
    if paragraph_size == 0 {
        anyhow::bail!("--paragraph-size must be >= 1");
    }

    let mut all_paragraphs: Vec<String> = Vec::new();
    for iso2 in langs {
        let corpus = iso2_to_leipzig_corpus(iso2, size)
            .ok_or_else(|| anyhow::anyhow!("unsupported language code: {iso2}"))?;
        let url = format!("https://downloads.wortschatz-leipzig.de/corpora/{corpus}.tar.gz");
        println!("[{iso2}] fetching {url}");

        let resp = ureq::get(&url)
            .call()
            .with_context(|| format!("GET {url}"))?;
        let gz = flate2::read::GzDecoder::new(resp.into_reader());
        let mut archive = tar::Archive::new(gz);

        let target = format!("{corpus}/{corpus}-sentences.txt");
        let mut sentences: Vec<String> = Vec::new();
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path_str = entry.path()?.to_string_lossy().into_owned();
            if path_str != target {
                continue;
            }
            let mut content = String::new();
            entry
                .read_to_string(&mut content)
                .with_context(|| format!("reading {path_str}"))?;
            // Leipzig format: "<id>\t<sentence>".
            for line in content.lines() {
                let Some((_, text)) = line.split_once('\t') else {
                    continue;
                };
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    sentences.push(trimmed.to_string());
                }
            }
            break;
        }
        if sentences.is_empty() {
            anyhow::bail!("no sentences found in {corpus} tarball");
        }

        // Group consecutive sentences into paragraphs.
        let mut paragraphs: Vec<String> = sentences
            .chunks(paragraph_size)
            .filter(|c| c.len() == paragraph_size)
            .map(|c| c.join(" "))
            .collect();

        // Deterministic partial Fisher-Yates to pick `samples` out of all
        // available paragraphs without bias toward the start of the file.
        let mut rng = seed
            .wrapping_add(1)
            .wrapping_mul(iso2.bytes().map(|b| b as u64).sum::<u64>().wrapping_add(1));
        let keep = paragraphs.len().min(samples);
        if paragraphs.len() > keep {
            for i in 0..keep {
                let remaining = (paragraphs.len() - i) as u64;
                let j = i + (lcg_next(&mut rng) % remaining) as usize;
                paragraphs.swap(i, j);
            }
            paragraphs.truncate(keep);
        }

        println!(
            "[{iso2}] kept {} paragraphs (avg {} chars)",
            paragraphs.len(),
            if paragraphs.is_empty() {
                0
            } else {
                paragraphs.iter().map(|p| p.len()).sum::<usize>() / paragraphs.len()
            }
        );
        all_paragraphs.extend(paragraphs);
    }

    // Shuffle across languages so the bench doesn't run contiguous per-lang
    // blocks (which would bias cache behavior).
    let mut rng = seed.wrapping_add(99);
    for i in 0..all_paragraphs.len() {
        let remaining = (all_paragraphs.len() - i) as u64;
        let j = i + (lcg_next(&mut rng) % remaining) as usize;
        all_paragraphs.swap(i, j);
    }

    let mut content = String::from("[");
    for (i, p) in all_paragraphs.iter().enumerate() {
        if i > 0 {
            content.push(',');
        }
        content.push_str("\n  ");
        content.push_str(&json_escape(p));
    }
    if !all_paragraphs.is_empty() {
        content.push('\n');
    }
    content.push_str("]\n");

    fs::write(out, content).with_context(|| format!("writing {out}"))?;
    println!("wrote {out} ({} paragraphs)", all_paragraphs.len());
    Ok(())
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// Simple LCG — deterministic, seeded, no extra dep.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
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
        let Some(raw_word) = parts.next() else {
            continue;
        };
        let Some(freq_s) = parts.next() else { continue };
        let Ok(freq) = freq_s.parse::<u64>() else {
            continue;
        };
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
    fs::create_dir_all(&lang_dir).with_context(|| format!("creating {}", lang_dir.display()))?;

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
    println!(
        "[{lang}] wrote {} ({} trigrams)",
        path.display(),
        scored.len()
    );
    Ok(())
}

fn extract_trigrams(word: &str) -> Vec<String> {
    // Must match papagan's runtime extraction byte-for-byte.
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
