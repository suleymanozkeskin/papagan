//! Character-trigram scoring. See DESIGN.md §3 and §7.

use smallvec::SmallVec;

use crate::lang::Lang;
use crate::ngram::data;
use crate::score::softmax_in_place;

// Must match xtask/src/main.rs::extract_trigrams byte-for-byte — the runtime
// model was trained on these exact strings.
pub(crate) fn extract_trigrams(word: &str) -> SmallVec<[String; 8]> {
    let mut padded = String::with_capacity(word.len() + 4);
    padded.push_str("^^");
    padded.push_str(word);
    padded.push_str("$$");
    let chars: Vec<char> = padded.chars().collect();
    let mut out = SmallVec::new();
    for w in chars.windows(3) {
        let mut s = String::with_capacity(12);
        s.extend(w);
        out.push(s);
    }
    out
}

pub(crate) fn score_word(word: &str, enabled: &[Lang]) -> SmallVec<[(Lang, f32); 8]> {
    let trigrams = extract_trigrams(word);
    if trigrams.is_empty() {
        return SmallVec::new();
    }

    let mut raw: SmallVec<[(Lang, f32); 8]> = enabled
        .iter()
        .map(|&l| (l, sum_logprobs(&trigrams, l)))
        .collect();

    softmax_in_place(&mut raw);
    raw
}

fn sum_logprobs(trigrams: &[String], lang: Lang) -> f32 {
    trigrams
        .iter()
        .map(|t| data::logprob(lang, t.as_str()))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_boundary_padded_trigrams() {
        let tris = extract_trigrams("hi");
        let joined: Vec<&str> = tris.iter().map(String::as_str).collect();
        assert_eq!(joined, vec!["^^h", "^hi", "hi$", "i$$"]);
    }

    #[test]
    fn single_char_word_yields_three_trigrams() {
        let tris = extract_trigrams("a");
        assert_eq!(tris.len(), 3);
        assert_eq!(tris[0], "^^a");
        assert_eq!(tris[1], "^a$");
        assert_eq!(tris[2], "a$$");
    }

    #[test]
    fn empty_word_yields_one_boundary_trigram() {
        // "^^" + "" + "$$" = "^^$$" → 2 trigrams "^^$" and "^$$"
        let tris = extract_trigrams("");
        assert_eq!(tris.len(), 2);
    }
}
