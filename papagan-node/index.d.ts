export type LangCode = 'de' | 'en' | 'tr' | 'ru' | 'fr' | 'es' | 'it' | 'nl' | 'pt' | 'pl' | '?'
export type MatchSource = 'dict' | 'ngram' | 'unknown'
export type LangScore = [LangCode, number]

export declare const Lang: {
  readonly De: 'de'
  readonly En: 'en'
  readonly Tr: 'tr'
  readonly Ru: 'ru'
  readonly Fr: 'fr'
  readonly Es: 'es'
  readonly It: 'it'
  readonly Nl: 'nl'
  readonly Pt: 'pt'
  readonly Pl: 'pl'
  readonly Unknown: '?'
}

/** Document-level language distribution. Instances come from `Detector.detect`. */
export declare class Output {
  private constructor()
  /** Highest-scoring `[language, probability]` pair. */
  top(): LangScore
  /** Full distribution, sorted by descending score, sums to ~1.0. */
  distribution(): LangScore[]
}

/** Per-token scoring detail — produced by `Detector.detect_detailed`. */
export declare class WordScore {
  private constructor()
  get token(): string
  get scores(): LangScore[]
  get source(): MatchSource
}

/** Per-word scores plus the aggregated document distribution. */
export declare class Detailed {
  private constructor()
  get words(): WordScore[]
  get aggregate(): Output
}

/** Fluent builder for `Detector`. Each method returns a new builder. */
export declare class DetectorBuilder {
  /** Restrict detection to a subset of the compiled-in languages. */
  only(langs: LangCode[]): DetectorBuilder
  /** Aggregated score below this threshold returns `Lang.Unknown` (default 0.20). */
  unknown_threshold(threshold: number): DetectorBuilder
  /** camelCase alias of `unknown_threshold`. */
  unknownThreshold(threshold: number): DetectorBuilder
  /** Word count at or above which per-word scoring runs in parallel (default 32). */
  parallel_threshold(threshold: number): DetectorBuilder
  /** camelCase alias of `parallel_threshold`. */
  parallelThreshold(threshold: number): DetectorBuilder
  build(): Detector
}

export interface DetectorOptions {
  only?: LangCode[]
  unknown_threshold?: number
  unknownThreshold?: number
  parallel_threshold?: number
  parallelThreshold?: number
}

/** Language detector. Thread-safe; construct once and reuse. */
export declare class Detector {
  constructor(options?: DetectorOptions)
  static builder(): DetectorBuilder
  /** Codes of all languages compiled into this build. */
  static supported_languages(): LangCode[]
  /** camelCase alias of `supported_languages`. */
  static supportedLanguages(): LangCode[]
  detect(input: string): Output
  detect_detailed(input: string): Detailed
  /** camelCase alias of `detect_detailed`. */
  detectDetailed(input: string): Detailed
  /**
   * Detect languages for a batch of inputs. When the batch size is ≥ 4,
   * detection runs in parallel across documents via rayon and returns one
   * result per input in the original order. Blocks the V8 thread for the
   * duration — for large batches on request hot paths, offload to a Worker.
   */
  detect_batch(inputs: string[]): Output[]
  /** camelCase alias of `detect_batch`. */
  detectBatch(inputs: string[]): Output[]
  detect_detailed_batch(inputs: string[]): Detailed[]
  /** camelCase alias of `detect_detailed_batch`. */
  detectDetailedBatch(inputs: string[]): Detailed[]
}

export declare function supported_languages(): LangCode[]
export declare function supportedLanguages(): LangCode[]
