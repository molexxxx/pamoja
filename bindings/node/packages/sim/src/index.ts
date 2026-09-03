/**
 * Ergonomic facade over the generated simulation binding.
 *
 * Drive a whole node with no hardware attached: a sensor that drifts, a replay
 * of a real capture, an actuator that records what it was told, and a robot
 * that moves only in arithmetic. A lossy link is `Transport.degraded`, since it
 * wraps a transport rather than standing alone.
 *
 * @packageDocumentation
 */

export {
  RecordingActuatorHandle as RecordingActuator,
  Replay,
  SimulatedRobot,
  SimulatedSensor,
} from '@pamoja/native'

export type { Pose, Twist } from '@pamoja/native'
