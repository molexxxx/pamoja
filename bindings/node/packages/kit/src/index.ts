/**
 * Ergonomic facade over the generated helper-math binding.
 *
 * The helpers are named for the goal rather than the technique, with the real
 * algorithm one layer down: smooth a noisy reading, hold a value with a PID, warn
 * before a tank runs dry, notice when a tracked point leaves its area, and drive
 * a robot: chassis kinematics, an arm, odometry, waypoint guidance, and the
 * safety gate every motion command passes through.
 *
 * They are synchronous and allocation-free in the core, so the facade re-exports
 * the generated classes rather than wrapping them; the additions are the runtime
 * {@link Boundary}, {@link Edge}, and {@link Elbow} objects, because the generated
 * enums are types-only. A reading that is not a finite number is ignored by every
 * helper that keeps state, an {@link Anomaly} flags it, and the motion helpers
 * stop or hold rather than move on it.
 *
 * @packageDocumentation
 */

import type {
  BoundaryState as BoundaryName,
  Edge as EdgeName,
  Elbow as ElbowName,
} from '@pamoja/native'

export {
  Anomaly,
  bearingBetween,
  Calibration,
  celsiusToFahrenheit,
  celsiusToKelvin,
  Complementary,
  type Coord,
  deadband,
  Debounce,
  Depletion,
  dewPoint,
  distanceBetween,
  fahrenheitToCelsius,
  Geofence,
  hectopascalsToPascals,
  Kalman,
  kelvinToCelsius,
  kilopascalsToPascals,
  Median,
  pascalsToHectopascals,
  pascalsToKilopascals,
  pascalsToPsi,
  percentToRatio,
  Pid,
  psiToPascals,
  Ramp,
  ratioToPercent,
  Smoother,
  Surge,
  Thermostat,
  type Tilt,
  tiltFromAccel,
  Trend,
  Trigger,
  Window,
  WINDOW_CAPACITY,
} from '@pamoja/native'

export {
  Ackermann,
  type BodyMotion,
  type DhParameters,
  DiffDrive,
  EStop,
  Esc,
  forwardKinematics,
  type Guidance,
  type Joints,
  Limits,
  Mecanum,
  obstacleStop,
  Odometry,
  type Point,
  type Pose,
  type Position,
  Quadrature,
  QuadratureScale,
  type Reach,
  SafetyGate,
  ServoMap,
  type SideSpeeds,
  SkidSteer,
  Transform,
  type Twist,
  TwoLinkArm,
  Watchdog,
  WaypointFollower,
  type WheelSpeeds,
} from '@pamoja/native'

/**
 * Which way a {@link TwoLinkArm}'s elbow bends; both reach the same point.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const Elbow = {
  /** The elbow angle is positive, counter-clockwise. */
  Up: 'up' as ElbowName,
  /** The elbow angle is negative, clockwise. */
  Down: 'down' as ElbowName,
} as const

/** One of the {@link Elbow} branches. */
export type Elbow = ElbowName

/**
 * What a {@link Trigger} reports when a reading changes its state.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const Edge = {
  /** The reading just crossed the line: the condition became true. */
  Set: 'set' as EdgeName,
  /** The reading just came back past the release band: the condition stopped holding. */
  Cleared: 'cleared' as EdgeName,
} as const

/** One of the {@link Edge} states. */
export type Edge = EdgeName

/**
 * Where a fix sits relative to a {@link Geofence}, including the moment it
 * crosses.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`Boundary.Exited`) and a type annotation.
 */
export const Boundary = {
  /** The fix is inside the fence and was inside before, or is the first fix inside. */
  Inside: 'Inside' as BoundaryName,
  /** The fix is outside the fence and was outside before, or is the first fix outside. */
  Outside: 'Outside' as BoundaryName,
  /** The fix just crossed from inside to outside: the moment to raise a breach alert. */
  Exited: 'Exited' as BoundaryName,
  /** The fix just crossed from outside back inside. */
  Entered: 'Entered' as BoundaryName,
} as const

/** One of the {@link Boundary} states. */
export type Boundary = BoundaryName
