//! Guardrail: overall accuracy on the fixture set must stay above a baseline.
//! If this trips after a change, either the change is a genuine regression or
//! the baseline needs explicit bumping — never silently.
//!
//! Baseline as of 2026-04-16: 99% (99/100). Threshold set to 95% to absorb
//! small fluctuations from knob tuning without masking real regressions.

#![cfg(feature = "all-langs")]

const MIN_ACCURACY_PCT: f32 = 95.0;

#[test]
fn overall_accuracy_does_not_regress() {
    let content = include_str!("fixtures/accuracy.tsv");
    let detector = lang_detect::Detector::new();

    let mut correct = 0;
    let mut total = 0;
    let mut failures = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((iso, text)) = line.split_once('\t') else {
            continue;
        };
        total += 1;
        let predicted = detector.detect(text).top().0.iso_639_1();
        if predicted == iso {
            correct += 1;
        } else {
            failures.push(format!("  expected={iso} got={predicted}  | {text}"));
        }
    }

    let accuracy = 100.0 * correct as f32 / total as f32;
    assert!(
        accuracy >= MIN_ACCURACY_PCT,
        "accuracy regressed to {accuracy:.1}% (threshold {MIN_ACCURACY_PCT}%); {correct}/{total} correct.\nFailures:\n{}",
        failures.join("\n")
    );
}
