use papagan::{
    Detailed as CoreDetailed, Detector as CoreDetector, Lang as CoreLang, Output as CoreOutput,
    WordScore as CoreWordScore,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn supported_language_codes() -> Vec<String> {
    papagan::supported_languages()
        .iter()
        .map(|lang| lang.iso_639_1().to_string())
        .collect()
}

fn invalid_lang_error(code: &str) -> PyErr {
    PyValueError::new_err(format!(
        "unsupported language `{code}`. supported languages: {}",
        supported_language_codes().join(", ")
    ))
}

fn parse_langs(langs: &[String]) -> PyResult<Vec<CoreLang>> {
    langs
        .iter()
        .map(|lang| CoreLang::from_iso_639_1(lang).ok_or_else(|| invalid_lang_error(lang)))
        .collect()
}

fn score_pairs(scores: &[(CoreLang, f32)]) -> Vec<(String, f32)> {
    scores
        .iter()
        .map(|(lang, score)| (lang.iso_639_1().to_string(), *score))
        .collect()
}

#[pyclass(module = "papagan._native", frozen)]
struct Lang;

#[pymethods]
#[allow(non_upper_case_globals)]
impl Lang {
    #[classattr]
    const De: &'static str = "de";
    #[classattr]
    const En: &'static str = "en";
    #[classattr]
    const Tr: &'static str = "tr";
    #[classattr]
    const Ru: &'static str = "ru";
    #[classattr]
    const Fr: &'static str = "fr";
    #[classattr]
    const Es: &'static str = "es";
    #[classattr]
    const It: &'static str = "it";
    #[classattr]
    const Nl: &'static str = "nl";
    #[classattr]
    const Pt: &'static str = "pt";
    #[classattr]
    const Pl: &'static str = "pl";
    #[classattr]
    const Unknown: &'static str = "?";

    #[staticmethod]
    fn all_enabled() -> Vec<String> {
        supported_language_codes()
    }
}

#[pyclass(skip_from_py_object, module = "papagan._native")]
#[derive(Clone)]
struct Output {
    scores: Vec<(String, f32)>,
}

#[pymethods]
impl Output {
    fn top(&self) -> (String, f32) {
        self.scores
            .first()
            .cloned()
            .unwrap_or_else(|| ("?".to_string(), 0.0))
    }

    fn distribution(&self) -> Vec<(String, f32)> {
        self.scores.clone()
    }
}

impl From<CoreOutput> for Output {
    fn from(output: CoreOutput) -> Self {
        Self {
            scores: score_pairs(output.distribution()),
        }
    }
}

#[pyclass(skip_from_py_object, module = "papagan._native")]
#[derive(Clone)]
struct WordScore {
    token: String,
    scores: Vec<(String, f32)>,
    source: String,
}

#[pymethods]
impl WordScore {
    #[getter]
    fn token(&self) -> String {
        self.token.clone()
    }

    #[getter]
    fn scores(&self) -> Vec<(String, f32)> {
        self.scores.clone()
    }

    #[getter]
    fn source(&self) -> String {
        self.source.clone()
    }
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

#[pyclass(skip_from_py_object, module = "papagan._native")]
#[derive(Clone)]
struct Detailed {
    words: Vec<WordScore>,
    aggregate: Output,
}

#[pymethods]
impl Detailed {
    #[getter]
    fn words(&self) -> Vec<WordScore> {
        self.words.clone()
    }

    #[getter]
    fn aggregate(&self) -> Output {
        self.aggregate.clone()
    }
}

impl From<CoreDetailed> for Detailed {
    fn from(detailed: CoreDetailed) -> Self {
        Self {
            words: detailed.words.into_iter().map(WordScore::from).collect(),
            aggregate: Output::from(detailed.aggregate),
        }
    }
}

#[pyclass(skip_from_py_object, module = "papagan._native")]
#[derive(Clone, Default)]
struct DetectorBuilder {
    only: Option<Vec<String>>,
    unknown_threshold: Option<f32>,
    parallel_threshold: Option<usize>,
}

#[pymethods]
impl DetectorBuilder {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    fn only(&self, langs: Vec<String>) -> Self {
        let mut next = self.clone();
        next.only = Some(langs);
        next
    }

    fn unknown_threshold(&self, threshold: f32) -> Self {
        let mut next = self.clone();
        next.unknown_threshold = Some(threshold);
        next
    }

    fn parallel_threshold(&self, threshold: usize) -> Self {
        let mut next = self.clone();
        next.parallel_threshold = Some(threshold);
        next
    }

    fn build(&self) -> PyResult<Detector> {
        Detector::from_options(
            self.only.clone(),
            self.unknown_threshold,
            self.parallel_threshold,
        )
    }
}

#[pyclass(module = "papagan._native")]
struct Detector {
    inner: CoreDetector,
}

impl Detector {
    fn from_options(
        only: Option<Vec<String>>,
        unknown_threshold: Option<f32>,
        parallel_threshold: Option<usize>,
    ) -> PyResult<Self> {
        let mut builder = CoreDetector::builder();
        if let Some(langs) = only {
            builder = builder.only(parse_langs(&langs)?);
        }
        if let Some(threshold) = unknown_threshold {
            builder = builder.unknown_threshold(threshold);
        }
        if let Some(threshold) = parallel_threshold {
            builder = builder.parallel_threshold(threshold);
        }
        Ok(Self {
            inner: builder.build(),
        })
    }
}

#[pymethods]
impl Detector {
    #[new]
    #[pyo3(signature = (*, only=None, unknown_threshold=None, parallel_threshold=None))]
    fn new(
        only: Option<Vec<String>>,
        unknown_threshold: Option<f32>,
        parallel_threshold: Option<usize>,
    ) -> PyResult<Self> {
        Self::from_options(only, unknown_threshold, parallel_threshold)
    }

    #[staticmethod]
    fn builder() -> DetectorBuilder {
        DetectorBuilder::default()
    }

    #[staticmethod]
    fn supported_languages() -> Vec<String> {
        supported_language_codes()
    }

    fn detect(&self, input: &str) -> Output {
        Output::from(self.inner.detect(input))
    }

    fn detect_detailed(&self, input: &str) -> Detailed {
        Detailed::from(self.inner.detect_detailed(input))
    }

    // The `py.allow_threads` wrapper releases the GIL while the Rust batch
    // runs — rayon worker threads never touch Python state, so they execute
    // uncontended against any concurrent Python threads. This is what makes
    // `detect_batch` genuinely parallel for CPU-bound Python callers.
    fn detect_batch(&self, py: Python<'_>, inputs: Vec<String>) -> Vec<Output> {
        py.detach(|| {
            self.inner
                .detect_batch(&inputs)
                .into_iter()
                .map(Output::from)
                .collect()
        })
    }

    fn detect_detailed_batch(&self, py: Python<'_>, inputs: Vec<String>) -> Vec<Detailed> {
        py.detach(|| {
            self.inner
                .detect_detailed_batch(&inputs)
                .into_iter()
                .map(Detailed::from)
                .collect()
        })
    }
}

#[pyfunction]
fn supported_languages() -> Vec<String> {
    supported_language_codes()
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(supported_languages, m)?)?;
    m.add_class::<Detailed>()?;
    m.add_class::<Detector>()?;
    m.add_class::<DetectorBuilder>()?;
    m.add_class::<Lang>()?;
    m.add_class::<Output>()?;
    m.add_class::<WordScore>()?;
    Ok(())
}
