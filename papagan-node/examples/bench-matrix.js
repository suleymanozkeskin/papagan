'use strict'

// Node-side rows of the binding × API × workload performance matrix.
// Uses the two open fixtures: Tatoeba short sentences (accuracy_large.tsv,
// regenerate via `cargo xtask fetch-eval`) and Leipzig news paragraphs
// (bench/paragraphs.json, regenerate via `cargo xtask fetch-leipzig`).
//
// Companion harnesses: papagan/examples/bench_matrix.rs,
// papagan-py/bench/matrix.py. Orchestrator: scripts/bench-matrix.sh.
//
// Usage:
//   node papagan-node/examples/bench-matrix.js

const fs = require('node:fs')
const path = require('node:path')

const REPO_ROOT = path.resolve(__dirname, '..', '..')
const TATOEBA_PATH = path.join(REPO_ROOT, 'papagan', 'tests', 'fixtures', 'accuracy_large.tsv')
const PARAGRAPHS_PATH = path.join(REPO_ROOT, 'bench', 'paragraphs.json')

const { Detector } = require('../index.js')

const ITERS = 7

function loadJson(p) {
  try { return JSON.parse(fs.readFileSync(p, 'utf8')) }
  catch {
    process.stderr.write(`${p}: missing\nhint: regenerate with \`cargo xtask fetch-leipzig\`.\n`)
    process.exit(1)
  }
}

function loadTsv(p) {
  let text
  try { text = fs.readFileSync(p, 'utf8') }
  catch {
    process.stderr.write(`${p}: missing\nhint: regenerate with \`cargo xtask fetch-eval\`.\n`)
    process.exit(1)
  }
  const out = []
  for (const line of text.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue
    const tab = line.indexOf('\t')
    if (tab > 0) out.push(line.slice(tab + 1))
  }
  return out
}

function bench(fn) {
  const samples = []
  for (let i = 0; i < ITERS; i++) {
    const t = process.hrtime.bigint()
    fn()
    samples.push(Number(process.hrtime.bigint() - t) / 1e6)
  }
  samples.sort((a, b) => a - b)
  return samples[Math.floor(samples.length / 2)]
}

async function benchAsync(fn) {
  const samples = []
  for (let i = 0; i < ITERS; i++) {
    const t = process.hrtime.bigint()
    await fn()
    samples.push(Number(process.hrtime.bigint() - t) / 1e6)
  }
  samples.sort((a, b) => a - b)
  return samples[Math.floor(samples.length / 2)]
}

function fixtureStats(items) {
  const tokens = items.reduce((acc, s) => acc + (s.split(/\s+/).filter(Boolean).length), 0)
  const bytes = items.reduce((acc, s) => acc + Buffer.byteLength(s, 'utf8'), 0)
  const tok = tokens >= 1000 ? `${Math.round(tokens / 1000)}k` : `${tokens}`
  const kb = `${Math.round(bytes / 1000)} KB`
  return { tokens, tok, kb }
}

function nsPerToken(ms, tokens) {
  if (!tokens) return '—'
  return `${Math.round(ms * 1_000_000 / tokens)}`
}

async function main() {
  const tatoeba = loadTsv(TATOEBA_PATH)
  const paragraphs = loadJson(PARAGRAPHS_PATH)
  const d = new Detector()
  for (const t of tatoeba.slice(0, 20)) d.detect(t)
  for (const p of paragraphs.slice(0, 20)) d.detect(p)

  const tatStats = fixtureStats(tatoeba)
  const parStats = fixtureStats(paragraphs)

  const tl = bench(() => { for (const t of tatoeba) d.detect(t) })
  const tb = bench(() => d.detectBatch(tatoeba))
  const ta = await benchAsync(() => d.detectBatchAsync(tatoeba))
  const pl = bench(() => { for (const p of paragraphs) d.detect(p) })
  const pb = bench(() => d.detectBatch(paragraphs))
  const pa = await benchAsync(() => d.detectBatchAsync(paragraphs))

  console.log(
    `| Node | papagan | ${tatStats.tok} | ${tatStats.kb} | ${tl.toFixed(2)} | ${nsPerToken(tl, tatStats.tokens)} | ${tb.toFixed(2)} | ${nsPerToken(tb, tatStats.tokens)} | ${ta.toFixed(2)} |`
  )
  console.log(
    `| Node | papagan | ${parStats.tok} | ${parStats.kb} | ${pl.toFixed(2)} | ${nsPerToken(pl, parStats.tokens)} | ${pb.toFixed(2)} | ${nsPerToken(pb, parStats.tokens)} | ${pa.toFixed(2)} |`
  )
}

main().catch(e => { console.error(e); process.exit(1) })
