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
 * the manifest JSON.
 *
 * The alert and control kinds are re-exported as runtime objects, because the
 * generated enums are types-only.
 *
 * @packageDocumentation
 */

import type { AlertKind as AlertKindName, ControlKind as ControlKindName } from '@pamoja/native'

export { Controller, Profile } from '@pamoja/native'

export type {
  AlertReport,
  ControlPolicy,
  PowerScheduleSpec,
  Reaction,
} from '@pamoja/native'

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
