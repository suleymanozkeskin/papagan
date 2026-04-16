//! Conditional parallelism — below threshold, rayon spawn cost dominates
//! per-word work (stage 1 is nanoseconds; stage 2 costs more but still needs
//! enough words to amortize). See DESIGN.md §6.

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
