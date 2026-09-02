/**
 * Ergonomic facade over the generated helper-math binding.
 *
 * The helpers are named for the goal rather than the technique, with the real
 * algorithm one layer down: smooth a noisy reading, hold a value with a PID, warn
 * before a tank runs dry, and notice when a tracked point leaves its area.
 *
 * They are synchronous and allocation-free in the core, so the facade re-exports
 * the generated classes rather than wrapping them; the one addition is a runtime
 * {@link Boundary} object, because the generated enum is types-only.
 *
 * @packageDocumentation
 */

export {
  Anomaly,
  bearingBetween,
  Calibration,
  type Coord,
  deadband,
  Debounce,
  Depletion,
  distanceBetween,
  Geofence,
  Kalman,
  Median,
  Pid,
  Ramp,
  Smoother,
  Surge,
  Thermostat,
  Trend,
  Window,
  WINDOW_CAPACITY,
} from '../index'

/**
 * Where a fix sits relative to a {@link Geofence}, including the moment it
 * crosses.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`Boundary.Exited`) and a type annotation.
 */
export const Boundary = {
  /** The fix is inside the fence and was inside before, or is the first fix inside. */
  Inside: 'Inside',
  /** The fix is outside the fence and was outside before, or is the first fix outside. */
  Outside: 'Outside',
  /** The fix just crossed from inside to outside: the moment to raise a breach alert. */
  Exited: 'Exited',
  /** The fix just crossed from outside back inside. */
  Entered: 'Entered',
} as const

/** One of the {@link Boundary} states. */
export type Boundary = (typeof Boundary)[keyof typeof Boundary]
