//! Stage 2 — character trigram fallback. See DESIGN.md §3 for the mechanism.

mod data;
mod model;

pub(crate) use model::score_word;
