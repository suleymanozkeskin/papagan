use smallvec::SmallVec;

use crate::lang::Lang;

pub struct Output {
    scores: SmallVec<[(Lang, f32); 8]>,
}

impl Output {
    pub(crate) fn unknown() -> Self {
        let mut scores = SmallVec::new();
        scores.push((Lang::Unknown, 1.0));
        Self { scores }
    }

    pub(crate) fn from_sorted(scores: SmallVec<[(Lang, f32); 8]>) -> Self {
        Self { scores }
    }

    pub fn top(&self) -> (Lang, f32) {
        self.scores.first().copied().unwrap_or((Lang::Unknown, 0.0))
    }

    pub fn distribution(&self) -> &[(Lang, f32)] {
        &self.scores
    }
}

pub struct Detailed {
    pub words: Vec<WordScore>,
    pub aggregate: Output,
}

impl Detailed {
    pub(crate) fn empty() -> Self {
        Self {
            words: Vec::new(),
            aggregate: Output::unknown(),
        }
    }
}

pub struct WordScore {
    pub token: Box<str>,
    pub scores: SmallVec<[(Lang, f32); 8]>,
    pub source: MatchSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchSource {
    Dict,
    Ngram,
    Unknown,
}

// Numerically-stable softmax. See DESIGN.md §7.
pub(crate) fn softmax_in_place(scores: &mut [(Lang, f32)]) {
    if scores.is_empty() {
        return;
    }
    let max = scores
        .iter()
        .map(|(_, s)| *s)
        .fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0_f32;
    for (_, s) in scores.iter_mut() {
        *s = (*s - max).exp();
        sum += *s;
    }
    if sum > 0.0 {
        for (_, s) in scores.iter_mut() {
            *s /= sum;
        }
    }
}

// Confidence-weighted document aggregation — each word votes with weight =
// max(its scores). Peaky distributions dominate; uniform distributions
// contribute little. See DESIGN.md §7.
pub(crate) fn aggregate(
    enabled: &[Lang],
    words: &[WordScore],
    unknown_threshold: f32,
) -> Output {
    if words.is_empty() || enabled.is_empty() {
        return Output::unknown();
    }

    let mut totals: SmallVec<[(Lang, f32); 8]> =
        enabled.iter().map(|&l| (l, 0.0_f32)).collect();
    let mut total_weight = 0.0_f32;

    for ws in words {
        if ws.source == MatchSource::Unknown || ws.scores.is_empty() {
            continue;
        }
        let confidence = ws.scores.iter().map(|(_, s)| *s).fold(0.0_f32, f32::max);
        total_weight += confidence;
        for (lang, score) in &ws.scores {
            if let Some(slot) = totals.iter_mut().find(|(l, _)| l == lang) {
                slot.1 += confidence * score;
            }
        }
    }

    if total_weight <= 0.0 {
        return Output::unknown();
    }

    for (_, s) in totals.iter_mut() {
        *s /= total_weight;
    }
    totals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((_, top)) = totals.first() {
        if *top < unknown_threshold {
            return Output::unknown();
        }
    }

    Output::from_sorted(totals)
}
