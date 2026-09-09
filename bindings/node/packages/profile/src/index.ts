/**
 * Ergonomic facade over the generated device-profile binding.
 *
 * A profile is a named, pre-wired bundle: a control policy, a publish topic,
 * and a power schedule. Instantiate one rather than choosing algorithms and
 * tuning constants by hand.
 *
 * A profile is the manifest, which loads from and saves to JSON so it ships as
 * a file. A controller is the decision logic that manifest describes: hand it a
 * reading and it says what the output should do and whether the reading crossed
 * a threshold worth raising. The presentation a dashboard reads travels inside
 * the manifest JSON and is read and built here as a typed {@link Presentation}.
 *
 * The alert and control kinds are re-exported as runtime objects, because the
 * generated enums are types-only.
 *
 * @packageDocumentation
 */

import type {
  AlertKind as AlertKindName,
  ControlKind as ControlKindName,
  Viz as VizName,
} from '@pamoja/native'

export { Controller, Profile } from '@pamoja/native'

export type {
  AlertReport,
  ControlPolicy,
  ElementSpec,
  PowerScheduleSpec,
  Presentation,
  Reaction,
  Theme,
} from '@pamoja/native'

/**
 * The graphic a dashboard draws an element with, named by the instrument rather
 * than the quantity. The values are the ones a manifest carries.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const Viz = {
  /** A rolling sparkline of recent values. */
  Spark: 'spark' as VizName,
  /** A 270-degree arch gauge, for a fraction or percentage. */
  Gauge: 'gauge' as VizName,
  /** A half-dial with a needle, for a pressure or flow reading. */
  Dial: 'dial' as VizName,
  /** A horizontal bar with a safe-band tick, for a level or stock. */
  Bar: 'bar' as VizName,
  /** A thermometer, for a temperature. */
  Thermometer: 'thermometer' as VizName,
  /** A liquid-filled droplet, for humidity or moisture. */
  Droplet: 'droplet' as VizName,
  /** A segmented battery cell, for a state of charge or voltage. */
  Battery: 'battery' as VizName,
  /** An anemometer, for wind speed. */
  Wind: 'wind' as VizName,
  /** A sun whose corona grows with the reading, for illuminance. */
  Sun: 'sun' as VizName,
  /** An acoustic waveform, for sound level or an acoustic event. */
  Wave: 'wave' as VizName,
  /** A labeled state chip, lit when the state reads as on. */
  Switch: 'switch' as VizName,
  /** A pipe valve, open along the flow or closed across it. */
  Valve: 'valve' as VizName,
  /** A row of hash-chained blocks, for a tamper-evident record count. */
  Chain: 'chain' as VizName,
  /** A neighbor-mesh topology map, for a mesh node's peers. */
  Mesh: 'mesh' as VizName,
  /** A plain numeric counter, for a node or network stat. */
  Count: 'count' as VizName,
} as const

/** One of the {@link Viz} choices. */
export type Viz = VizName

/**
 * Which control policy a profile applies to each reading.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const ControlKind = {
  /** Hold a reading near a setpoint by switching an output on and off. */
  Setpoint: 'Setpoint' as ControlKindName,
  /** Watch a falling level and warn before it reaches empty. */
  Level: 'Level' as ControlKindName,
  /** Warn when a reading changes faster than a limit. */
  Surge: 'Surge' as ControlKindName,
  /** Report readings only, with no output and no alerts. */
  Monitor: 'Monitor' as ControlKindName,
} as const

/** One of the {@link ControlKind} choices. */
export type ControlKind = ControlKindName

/**
 * Which threshold a reading crossed.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const AlertKind = {
  /** A controlled reading drifted outside its safe band. */
  OutOfRange: 'OutOfRange' as AlertKindName,
  /** A falling level will reach empty within a few more samples. */
  RunningOut: 'RunningOut' as AlertKindName,
  /** A reading is changing faster than its safe rate. */
  ChangingFast: 'ChangingFast' as AlertKindName,
} as const

/** One of the {@link AlertKind} choices. */
export type AlertKind = AlertKindName
