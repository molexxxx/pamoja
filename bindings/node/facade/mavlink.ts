/**
 * Ergonomic facade over the generated MAVLink binding.
 *
 * MAVLink is the language drones speak: PX4 and ArduPilot autopilots and MAVSDK
 * ground stations all exchange MAVLink frames, so talking to a vehicle means
 * putting exactly the right bytes on the wire and trusting the bytes that come
 * back. This is that byte layer: v1 and v2 frames, the CRC-16/MCRF4XX checksum
 * every frame carries, the per-message `CRC_EXTRA` seed that catches a frame
 * whose shape does not match, and MAVLink 2 signing.
 *
 * Nothing here is limited to the messages this build happens to know. The common
 * dialect's seeds are built in, and {@link Dialect} carries any others, derived
 * from a message definition the way the specification does.
 *
 * @packageDocumentation
 */

import {
  Dialect,
  MavlinkFrame,
  MavlinkParser,
  MavlinkSigner,
  MavlinkVerifier,
  type MavlinkField,
  type MavlinkHeader,
  type MavlinkVersion as NativeMavlinkVersion,
  MAVLINK_DEFAULT_TIMESTAMP_WINDOW,
  MAVLINK_KEY_LEN,
  MAVLINK_MAX_FRAME,
  MAVLINK_MAX_PAYLOAD,
  MAVLINK_SIGNATURE_LEN,
  mavlinkCrc16Mcrf4Xx,
  mavlinkKnownCrcExtra,
  mavlinkMessageCrcExtra,
  mavlinkTimestampFromUnixMicros,
} from '../index'

export {
  Dialect,
  MavlinkFrame,
  MavlinkParser,
  MavlinkSigner,
  MavlinkVerifier,
  type MavlinkField,
  type MavlinkHeader,
}

/** The largest payload a frame can carry, in bytes. */
export const MAX_PAYLOAD = MAVLINK_MAX_PAYLOAD

/** The largest frame, in bytes, header, checksum and signature included. */
export const MAX_FRAME = MAVLINK_MAX_FRAME

/** The length of a v2 signature block, in bytes. */
export const SIGNATURE_LEN = MAVLINK_SIGNATURE_LEN

/** The length of a signing key, in bytes. */
export const KEY_LEN = MAVLINK_KEY_LEN

/** The default window a verifier accepts a timestamp within. */
export const DEFAULT_TIMESTAMP_WINDOW = MAVLINK_DEFAULT_TIMESTAMP_WINDOW

/**
 * Which MAVLink wire format a frame uses.
 *
 * Provided as a runtime object, as the generated string enum is erased at
 * compile time and so has no value a JavaScript caller can reach.
 */
export const MavlinkVersion = {
  /** The original six-byte-header format. */
  V1: 'V1' as NativeMavlinkVersion,
  /** The current format: a 24-bit message id, flag bytes, and optional signing. */
  V2: 'V2' as NativeMavlinkVersion,
} as const

/** One of the {@link MavlinkVersion} values. */
export type MavlinkVersion = NativeMavlinkVersion

/**
 * Returns the CRC-16/MCRF4XX checksum of a byte string.
 *
 * This is the checksum every MAVLink frame carries, exposed because a host that
 * implements part of the protocol itself needs the same arithmetic.
 *
 * @param bytes - The data to checksum.
 * @returns The checksum.
 */
export function crc16(bytes: Buffer): number {
  return mavlinkCrc16Mcrf4Xx(bytes)
}

/**
 * Derives the `CRC_EXTRA` seed of a message from its definition.
 *
 * This is what makes a dialect this build has never seen usable: given a
 * message's name and its base fields in wire order, the seed comes out the same
 * as the one the dialect publishes, and a frame carrying that message then
 * checks like any other.
 *
 * Extension fields are excluded from the seed and must not be listed, which is
 * what lets a peer that predates them still check the frame.
 *
 * @param name - The message name, such as `HEARTBEAT`.
 * @param fields - The base fields in wire order.
 * @returns The seed.
 *
 * @example
 * ```ts
 * const seed = messageCrcExtra('PRIVATE_STATUS', [
 *   { typeName: 'uint32_t', fieldName: 'uptime' },
 * ])
 * ```
 */
export function messageCrcExtra(name: string, fields: MavlinkField[]): number {
  return mavlinkMessageCrcExtra(name, fields)
}

/**
 * Returns the `CRC_EXTRA` the common dialect publishes for a message id.
 *
 * @param msgid - The message id to look up.
 * @returns The seed, or `null` for an id outside the common dialect, which is
 *   what a {@link Dialect} is for.
 */
export function knownCrcExtra(msgid: number): number | null {
  return mavlinkKnownCrcExtra(msgid) ?? null
}

/**
 * Converts Unix time into the timestamp MAVLink signing counts in.
 *
 * @param unixMicros - The time in microseconds since the Unix epoch.
 * @returns The signing timestamp, in units of ten microseconds since 2015.
 */
export function timestampFromUnixMicros(unixMicros: number): number {
  return mavlinkTimestampFromUnixMicros(unixMicros)
}

/**
 * Returns a signing timestamp for now.
 *
 * @returns The signing timestamp matching the current clock.
 */
export function timestampNow(): number {
  return mavlinkTimestampFromUnixMicros(Date.now() * 1000)
}

/**
 * Builds a v2 frame carrying a message the common dialect defines.
 *
 * The seed is looked up rather than passed, which is the usual case: a sender
 * emitting a standard message should not have to know its checksum constant.
 *
 * @param header - The addressing fields to stamp on the frame.
 * @param msgid - The message id.
 * @param payload - The message payload.
 * @returns The frame ready to send.
 * @throws If the id is outside the common dialect, in which case build the
 *   frame with {@link MavlinkFrame.raw} and a seed of your own.
 */
export function frame(header: MavlinkHeader, msgid: number, payload: Buffer): MavlinkFrame {
  const crcExtra = mavlinkKnownCrcExtra(msgid)
  if (crcExtra == null) {
    throw new Error(
      `message ${msgid} is not in the common dialect; supply its CRC_EXTRA with MavlinkFrame.raw`,
    )
  }
  return MavlinkFrame.encodeV2(header, msgid, payload, crcExtra)
}
