use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

// Tokenize + normalize per DESIGN.md §11:
//   NFKC, then Unicode default lowercase. Preserves Turkish İ/ı/I/i distinctions.
pub(crate) fn tokenize(input: &str) -> Vec<String> {
    input
        .unicode_words()
        .map(normalize)
        .filter(|s| !s.is_empty())
        .collect()
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
}
