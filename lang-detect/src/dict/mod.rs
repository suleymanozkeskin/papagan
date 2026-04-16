//! Stage 1 — dictionary fast path. See DESIGN.md §3.
//!
//! Pipeline tokenizes before lookup, so we use a sorted-by-word table with
//! binary search rather than aho-corasick (which shines at raw-text multi-pattern
//! scanning, not exact-word lookup after tokenization).

mod data;

use smallvec::SmallVec;

use self::data::DICT_ENTRIES;
use crate::lang::Lang;

// Dampens top-rank dominance in the prior — `die` at rank 0 shouldn't have
// infinite weight versus rank 1. See DESIGN.md §7.
pub(crate) const PRIOR_DAMPENER: f32 = 10.0;

pub(crate) fn lookup(word: &str) -> SmallVec<[(Lang, u16); 4]> {
    let start = DICT_ENTRIES.partition_point(|(w, _, _)| *w < word);
    let mut hits = SmallVec::new();
    for (w, lang, rank) in &DICT_ENTRIES[start..] {
        if *w != word {
            break;
        }
        hits.push((*lang, *rank));
    }
    hits
}

pub(crate) fn rank_weighted_priors(hits: &[(Lang, u16)]) -> SmallVec<[(Lang, f32); 8]> {
    let mut scores: SmallVec<[(Lang, f32); 8]> = hits
        .iter()
        .map(|(lang, rank)| (*lang, 1.0 / (*rank as f32 + PRIOR_DAMPENER)))
        .collect();
    let sum: f32 = scores.iter().map(|(_, w)| *w).sum();
    if sum > 0.0 {
        for (_, w) in scores.iter_mut() {
            *w /= sum;
        }
    }
    scores
}
