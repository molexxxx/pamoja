/**
 * The whole pamoja framework in one package.
 *
 * Each capability is its own package (`@pamoja/mqtt`, `@pamoja/security`, and so
 * on), a hand-written facade in the language's idiom over the generated napi-rs
 * contract in `@pamoja/native`: rejected promises for errors, async iteration,
 * plain JavaScript values instead of buffers. This package depends on all of them
 * and on `@pamoja/core`, the engine's surface, and re-exports them, flattening the
 * identity, codec, helper, and MQTT surfaces and namespacing the rest, whose names
 * are ordinary words that only read unambiguously with the capability in front of
 * them.
 *
 * @packageDocumentation
 */

export { version } from '@pamoja/core'

export { MqttClient, type MqttClientOptions, type MqttMessage, Qos } from '@pamoja/mqtt'

export { DeviceIdentity, fingerprint, type Payload, verify } from '@pamoja/security'

export { fromCbor, packSamples, Quantizer, toCbor, unpackSamples } from '@pamoja/codec'

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
} from '@pamoja/kit'

// The field-I/O, radio, and operational capabilities are namespaced rather than
// flattened: their names are ordinary words ("frame", "Session", "Level",
// "Manifest") that only read unambiguously with the capability in front of them.
export * as actuators from '@pamoja/actuators'

export * as audit from '@pamoja/audit'

export * as bus from '@pamoja/bus'

export * as can from '@pamoja/can'

export * as coap from '@pamoja/coap'

export * as gpio from '@pamoja/gpio'

export * as ladder from '@pamoja/ladder'

export * as loopback from '@pamoja/loopback'

export * as lora from '@pamoja/lora'

export * as lorawan from '@pamoja/lorawan'
export * as mavlink from '@pamoja/mavlink'

export * as mesh from '@pamoja/mesh'

export * as modbus from '@pamoja/modbus'

export * as power from '@pamoja/power'

export * as profile from '@pamoja/profile'

export * as ros2 from '@pamoja/ros2'

export * as routing from '@pamoja/routing'

export * as sensors from '@pamoja/sensors'

export * as serial from '@pamoja/serial'

export * as session from '@pamoja/session'

export * as sim from '@pamoja/sim'

export * as sync from '@pamoja/sync'

export * as telemetry from '@pamoja/telemetry'

export * as transport from '@pamoja/core/transport'

export * as update from '@pamoja/update'

export * as zenoh from '@pamoja/zenoh'
