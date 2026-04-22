'use strict'

const { existsSync, readFileSync } = require('node:fs')
const { join } = require('node:path')

const { platform, arch } = process

let nativeBinding = null
let loadError = null

// Linux musl detection (vs glibc). Mirrors napi-rs standard pattern.
function isMusl() {
  if (process.report && typeof process.report.getReport === 'function') {
    const { glibcVersionRuntime } = process.report.getReport().header
    return !glibcVersionRuntime
  }
  try {
    const lddPath = require('node:child_process').execSync('which ldd').toString().trim()
    return readFileSync(lddPath, 'utf8').includes('musl')
  } catch (_) {
    return false
  }
}

function tryLoad(triple) {
  const localPath = join(__dirname, `papagan.${triple}.node`)
  try {
    if (existsSync(localPath)) {
      return require(localPath)
    }
    return require(`papagan-${triple}`)
  } catch (error) {
    loadError = error
    return null
  }
}

switch (platform) {
  case 'darwin':
    if (arch === 'arm64' || arch === 'x64') {
      nativeBinding = tryLoad(`darwin-${arch}`)
    }
    break
  case 'linux':
    if (arch === 'x64' || arch === 'arm64') {
      const libc = isMusl() ? 'musl' : 'gnu'
      nativeBinding = tryLoad(`linux-${arch}-${libc}`)
    }
    break
  case 'win32':
    if (arch === 'x64') {
      nativeBinding = tryLoad('win32-x64-msvc')
    }
    break
}

if (!nativeBinding) {
  throw loadError ?? new Error(
    `papagan does not ship a prebuilt binary for ${platform}-${arch}. ` +
    `Supported: darwin-arm64, darwin-x64, linux-x64 (gnu+musl), linux-arm64 (gnu+musl), win32-x64.`
  )
}

const Lang = Object.freeze({
  De: 'de',
  En: 'en',
  Tr: 'tr',
  Ru: 'ru',
  Fr: 'fr',
  Es: 'es',
  It: 'it',
  Nl: 'nl',
  Pt: 'pt',
  Pl: 'pl',
  Unknown: '?',
})

class Output {
  constructor(inner) {
    this._inner = inner
  }

  top() {
    return [this._inner.topLang, this._inner.topScore]
  }

  distribution() {
    return this._inner.scores.map(({ lang, score }) => [lang, score])
  }
}

class WordScore {
  constructor(inner) {
    this._inner = inner
  }

  get token() {
    return this._inner.token
  }

  get scores() {
    return this._inner.scores.map(({ lang, score }) => [lang, score])
  }

  get source() {
    return this._inner.source
  }
}

class Detailed {
  constructor(inner) {
    this._inner = inner
  }

  get words() {
    return this._inner.words.map((word) => new WordScore(word))
  }

  get aggregate() {
    return new Output(this._inner.aggregate)
  }
}

class DetectorBuilder {
  constructor(options = {}) {
    this._only = options.only ?? null
    this._unknownThreshold = options.unknownThreshold ?? null
    this._parallelThreshold = options.parallelThreshold ?? null
  }

  only(langs) {
    return new DetectorBuilder({
      only: langs,
      unknownThreshold: this._unknownThreshold,
      parallelThreshold: this._parallelThreshold,
    })
  }

  unknown_threshold(threshold) {
    return this.unknownThreshold(threshold)
  }

  unknownThreshold(threshold) {
    return new DetectorBuilder({
      only: this._only,
      unknownThreshold: threshold,
      parallelThreshold: this._parallelThreshold,
    })
  }

  parallel_threshold(threshold) {
    return this.parallelThreshold(threshold)
  }

  parallelThreshold(threshold) {
    return new DetectorBuilder({
      only: this._only,
      unknownThreshold: this._unknownThreshold,
      parallelThreshold: threshold,
    })
  }

  build() {
    return new Detector({
      only: this._only,
      unknownThreshold: this._unknownThreshold,
      parallelThreshold: this._parallelThreshold,
    })
  }
}

class Detector {
  constructor(options = {}) {
    const only = options.only ?? null
    const unknownThreshold = options.unknownThreshold ?? options.unknown_threshold ?? null
    const parallelThreshold = options.parallelThreshold ?? options.parallel_threshold ?? null
    this._inner = new nativeBinding.NativeDetector(only, unknownThreshold, parallelThreshold)
  }

  static builder() {
    return new DetectorBuilder()
  }

  static supported_languages() {
    return nativeBinding.supportedLanguages()
  }

  static supportedLanguages() {
    return Detector.supported_languages()
  }

  detect(input) {
    return new Output(this._inner.detect(input))
  }

  detect_detailed(input) {
    return this.detectDetailed(input)
  }

  detectDetailed(input) {
    return new Detailed(this._inner.detectDetailed(input))
  }

  detect_batch(inputs) {
    return this.detectBatch(inputs)
  }

  detectBatch(inputs) {
    return this._inner.detectBatch(inputs).map((raw) => new Output(raw))
  }

  detect_detailed_batch(inputs) {
    return this.detectDetailedBatch(inputs)
  }

  detectDetailedBatch(inputs) {
    return this._inner.detectDetailedBatch(inputs).map((raw) => new Detailed(raw))
  }

  detect_batch_async(inputs) {
    return this.detectBatchAsync(inputs)
  }

  async detectBatchAsync(inputs) {
    const raws = await this._inner.detectBatchAsync(inputs)
    return raws.map((raw) => new Output(raw))
  }

  detect_detailed_batch_async(inputs) {
    return this.detectDetailedBatchAsync(inputs)
  }

  async detectDetailedBatchAsync(inputs) {
    const raws = await this._inner.detectDetailedBatchAsync(inputs)
    return raws.map((raw) => new Detailed(raw))
  }
}

function supported_languages() {
  return Detector.supported_languages()
}

function supportedLanguages() {
  return supported_languages()
}

module.exports = {
  Detailed,
  Detector,
  DetectorBuilder,
  Lang,
  Output,
  WordScore,
  supported_languages,
  supportedLanguages,
}
