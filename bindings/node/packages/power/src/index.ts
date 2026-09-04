/**
 * Ergonomic facade over the generated power binding.
 *
 * A node on a battery and a panel has to decide how often to do anything at
 * all. A duty cycle says how the time splits between working and sleeping; a
 * power plan says how that split should change as the charge falls, so a node
 * that would otherwise go dark in a cloudy week keeps reporting, less often.
 *
 * The power mode is re-exported as a runtime {@link PowerMode} object, because
 * the generated enum is types-only.
 *
 * @packageDocumentation
 */

import type { PowerMode as PowerModeName } from '@pamoja/native'

export { DutyCycle, PowerPlan } from '@pamoja/native'

/**
 * What a node should be doing at the current state of charge.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`PowerMode.Saver`) and a type annotation.
 */
export const PowerMode = {
  /** Full duty, because the charge is healthy. */
  Active: 'Active' as PowerModeName,
  /** Reduced duty, to conserve charge. */
  Saver: 'Saver' as PowerModeName,
  /** Minimum duty, to stay alive as long as possible. */
  Critical: 'Critical' as PowerModeName,
} as const

/** One of the {@link PowerMode} choices. */
export type PowerMode = PowerModeName
