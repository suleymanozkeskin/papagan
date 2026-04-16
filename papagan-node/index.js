'use strict'

const fs = require('node:fs')
const path = require('node:path')

function loadNative() {
  const root = __dirname
  const candidates = [
    'papagan.node',
    `papagan.${process.platform}-${process.arch}.node`,
    `index.${process.platform}-${process.arch}.node`,
    'index.node',
  ]

  let lastError
  for (const candidate of candidates) {
    const target = path.join(root, candidate)
    if (!fs.existsSync(target)) {
      continue
    }
    try {
      return require(target)
    } catch (error) {
      lastError = error
    }
  }

  throw lastError ?? new Error('Unable to locate the papagan native module')
}

const native = loadNative()

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
    this._inner = new native.NativeDetector(only, unknownThreshold, parallelThreshold)
  }

  static builder() {
    return new DetectorBuilder()
  }

  static supported_languages() {
    return native.supportedLanguages()
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
