//! Fast, small-binary language detection with opt-in language packs.
//!
//! Architecture and design decisions: see `DESIGN.md` at the workspace root.

mod detector;
mod dict;
mod lang;
mod ngram;
mod parallel;
mod score;
mod tokenize;

pub use detector::{Detector, DetectorBuilder};
pub use lang::Lang;
pub use score::{Detailed, MatchSource, Output, WordScore};
