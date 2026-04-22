use smallvec::SmallVec;

use crate::dict;
use crate::lang::Lang;
use crate::ngram;
use crate::parallel;
use crate::score::{self, Detailed, MatchSource, Output, WordScore};
use crate::tokenize;

// Calibrated for multilingual inputs: with three languages roughly equally
// mixed, the top peaks around ~0.33. Anything below 0.20 means we have less
// than one-in-five confidence — emit Unknown rather than a near-random guess.
const DEFAULT_UNKNOWN_THRESHOLD: f32 = 0.20;
// Threshold picked by sweeping {∞, 1024, 256, 128, 64, 32, 16, 0} on
// Leipzig paragraphs (median 84 words) and short titles (median 8 words).
// At 32: titles stay fully serial (p95 = 13 words), paragraphs win ~10% over
// the prior default of 64. Below 16, short inputs start paying rayon spawn
// overhead. See examples/parallel_sweep.rs.
const DEFAULT_PARALLEL_THRESHOLD: usize = 32;

pub struct Detector {
    enabled: Vec<Lang>,
    unknown_threshold: f32,
    parallel_threshold: usize,
}

impl Detector {
    pub fn new() -> Self {
        Self {
            enabled: Lang::all_enabled().to_vec(),
            unknown_threshold: DEFAULT_UNKNOWN_THRESHOLD,
            parallel_threshold: DEFAULT_PARALLEL_THRESHOLD,
        }
    }

    pub fn builder() -> DetectorBuilder {
        DetectorBuilder::default()
    }

    pub fn detect(&self, input: &str) -> Output {
        self.detect_detailed(input).aggregate
    }

    pub fn detect_detailed(&self, input: &str) -> Detailed {
        let tokens = tokenize::tokenize(input);
        if tokens.is_empty() {
            return Detailed::empty();
        }

        let words = parallel::map_words(tokens, self.parallel_threshold, |t| self.score_word(t));
        let aggregate = score::aggregate(&self.enabled, &words, self.unknown_threshold);

        Detailed { words, aggregate }
    }

    fn score_word(&self, token: String) -> WordScore {
        let raw_hits = dict::lookup(&token);
        let filtered: SmallVec<[(Lang, u16); 4]> = raw_hits
            .into_iter()
            .filter(|(lang, _)| self.enabled.contains(lang))
            .collect();

        if !filtered.is_empty() {
            return WordScore {
                token: token.into_boxed_str(),
                scores: dict::rank_weighted_priors(&filtered),
                source: MatchSource::Dict,
            };
        }

        // Stage 2 fallback — score by character trigrams.
        let scores = ngram::score_word(&token, &self.enabled);
        if scores.is_empty() {
            return WordScore {
                token: token.into_boxed_str(),
                scores: SmallVec::new(),
                source: MatchSource::Unknown,
            };
        }

        WordScore {
            token: token.into_boxed_str(),
            scores,
            source: MatchSource::Ngram,
        }
    }
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct DetectorBuilder {
    langs: Option<Vec<Lang>>,
    unknown_threshold: Option<f32>,
    parallel_threshold: Option<usize>,
}

impl DetectorBuilder {
    pub fn only(mut self, langs: impl IntoIterator<Item = Lang>) -> Self {
        self.langs = Some(langs.into_iter().collect());
        self
    }

    pub fn unknown_threshold(mut self, t: f32) -> Self {
        self.unknown_threshold = Some(t);
        self
    }

    pub fn parallel_threshold(mut self, n: usize) -> Self {
        self.parallel_threshold = Some(n);
        self
    }

    pub fn build(self) -> Detector {
        Detector {
            enabled: self.langs.unwrap_or_else(|| Lang::all_enabled().to_vec()),
            unknown_threshold: self.unknown_threshold.unwrap_or(DEFAULT_UNKNOWN_THRESHOLD),
            parallel_threshold: self
                .parallel_threshold
                .unwrap_or(DEFAULT_PARALLEL_THRESHOLD),
        }
    }
}
