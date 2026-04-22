//! Conditional parallelism — below threshold, rayon spawn cost dominates
//! per-word work (stage 1 is nanoseconds; stage 2 costs more but still needs
//! enough words to amortize). See DESIGN.md §6.

// Batch-level rayon fan-out is gated on two cheap signals measured per call:
// cardinality (N) and approximate total work (sum of whitespace-delimited
// word counts). A single fixed batch-size threshold misroutes because the
// same N means very different work for short titles vs paragraphs.
//
// Boundaries set from the two-axis sweep in `examples/batch_routing_sweep.rs`:
//
// - `MIN_CARDINALITY = 2`: rayon can't meaningfully fan out a single item,
//   and the N=1 path adds only overhead. The sweep had one N=1 marginal win
//   (medium, 30 tokens, 0.93×) but also N=1 losses (long, 85 tokens, 1.29×),
//   so this is a pragmatic safety rule, not a "data proves N=1 always loses"
//   claim.
//
// - `MIN_APPROX_TOKENS = 50`: picked as the conservative boundary where
//   parallel reliably wins across all (N, input-length) cells tested. One
//   known missed marginal win: N=6 batches of very short (7-word) inputs
//   at 42 total tokens show a ~8% parallel win we leave on the table.
//   Acceptable trade for always avoiding the titles-at-N=4 regression.
pub(crate) const BATCH_MIN_CARDINALITY: usize = 2;
pub(crate) const BATCH_MIN_APPROX_TOKENS: usize = 50;

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
// checking the routing gates (`BATCH_MIN_CARDINALITY` and
// `BATCH_MIN_APPROX_TOKENS`) before invoking this — below the gates the
// batch API routes back through the regular per-call path so intra-document
// parallelism is preserved.
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
