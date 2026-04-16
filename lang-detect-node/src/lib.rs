use lang_detect::{
    Detailed as CoreDetailed, Detector as CoreDetector, Lang as CoreLang, Output as CoreOutput,
    WordScore as CoreWordScore,
};
use napi::bindgen_prelude::{Error, Result, Status};
use napi_derive::napi;

fn supported_language_codes() -> Vec<String> {
    lang_detect::supported_languages()
        .iter()
        .map(|lang| lang.iso_639_1().to_string())
        .collect()
}

fn invalid_lang_error(code: &str) -> Error {
    Error::new(
        Status::InvalidArg,
        format!(
            "unsupported language `{code}`. supported languages: {}",
            supported_language_codes().join(", ")
        ),
    )
}

fn parse_langs(langs: &[String]) -> Result<Vec<CoreLang>> {
    langs
        .iter()
        .map(|lang| CoreLang::from_iso_639_1(lang).ok_or_else(|| invalid_lang_error(lang)))
        .collect()
}

fn score_pairs(scores: &[(CoreLang, f32)]) -> Vec<LangScore> {
    scores
        .iter()
        .map(|(lang, score)| LangScore {
            lang: lang.iso_639_1().to_string(),
            score: *score as f64,
        })
        .collect()
}

#[napi(object)]
pub struct LangScore {
    pub lang: String,
    pub score: f64,
}

#[napi(object)]
pub struct Output {
    pub scores: Vec<LangScore>,
    pub top_lang: String,
    pub top_score: f64,
}

impl From<CoreOutput> for Output {
    fn from(output: CoreOutput) -> Self {
        let scores = score_pairs(output.distribution());
        let (top_lang, top_score) = scores
            .first()
            .map(|score| (score.lang.clone(), score.score))
            .unwrap_or_else(|| ("?".to_string(), 0.0));
        Self {
            scores,
            top_lang,
            top_score,
        }
    }
}

#[napi(object)]
pub struct WordScore {
    pub token: String,
    pub scores: Vec<LangScore>,
    pub source: String,
}

impl From<CoreWordScore> for WordScore {
    fn from(word: CoreWordScore) -> Self {
        Self {
            token: word.token.into(),
            scores: score_pairs(&word.scores),
            source: word.source.as_str().to_string(),
        }
    }
}

#[napi(object)]
pub struct Detailed {
    pub words: Vec<WordScore>,
    pub aggregate: Output,
}

impl From<CoreDetailed> for Detailed {
    fn from(detailed: CoreDetailed) -> Self {
        Self {
            words: detailed.words.into_iter().map(WordScore::from).collect(),
            aggregate: Output::from(detailed.aggregate),
        }
    }
}

#[napi]
pub struct NativeDetector {
    inner: CoreDetector,
}

#[napi]
impl NativeDetector {
    #[napi(constructor)]
    pub fn new(
        only: Option<Vec<String>>,
        unknown_threshold: Option<f64>,
        parallel_threshold: Option<u32>,
    ) -> Result<Self> {
        let mut builder = CoreDetector::builder();
        if let Some(langs) = only {
            builder = builder.only(parse_langs(&langs)?);
        }
        if let Some(threshold) = unknown_threshold {
            builder = builder.unknown_threshold(threshold as f32);
        }
        if let Some(threshold) = parallel_threshold {
            builder = builder.parallel_threshold(threshold as usize);
        }
        Ok(Self {
            inner: builder.build(),
        })
    }

    #[napi]
    pub fn detect(&self, input: String) -> Output {
        Output::from(self.inner.detect(&input))
    }

    #[napi]
    pub fn detect_detailed(&self, input: String) -> Detailed {
        Detailed::from(self.inner.detect_detailed(&input))
    }
}

#[napi]
pub fn supported_languages() -> Vec<String> {
    supported_language_codes()
}
