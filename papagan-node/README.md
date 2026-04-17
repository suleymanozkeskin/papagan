# papagan

[![npm](https://img.shields.io/npm/v/papagan.svg)](https://www.npmjs.com/package/papagan)
[![types: TypeScript](https://img.shields.io/npm/types/papagan.svg)](https://www.npmjs.com/package/papagan)

Fast language detection for Node.js, powered by Rust (via [napi-rs](https://napi.rs)).

10 languages bundled, weighted per-word output, TypeScript types included.

## Install

```bash
bun add papagan
# or
pnpm add papagan
# or
yarn add papagan
# or
npm install papagan
```

Prebuilt binaries ship for Linux (x64, arm64 — glibc & musl), macOS (x64, arm64), and Windows (x64). Node.js 18+.

## Quick start

### JavaScript

```js
const { Detector } = require('papagan')

const detector = new Detector()

// Document-level detection
const output = detector.detect('Die Katze sitzt auf der Matte')
const [lang, confidence] = output.top()
console.log(`${lang}: ${confidence.toFixed(3)}`)
// de: 0.996

// Full distribution
for (const [lang, score] of output.distribution()) {
  console.log(`  ${lang}: ${score.toFixed(3)}`)
}
```

### TypeScript

```ts
import { Detector, Lang, type LangCode } from 'papagan'

const detector = new Detector()
const [lang, score]: [LangCode, number] = detector.detect('Hello world').top()

if (lang === Lang.En) {
  console.log(`English with ${score.toFixed(2)} confidence`)
}
```

### Per-word detail

```ts
const detailed = detector.detectDetailed('The cat is black. Die Katze ist schwarz.')

for (const word of detailed.words) {
  const [topLang, topScore] = word.scores.reduce((a, b) => (a[1] > b[1] ? a : b))
  console.log(`  ${word.token.padEnd(10)} [${word.source}]  ${topLang} (${topScore.toFixed(2)})`)
}

const [topLang, confidence] = detailed.aggregate.top()
```

### Restrict to specific languages

```ts
const detector = new Detector({ only: ['en', 'de'] })
// or via builder:
const detector = Detector.builder().only(['en', 'de']).build()
```

### Configuration

```ts
const detector = new Detector({
  only: ['en', 'de', 'fr'],    // restrict to a subset
  unknownThreshold: 0.25,      // below this => Lang.Unknown
  parallelThreshold: 128,      // parallelize at 128+ words
})
```

Both `camelCase` and `snake_case` are supported on builders and options (`unknownThreshold` or `unknown_threshold`, `detectDetailed` or `detect_detailed`, etc.) for ergonomic match to your codebase style.

## Supported languages

| Code | Language | Code | Language |
|---|---|---|---|
| `de` | German | `it` | Italian |
| `en` | English | `nl` | Dutch |
| `es` | Spanish | `pl` | Polish |
| `fr` | French | `pt` | Portuguese |
| `ru` | Russian | `tr` | Turkish |

All 10 languages bundled — no build-time configuration.

## Accuracy

~99.4% on a 5000-sentence Tatoeba evaluation across the 10 supported languages. Runs in a few microseconds per sentence on a modern laptop; for long documents, per-word scoring automatically parallelizes above a 64-word threshold.

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.

## Related

- [Rust crate](https://crates.io/crates/papagan) — the core library
- [Python package](https://pypi.org/project/papagan/) — Python bindings
- [GitHub](https://github.com/suleymanozkeskin/papagan) — source, issues, development
