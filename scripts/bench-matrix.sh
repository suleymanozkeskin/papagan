#!/usr/bin/env bash
# Run the binding × API × workload benchmark matrix across Rust, Python,
# and Node, with competing Python libraries included for direct
# comparison. Emits a combined markdown table ready to paste into the
# top-level README.
#
# Fixtures are open, reproducible:
#   - papagan/tests/fixtures/accuracy_large.tsv  (Tatoeba, 5k short
#                                                 sentences; `cargo xtask
#                                                 fetch-eval` to regenerate)
#   - bench/paragraphs.json                       (Leipzig news, 1k long
#                                                 paragraphs; `cargo xtask
#                                                 fetch-leipzig`)
#
# Prerequisites (fail early and say so):
#   - Python venv at papagan-py/.venv with current build
#       (`cd papagan-py && source .venv/bin/activate && maturin develop --release`)
#     Competitor rows appear only if the libs are installed:
#       pip install py3langid langdetect lingua-language-detector
#   - Node release binary built
#       (`cd papagan-node && npm run build:platform`)
#
# Usage:   ./scripts/bench-matrix.sh [> reports/bench-matrix.md]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

[[ -f papagan/tests/fixtures/accuracy_large.tsv ]] || {
  echo "missing papagan/tests/fixtures/accuracy_large.tsv — run 'cargo xtask fetch-eval'" >&2; exit 1;
}
[[ -f bench/paragraphs.json ]] || {
  echo "missing bench/paragraphs.json — run 'cargo xtask fetch-leipzig'" >&2; exit 1;
}
[[ -x papagan-py/.venv/bin/python ]] || { echo "missing papagan-py venv — see script header" >&2; exit 1; }
[[ -f papagan-node/papagan.darwin-arm64.node || -f papagan-node/papagan.linux-x64-gnu.node || -f papagan-node/papagan.linux-arm64-gnu.node || -f papagan-node/papagan.win32-x64-msvc.node ]] \
  || { echo "missing papagan-node native binary — run 'npm run build:platform' in papagan-node/" >&2; exit 1; }

echo "## Performance matrix — binding × library × workload"
echo
echo "Measured on $(uname -sm), $(date -u +%Y-%m-%d).  Median of 7 full-sweep iterations per cell.  All numbers in ms."
echo
echo "| Binding | Library | Tokens | Bytes | Loop (ms) | Loop (ns/tok) | Batch (ms) | Batch (ns/tok) | Async (ms) |"
echo "|---|---|---:|---:|---:|---:|---:|---:|---:|"

# Rust rows.
cargo run --release --example bench_matrix --features all-langs 2>/dev/null

# Python rows (papagan + competitors if installed).
papagan-py/.venv/bin/python papagan-py/bench/matrix.py

# Node rows.
node papagan-node/examples/bench-matrix.js

echo
echo "### How to reproduce"
echo
echo "\`\`\`bash"
echo "./scripts/bench-matrix.sh  # emits the table above"
echo "\`\`\`"
