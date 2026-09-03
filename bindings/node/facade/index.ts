/**
 * Ergonomic facade over the generated pamoja Node binding.
 *
 * This hand-written layer is the package's default entry point. It adds
 * idiomatic ergonomics - rejected promises for errors, async iteration, plain
 * JavaScript values instead of buffers - without adding behavior; all real work
 * happens in the native core reached through the generated contract.
 *
 * Each capability also has its own entry point (`@pamoja/core/mqtt`,
 * `/security`, `/codec`, `/kit`, `/serial`, `/modbus`, `/can`, `/gpio`,
 * `/sensors`, `/actuators`, `/lora`, `/lorawan`, `/mesh`, `/routing`,
 * `/audit`, `/session`, `/update`, `/power`, `/telemetry`, `/coap`,
 * `/loopback`, `/sync`, `/ladder`, `/bus`, `/sim`, `/transport`, `/profile`,
 * `/ros2`, `/zenoh`) for callers
 * who want only one, and the generated low-level surface remains available at
 * `@pamoja/core/raw`.
 *
 * @packageDocumentation
 */

export { version } from '../index'

export { MqttClient, type MqttClientOptions, type MqttMessage, Qos } from './mqtt'

export { DeviceIdentity, fingerprint, type Payload, verify } from './security'

export { fromCbor, packSamples, Quantizer, toCbor, unpackSamples } from './codec'

export {
  Anomaly,
  bearingBetween,
  Boundary,
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
} from './kit'

// The field-I/O, radio, and operational capabilities are namespaced rather than
// flattened: their names are ordinary words ("frame", "Session", "Level",
// "Manifest") that only read unambiguously with the capability in front of them.
export * as actuators from './actuators'

export * as audit from './audit'

export * as bus from './bus'

export * as can from './can'

export * as coap from './coap'

export * as gpio from './gpio'

export * as ladder from './ladder'

export * as loopback from './loopback'

export * as lora from './lora'

export * as lorawan from './lorawan'
export * as mavlink from './mavlink'

export * as mesh from './mesh'

export * as modbus from './modbus'

export * as power from './power'

export * as profile from './profile'

export * as ros2 from './ros2'

export * as routing from './routing'

export * as sensors from './sensors'

export * as serial from './serial'

export * as session from './session'

export * as sim from './sim'

export * as sync from './sync'

export * as telemetry from './telemetry'

export * as transport from './transport'

export * as update from './update'

export * as zenoh from './zenoh'
