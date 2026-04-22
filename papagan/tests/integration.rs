use papagan::{Detector, Lang};

#[cfg(any(feature = "de", feature = "tr", feature = "ru"))]
use papagan::MatchSource;

#[cfg(feature = "en")]
#[test]
fn detects_english() {
    let d = Detector::new();
    let out = d.detect("the quick brown fox jumps over the lazy dog");
    assert_eq!(out.top().0, Lang::En);
}

#[cfg(all(feature = "en", feature = "de"))]
#[test]
fn detects_german() {
    let d = Detector::new();
    let out = d.detect("der schnelle braune fuchs springt über den faulen hund");
    assert_eq!(out.top().0, Lang::De);
}

#[cfg(all(feature = "en", feature = "tr"))]
#[test]
fn detects_turkish() {
    let d = Detector::new();
    let out = d.detect("merhaba dünya bugün hava çok güzel");
    assert_eq!(out.top().0, Lang::Tr);
}

#[cfg(all(feature = "en", feature = "de"))]
#[test]
fn mixed_input_shows_distribution() {
    let d = Detector::new();
    let out = d.detect("The cat is black. Die Katze ist schwarz.");
    let dist = out.distribution();
    let de = dist
        .iter()
        .find(|(l, _)| *l == Lang::De)
        .map(|(_, s)| *s)
        .unwrap_or(0.0);
    let en = dist
        .iter()
        .find(|(l, _)| *l == Lang::En)
        .map(|(_, s)| *s)
        .unwrap_or(0.0);
    assert!(de > 0.1, "expected non-trivial German share, got {de}");
    assert!(en > 0.1, "expected non-trivial English share, got {en}");
}

#[cfg(feature = "en")]
#[test]
fn empty_input_is_unknown() {
    let d = Detector::new();
    assert_eq!(d.detect("").top().0, Lang::Unknown);
    assert_eq!(d.detect("   ").top().0, Lang::Unknown);
}

#[cfg(feature = "en")]
#[test]
fn builder_subsets_languages() {
    let d = Detector::builder().only([Lang::En]).build();
    let out = d.detect("the quick brown fox");
    assert_eq!(out.top().0, Lang::En);
}

#[cfg(all(feature = "en", feature = "de", feature = "tr"))]
#[test]
fn ngram_classifies_unknown_german_word() {
    // "morgendämmerung" (dawn) is almost certainly not in the top-1000.
    // Stage 1 misses; stage 2 should classify German via trigram signal.
    let d = Detector::new();
    let detailed = d.detect_detailed("morgendämmerung");
    let word = &detailed.words[0];
    assert_eq!(word.source, MatchSource::Ngram, "expected stage 2 to fire");
    assert_eq!(detailed.aggregate.top().0, Lang::De);
}

#[cfg(all(feature = "en", feature = "tr"))]
#[test]
fn ngram_classifies_unknown_turkish_word() {
    // Inflected Turkish form not in top-1000 list.
    let d = Detector::new();
    let detailed = d.detect_detailed("kitaplarınızı");
    let word = &detailed.words[0];
    assert_eq!(word.source, MatchSource::Ngram);
    assert_eq!(detailed.aggregate.top().0, Lang::Tr);
}

#[cfg(all(feature = "en", feature = "de", feature = "tr"))]
#[test]
fn detailed_reports_per_word_sources() {
    // "the" is in top-1000 (dict), "thermodynamics" is not (ngram).
    let d = Detector::new();
    let detailed = d.detect_detailed("the thermodynamics");
    assert_eq!(detailed.words.len(), 2);
    assert_eq!(detailed.words[0].source, MatchSource::Dict);
    assert_eq!(detailed.words[1].source, MatchSource::Ngram);
}

#[cfg(feature = "en")]
#[test]
fn parallel_path_large_input() {
    // 200 words — exceeds default threshold of 64, so parallel path activates
    // under `feature = "parallel"`. Verifies correctness at scale.
    let sentence = "the cat sat on the mat ";
    let long_input = sentence.repeat(40);
    let d = Detector::new();
    let out = d.detect(&long_input);
    assert_eq!(out.top().0, Lang::En);
}

#[cfg(feature = "en")]
#[test]
fn builder_parallel_threshold_is_respected() {
    // Force parallel path on small input by setting threshold=1.
    let d = Detector::builder().parallel_threshold(1).build();
    let out = d.detect("the quick brown fox");
    assert_eq!(out.top().0, Lang::En);
}

#[cfg(feature = "en")]
#[test]
fn detect_batch_matches_per_call() {
    // Batch results must match a per-call loop one-for-one. This test is
    // en-only on purpose so it runs under default features (where CI lives)
    // and has ≥ 4 items so it actually exercises the par_map_batch path
    // above BATCH_PARALLEL_THRESHOLD.
    let d = Detector::new();
    let inputs: Vec<&str> = vec![
        "the quick brown fox jumps over the lazy dog",
        "how are you doing today my friend",
        "language detection is a fun problem to solve and discuss",
        "she sells sea shells by the sea shore early in the morning",
        "a stitch in time saves nine on a busy day",
        "the rain in spain falls mainly on the plain every year",
    ];
    // 6 inputs, ~50+ approx-token total — trips both routing gates
    // (cardinality ≥ 2 and approx_tokens ≥ 50) so par_map_batch runs.
    let total_words: usize = inputs.iter().map(|s| s.split_whitespace().count()).sum();
    assert!(
        total_words >= 50,
        "test input must exceed approx-tokens gate; got {total_words}"
    );
    let serial: Vec<_> = inputs.iter().map(|s| d.detect(s).top().0).collect();
    let batched: Vec<_> = d
        .detect_batch(&inputs)
        .into_iter()
        .map(|o| o.top().0)
        .collect();
    assert_eq!(serial, batched);

    // Small-batch fallback — below the cardinality gate (N < 2), falls
    // through to the per-call path.
    let small = &inputs[..1];
    let serial: Vec<_> = small.iter().map(|s| d.detect(s).top().0).collect();
    let batched: Vec<_> = d
        .detect_batch(small)
        .into_iter()
        .map(|o| o.top().0)
        .collect();
    assert_eq!(serial, batched);

    // Low-work fallback — cardinality OK but total approx tokens < 50 so
    // we route through the per-call path. Results must still match.
    let tiny = ["a", "b", "c", "the", "a", "b"]; // ~6 tokens total
    let serial: Vec<_> = tiny.iter().map(|s| d.detect(s).top().0).collect();
    let batched: Vec<_> = d
        .detect_batch(&tiny)
        .into_iter()
        .map(|o| o.top().0)
        .collect();
    assert_eq!(serial, batched);

    // Empty input.
    let empty: Vec<&str> = Vec::new();
    assert!(d.detect_batch(&empty).is_empty());
}

#[cfg(feature = "en")]
#[test]
fn detect_detailed_batch_matches_per_call() {
    // Same coverage guarantee for the detailed variant — one result per
    // input, same top language as the per-call path.
    let d = Detector::new();
    let inputs: Vec<&str> = vec![
        "one two three four five",
        "the thermodynamics of irreversible processes",
        "hello world this is a test",
        "she sells sea shells",
        "a quick brown fox",
    ];
    let serial: Vec<_> = inputs.iter().map(|s| d.detect_detailed(s)).collect();
    let batched = d.detect_detailed_batch(&inputs);
    assert_eq!(serial.len(), batched.len());
    for (i, (s, b)) in serial.iter().zip(batched.iter()).enumerate() {
        assert_eq!(
            s.aggregate.top().0,
            b.aggregate.top().0,
            "mismatch at index {i}"
        );
        assert_eq!(s.words.len(), b.words.len(), "word count mismatch at {i}");
    }
}

#[cfg(feature = "en")]
#[test]
fn detect_batch_accepts_string_slices() {
    // AsRef<str> bound — Vec<String> should work without intermediate refs.
    let d = Detector::new();
    let owned: Vec<String> = vec!["the cat sat".into(), "on the mat".into()];
    let out = d.detect_batch(&owned);
    assert_eq!(out.len(), 2);
}

#[cfg(feature = "en")]
#[test]
fn detect_batch_respects_parallel_threshold_opt_out() {
    // Setting parallel_threshold to usize::MAX is the documented opt-out
    // for rayon — it must suppress batch-level parallelism too, otherwise
    // users who explicitly disable parallelism still get cross-document
    // rayon fan-out. Correctness check: results match a per-call loop.
    let d = Detector::builder().parallel_threshold(usize::MAX).build();
    let inputs: Vec<&str> = vec![
        "the quick brown fox",
        "jumps over the lazy dog",
        "hello world test",
        "one two three four",
        "five six seven eight",
    ];
    let serial: Vec<_> = inputs.iter().map(|s| d.detect(s).top().0).collect();
    let batched: Vec<_> = d
        .detect_batch(&inputs)
        .into_iter()
        .map(|o| o.top().0)
        .collect();
    assert_eq!(serial, batched);
}

#[cfg(all(feature = "en", feature = "ru"))]
#[test]
fn detects_russian() {
    let d = Detector::new();
    let out = d.detect("привет мир сегодня очень хорошая погода");
    assert_eq!(out.top().0, Lang::Ru);
}

#[cfg(all(feature = "en", feature = "fr"))]
#[test]
fn detects_french() {
    let d = Detector::new();
    let out = d.detect("bonjour le monde aujourd'hui il fait très beau");
    assert_eq!(out.top().0, Lang::Fr);
}

#[cfg(all(feature = "en", feature = "es"))]
#[test]
fn detects_spanish() {
    let d = Detector::new();
    let out = d.detect("hola mundo hoy hace muy buen tiempo");
    assert_eq!(out.top().0, Lang::Es);
}

#[cfg(all(feature = "en", feature = "it", feature = "es"))]
#[test]
fn italian_vs_spanish_discriminable() {
    // Italian and Spanish share many cognates — but the signal should still lean right.
    let d = Detector::new();
    let it_out = d.detect("il gatto è sul tappeto e beve il latte");
    assert_eq!(it_out.top().0, Lang::It);

    let es_out = d.detect("el gato está en la alfombra y bebe la leche");
    assert_eq!(es_out.top().0, Lang::Es);
}

#[cfg(all(feature = "en", feature = "de", feature = "fr"))]
#[test]
fn trilingual_input_returns_distribution_not_unknown() {
    // Balanced en/de/fr — each peaks around ~0.33. Must NOT collapse to Unknown.
    let d = Detector::new();
    let out = d.detect(
        "Die Katze sitzt auf der Matte. The cat sits on the mat. Le chat est sur le tapis.",
    );
    let top = out.top().0;
    assert!(
        matches!(top, Lang::De | Lang::En | Lang::Fr),
        "expected one of de/en/fr as top, got {top:?}"
    );
    let dist = out.distribution();
    let de = dist
        .iter()
        .find(|(l, _)| *l == Lang::De)
        .map(|(_, s)| *s)
        .unwrap_or(0.0);
    let en = dist
        .iter()
        .find(|(l, _)| *l == Lang::En)
        .map(|(_, s)| *s)
        .unwrap_or(0.0);
    let fr = dist
        .iter()
        .find(|(l, _)| *l == Lang::Fr)
        .map(|(_, s)| *s)
        .unwrap_or(0.0);
    assert!(
        de > 0.1 && en > 0.1 && fr > 0.1,
        "expected non-trivial share for all three: de={de} en={en} fr={fr}"
    );
}

#[cfg(all(feature = "en", feature = "ru"))]
#[test]
fn cyrillic_script_routes_to_russian_via_ngrams() {
    // A word unlikely to be in top-1000 Russian. Script alone should drive the
    // trigram model there since Cyrillic trigrams have floor probability under
    // Latin-trained models.
    let d = Detector::new();
    let detailed = d.detect_detailed("библиотекарша");
    assert_eq!(detailed.words[0].source, MatchSource::Ngram);
    assert_eq!(detailed.aggregate.top().0, Lang::Ru);
}
