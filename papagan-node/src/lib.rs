use std::sync::Arc;

use napi::bindgen_prelude::{AsyncTask, Env, Error, Result, Status, Task};
use napi_derive::napi;
use papagan::{
    Detailed as CoreDetailed, Detector as CoreDetector, Lang as CoreLang, Output as CoreOutput,
    WordScore as CoreWordScore,
};

fn supported_language_codes() -> Vec<String> {
    papagan::supported_languages()
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
    // Arc so we can cheaply share a reference with async tasks running on the
    // libuv thread pool. `CoreDetector` is `Send + Sync` (primitives + `Vec<Lang>`),
    // so `Arc<CoreDetector>` is safe to move between threads.
    inner: Arc<CoreDetector>,
}

pub struct DetectBatchTask {
    detector: Arc<CoreDetector>,
    inputs: Vec<String>,
}

impl Task for DetectBatchTask {
    type Output = Vec<Output>;
    type JsValue = Vec<Output>;

    fn compute(&mut self) -> Result<Self::Output> {
        // Runs on libuv's thread pool — V8 event loop stays free.
        Ok(self
            .detector
            .detect_batch(&self.inputs)
            .into_iter()
            .map(Output::from)
            .collect())
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

pub struct DetectDetailedBatchTask {
    detector: Arc<CoreDetector>,
    inputs: Vec<String>,
}

impl Task for DetectDetailedBatchTask {
    type Output = Vec<Detailed>;
    type JsValue = Vec<Detailed>;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(self
            .detector
            .detect_detailed_batch(&self.inputs)
            .into_iter()
            .map(Detailed::from)
            .collect())
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
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
            inner: Arc::new(builder.build()),
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

    // Sync batch call — blocks the V8 thread for the duration of detection.
    // For most Node workloads (REST handlers, CLI tools) this is fine since
    // the batch is N× faster than calling `detect` in a loop, so total wall
    // time on the hot path is reduced. For long-running batches on
    // latency-sensitive event loops, use `detect_batch_async` instead.
    #[napi]
    pub fn detect_batch(&self, inputs: Vec<String>) -> Vec<Output> {
        self.inner
            .detect_batch(&inputs)
            .into_iter()
            .map(Output::from)
            .collect()
    }

    #[napi]
    pub fn detect_detailed_batch(&self, inputs: Vec<String>) -> Vec<Detailed> {
        self.inner
            .detect_detailed_batch(&inputs)
            .into_iter()
            .map(Detailed::from)
            .collect()
    }

    // Async batch — returns a Promise. Detection runs on libuv's thread pool,
    // so the V8 event loop stays responsive during the batch. Use this in
    // request handlers where tail-latency on other work matters more than
    // throughput on this particular call.
    #[napi(ts_return_type = "Promise<Output[]>")]
    pub fn detect_batch_async(&self, inputs: Vec<String>) -> AsyncTask<DetectBatchTask> {
        AsyncTask::new(DetectBatchTask {
            detector: self.inner.clone(),
            inputs,
        })
    }

    #[napi(ts_return_type = "Promise<Detailed[]>")]
    pub fn detect_detailed_batch_async(
        &self,
        inputs: Vec<String>,
    ) -> AsyncTask<DetectDetailedBatchTask> {
        AsyncTask::new(DetectDetailedBatchTask {
            detector: self.inner.clone(),
            inputs,
        })
    }
}

#[napi]
pub fn supported_languages() -> Vec<String> {
    supported_language_codes()
}
