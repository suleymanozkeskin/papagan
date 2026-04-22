'use strict'

// Measures how much the Node event loop stalls during a batch call.
// Fires a setInterval every 5ms and records the actual gap between fires
// — when the V8 thread is blocked, gaps balloon.
//
// Usage:  node examples/event-loop-latency.js
// Requires bench/paragraphs.json (`cargo xtask fetch-leipzig`).

const fs = require('node:fs')
const path = require('node:path')
const { Detector } = require('../index.js')

const paragraphsPath = path.resolve(__dirname, '../../bench/paragraphs.json')
const paragraphs = JSON.parse(fs.readFileSync(paragraphsPath, 'utf8'))

const d = new Detector()

function measureWith(runLoad) {
  return new Promise((resolve) => {
    const gaps = []
    let lastTick = process.hrtime.bigint()
    const interval = setInterval(() => {
      const now = process.hrtime.bigint()
      gaps.push(Number(now - lastTick) / 1e6)
      lastTick = now
    }, 5)

    runLoad().then((wallMs) => {
      setTimeout(() => {
        clearInterval(interval)
        gaps.sort((a, b) => a - b)
        resolve({
          wall: wallMs,
          count: gaps.length,
          max: gaps[gaps.length - 1],
          p99: gaps[Math.floor(gaps.length * 0.99)],
          p95: gaps[Math.floor(gaps.length * 0.95)],
          median: gaps[Math.floor(gaps.length * 0.5)],
        })
      }, 30)
    })
  })
}

async function main() {
  // Warmup both paths so the rayon pool + V8 JIT are primed.
  for (let i = 0; i < 3; i++) d.detectBatch(paragraphs.slice(0, 20))
  for (let i = 0; i < 3; i++) await d.detectBatchAsync(paragraphs.slice(0, 20))

  console.log('Event-loop latency during a batch of 1000 paragraphs')
  console.log('(setInterval fires every 5 ms; gap numbers = actual time between fires)')
  console.log()

  let r = await measureWith(async () => {
    const t0 = process.hrtime.bigint()
    d.detectBatch(paragraphs)
    return Number(process.hrtime.bigint() - t0) / 1e6
  })
  console.log('sync  detectBatch       ' +
    ` wall=${r.wall.toFixed(1)}ms` +
    `  loop-gap median=${r.median.toFixed(1)}ms` +
    `  p95=${r.p95.toFixed(1)}ms  p99=${r.p99.toFixed(1)}ms  max=${r.max.toFixed(1)}ms`)

  r = await measureWith(async () => {
    const t0 = process.hrtime.bigint()
    await d.detectBatchAsync(paragraphs)
    return Number(process.hrtime.bigint() - t0) / 1e6
  })
  console.log('async detectBatchAsync  ' +
    ` wall=${r.wall.toFixed(1)}ms` +
    `  loop-gap median=${r.median.toFixed(1)}ms` +
    `  p95=${r.p95.toFixed(1)}ms  p99=${r.p99.toFixed(1)}ms  max=${r.max.toFixed(1)}ms`)
}

main().catch((e) => { console.error(e); process.exit(1) })
