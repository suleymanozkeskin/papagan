//! Conditional parallelism — below threshold, rayon spawn cost dominates
//! per-word work (stage 1 is nanoseconds; stage 2 costs more but still needs
//! enough words to amortize). See DESIGN.md §6.

// Minimum batch size to use rayon at the document level. Below this the
// serial loop wins — rayon setup is ~µs, per-document work for short inputs
// can be ~µs too, so we'd pay more than we save.
pub(crate) const BATCH_PARALLEL_THRESHOLD: usize = 4;

#[cfg(feature = "parallel")]
pub(crate) fn map_words<F, T>(words: Vec<String>, threshold: usize, f: F) -> Vec<T>
where
    F: Fn(String) -> T + Sync + Send,
    T: Send,
{
    if words.len() < threshold {
        words.into_iter().map(f).collect()
    } else {
        use rayon::prelude::*;
        words.into_par_iter().map(f).collect()
    }
}

#[cfg(not(feature = "parallel"))]
pub(crate) fn map_words<F, T>(words: Vec<String>, _threshold: usize, f: F) -> Vec<T>
where
    F: Fn(String) -> T,
{
    words.into_iter().map(f).collect()
}

// Unconditional rayon fan-out across a batch. Callers are responsible for
// checking `BATCH_PARALLEL_THRESHOLD` before invoking this — the point of the
// split is that below the threshold the batch API routes back through the
// regular per-call path so intra-document parallelism is preserved.
#[cfg(feature = "parallel")]
pub(crate) fn par_map_batch<S, F, T>(inputs: &[S], f: F) -> Vec<T>
where
    S: Sync,
    F: Fn(&S) -> T + Sync + Send,
    T: Send,
{
    use rayon::prelude::*;
    inputs.par_iter().map(f).collect()
}

#[cfg(not(feature = "parallel"))]
pub(crate) fn par_map_batch<S, F, T>(inputs: &[S], f: F) -> Vec<T>
where
    F: Fn(&S) -> T,
{
    inputs.iter().map(f).collect()
}
