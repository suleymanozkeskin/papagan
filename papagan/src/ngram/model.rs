//! Character-trigram scoring. See DESIGN.md §3 and §7.

use smallvec::SmallVec;

use crate::lang::Lang;
use crate::ngram::data;
use crate::score::softmax_in_place;

// Build ^^word$$ once, record char-start byte offsets, then slice the buffer
// per trigram. Returns (padded, offsets) so the caller can yield &strs by
// slicing `padded[offsets[i]..offsets[i+3]]`.
//
// Must match xtask/src/main.rs::extract_trigrams byte-for-byte — the runtime
// model was trained on these exact strings.
fn padded_with_offsets(word: &str) -> (String, SmallVec<[usize; 16]>) {
    let mut padded = String::with_capacity(word.len() + 4);
    padded.push_str("^^");
    padded.push_str(word);
    padded.push_str("$$");
    let mut offsets: SmallVec<[usize; 16]> = padded.char_indices().map(|(i, _)| i).collect();
    offsets.push(padded.len());
    (padded, offsets)
}

#[cfg(test)]
pub(crate) fn extract_trigrams(word: &str) -> SmallVec<[String; 8]> {
    let (padded, offsets) = padded_with_offsets(word);
    if offsets.len() < 4 {
        return SmallVec::new();
    }
    (0..offsets.len() - 3)
        .map(|i| padded[offsets[i]..offsets[i + 3]].to_string())
        .collect()
}

pub(crate) fn score_word(word: &str, enabled: &[Lang]) -> SmallVec<[(Lang, f32); 8]> {
    let (padded, offsets) = padded_with_offsets(word);
    if offsets.len() < 4 {
        return SmallVec::new();
    }
    let n = offsets.len() - 3;

    let mut raw: SmallVec<[(Lang, f32); 8]> = enabled
        .iter()
        .map(|&l| {
            let mut sum = 0.0_f32;
            for i in 0..n {
                let tri = &padded[offsets[i]..offsets[i + 3]];
                sum += data::logprob(l, tri);
            }
            (l, sum)
        })
        .collect();

    softmax_in_place(&mut raw);
    raw
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
