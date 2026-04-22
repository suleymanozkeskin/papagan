use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

// Tokenize + normalize per DESIGN.md §11:
//   NFKC, then Unicode default lowercase. Preserves Turkish İ/ı/I/i distinctions.
//
// Numeric-only tokens ("802", "2", "1920s-style" minus the letters) are
// dropped — they carry no language signal but their short trigram footprint
// produces high-confidence ngram noise that biases aggregation.
// The training pipeline applies the same filter (xtask/src/main.rs),
// so runtime behavior matches what the model was trained against.
pub(crate) fn tokenize(input: &str) -> Vec<String> {
    if input.is_ascii() {
        return tokenize_ascii(input);
    }
    input
        .unicode_words()
        .filter(|w| w.chars().any(|c| c.is_alphabetic()))
        .map(normalize)
        .filter(|s| !s.is_empty())
        .collect()
}

// Fast path: pure ASCII bypasses unicode_words + NFKC + Unicode lowercase.
// ASCII is NFKC-normal; alpha lowercase is `b | 0x20`. Word = run of
// alphanumeric or apostrophe, matching UAX #29 behavior for the ASCII subset.
fn tokenize_ascii(input: &str) -> Vec<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if is_ascii_word_byte(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_ascii_word_byte(bytes[i]) {
                i += 1;
            }
            if let Some(word) = ascii_normalize(&bytes[start..i]) {
                out.push(word);
            }
        } else {
            i += 1;
        }
    }
    out
}

#[inline]
fn is_ascii_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'\''
}

fn ascii_normalize(run: &[u8]) -> Option<String> {
    // Strip outer apostrophes — UAX #29 treats apostrophe as part of a word
    // only when flanked by letters, so a leading/trailing `'` isn't part of
    // the emitted token.
    let mut start = 0;
    let mut end = run.len();
    while start < end && run[start] == b'\'' {
        start += 1;
    }
    while end > start && run[end - 1] == b'\'' {
        end -= 1;
    }
    if start == end {
        return None;
    }
    let slice = &run[start..end];
    // Drop tokens that contain no ASCII letters — see tokenize() note. Keeps
    // "2pm" and "i18n" (letters present) but filters "802" / "1920".
    if !slice.iter().any(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let mut s = String::with_capacity(slice.len());
    for &b in slice {
        let lower = if b.is_ascii_uppercase() { b | 0x20 } else { b };
        s.push(lower as char);
    }
    Some(s)
}

pub(crate) fn normalize(word: &str) -> String {
    word.nfkc().flat_map(char::to_lowercase).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_ascii_words() {
        assert_eq!(tokenize("The cat sat."), vec!["the", "cat", "sat"]);
    }

    #[test]
    fn preserves_turkish_dotted_i() {
        // "İ" lowercases to "i\u{0307}" (i + combining dot above) under Unicode default.
        // "I" lowercases to plain "i". These must stay distinct.
        let dotted = normalize("İ");
        let plain = normalize("I");
        assert_ne!(dotted, plain);
        assert_eq!(plain, "i");
    }

    #[test]
    fn handles_mixed_script() {
        let out = tokenize("Hello привет");
        assert_eq!(out, vec!["hello", "привет"]);
    }

    fn tokenize_unicode_only(input: &str) -> Vec<String> {
        input
            .unicode_words()
            .filter(|w| w.chars().any(|c| c.is_alphabetic()))
            .map(normalize)
            .filter(|s| !s.is_empty())
            .collect()
    }

    #[test]
    fn ascii_fast_path_matches_unicode_path() {
        let cases = [
            "hello world",
            "Don't stop believing",
            "it's a test",
            "foo-bar baz",
            "mixed 123abc test",
            "Hello, World!",
            "''empty quotes''",
            "trailing.",
            "UPPER case",
            "tab\there",
            "newline\nthere",
            "a,b,c",
            "What is your age?",
            "Are you authorized to work in Germany?",
        ];
        for c in cases {
            assert_eq!(
                tokenize(c),
                tokenize_unicode_only(c),
                "fast vs unicode diverged on: {c:?}"
            );
        }
    }
}
