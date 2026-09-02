/**
 * Ergonomic facade over the generated telemetry binding.
 *
 * A node that ships every event it produces will spend more on reporting than
 * on the job it was installed to do, and on a satellite link that is money. A
 * reporter ships what is worth its bytes, counts everything either way, and
 * moves its own bar as the link gets more expensive, so the aggregate picture
 * survives even when the detail cannot be sent.
 *
 * The generated binding decides on the level alone, since that is all the core
 * reporter reads. The event a caller writes travels no further than this layer,
 * which hands it straight back when it should be shipped.
 *
 * @packageDocumentation
 */

import {
  Level as NativeLevel,
  LinkCost as NativeLinkCost,
  linkCostThreshold as nativeThreshold,
  Reporter as NativeReporter,
  type Snapshot,
} from '../index'

export type { Snapshot }

/**
 * How urgent an event is.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`Level.Warn`) and a type annotation.
 */
export const Level = {
  /** Fine-grained detail, useful only when chasing a specific problem. */
  Trace: 'Trace',
  /** Diagnostic detail for development. */
  Debug: 'Debug',
  /** A normal, noteworthy event. */
  Info: 'Info',
  /** Something unexpected that the node recovered from. */
  Warn: 'Warn',
  /** A failure that needs attention. */
  Error: 'Error',
} as const

/** One of the {@link Level} choices. */
export type Level = (typeof Level)[keyof typeof Level]

/**
 * What the link back to the network currently costs.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const LinkCost = {
  /** Bytes are effectively free, such as on wired power and ethernet. */
  Free: 'Free',
  /** Bytes are paid for, such as on a cellular plan. */
  Metered: 'Metered',
  /** Bytes are scarce, such as on a satellite or long-range radio link. */
  Expensive: 'Expensive',
  /** Nothing can be shipped at all. */
  Offline: 'Offline',
} as const

/** One of the {@link LinkCost} choices. */
export type LinkCost = (typeof LinkCost)[keyof typeof LinkCost]

/**
 * A structured telemetry event.
 *
 * The code is a stable, short label such as `battery.low` rather than a
 * free-form message, so events stay small and group cleanly into counts.
 */
export interface TelemetryEvent {
  /** How urgent the event is. */
  level: Level
  /** A stable, short identifier for what happened. */
  code: string
  /** An optional measurement, such as the charge that triggered it. */
  value?: number
}

/** Returns the level a link cost calls for. */
export function linkCostThreshold(cost: LinkCost): Level {
  return nativeThreshold(cost as NativeLinkCost) as Level
}

/**
 * Records telemetry events, ships the ones worth their bytes, and counts them
 * all.
 */
export class Reporter {
  readonly #inner: NativeReporter

  /**
   * Creates a reporter that ships events at or above a level.
   *
   * @param threshold - The lowest level to ship.
   */
  constructor(threshold: Level) {
    this.#inner = new NativeReporter(threshold as NativeLevel)
  }

  /** The level this reporter is currently shipping from. */
  get threshold(): Level {
    return this.#inner.threshold as Level
  }

  set threshold(threshold: Level) {
    this.#inner.threshold = threshold as NativeLevel
  }

  /** How many events have been seen across every level. */
  get total(): number {
    return this.#inner.total
  }

  /** How many events passed the threshold and were shipped. */
  get emitted(): number {
    return this.#inner.emitted
  }

  /** How many events the threshold dropped. */
  get dropped(): number {
    return this.#inner.dropped
  }

  /**
   * Moves the threshold to match what the link now costs.
   *
   * @param cost - What the link currently costs.
   */
  adaptTo(cost: LinkCost): void {
    this.#inner.adaptTo(cost as NativeLinkCost)
  }

  /**
   * Records an event, returning it when it should be shipped.
   *
   * @param event - The event that occurred.
   * @returns The same event when it passed the threshold, or `null` when it was
   *   counted and dropped.
   */
  record(event: TelemetryEvent): TelemetryEvent | null {
    return this.#inner.record(event.level as NativeLevel) ? event : null
  }

  /**
   * Returns how many events have been seen at a level, shipped or not.
   *
   * @param level - The level to count.
   */
  count(level: Level): number {
    return this.#inner.count(level as NativeLevel)
  }

  /** Takes a snapshot of the counters to ship in place of the event stream. */
  snapshot(): Snapshot {
    return this.#inner.snapshot()
  }
}
