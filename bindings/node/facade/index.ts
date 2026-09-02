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
 * `/sensors`, `/actuators`) for callers who want only one, and the generated
 * low-level surface remains available at `@pamoja/core/raw`.
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

// The field-I/O capabilities are namespaced rather than flattened: their
// operations are named for their protocol ("frame", "raw", "parseFrame"), which
// only reads unambiguously with the protocol in front of it.
export * as actuators from './actuators'

export * as can from './can'

export * as gpio from './gpio'

export * as modbus from './modbus'

export * as sensors from './sensors'

export * as serial from './serial'
