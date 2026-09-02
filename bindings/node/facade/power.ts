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

export { DutyCycle, PowerPlan } from '../index'

/**
 * What a node should be doing at the current state of charge.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`PowerMode.Saver`) and a type annotation.
 */
export const PowerMode = {
  /** Full duty, because the charge is healthy. */
  Active: 'Active',
  /** Reduced duty, to conserve charge. */
  Saver: 'Saver',
  /** Minimum duty, to stay alive as long as possible. */
  Critical: 'Critical',
} as const

/** One of the {@link PowerMode} choices. */
export type PowerMode = (typeof PowerMode)[keyof typeof PowerMode]
