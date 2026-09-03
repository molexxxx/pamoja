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
 * Above the bytes sits the shape: {@link MessageSchema} names a message's fields, so a
 * {@link MavlinkMessage} is filled in and read back by name rather than by byte offset,
 * and {@link MessageSchemaBuilder} describes a message this build has never heard of.
 *
 * Above the messages sit the exchanges: {@link MissionSender} and {@link MissionReceiver}
 * carry a plan between a station and a vehicle, {@link CommandProtocol} matches a command
 * to its acknowledgement and counts retries, and {@link offboard} builds setpoints. Each
 * takes a frame off the link and hands back the frame to send, with no IO or timers of its
 * own.
 *
 * @packageDocumentation
 */

import {
  CommandProtocol as NativeCommandProtocol,
  Dialect,
  MavlinkFrame,
  MavlinkMessage,
  MavlinkParser,
  MavlinkSigner,
  MavlinkVerifier,
  MessageSchema,
  MessageSchemaBuilder,
  MissionReceiver,
  MissionSender,
  ReceiverStep,
  SenderStep,
  type MavlinkField,
  type MavlinkFieldInfo,
  type MavlinkHeader,
  type MavlinkVersion as NativeMavlinkVersion,
  MAVLINK_DEFAULT_TIMESTAMP_WINDOW,
  MAVLINK_FIELD_CHAR,
  MAVLINK_FIELD_DOUBLE,
  MAVLINK_FIELD_FLOAT,
  MAVLINK_FIELD_INT16,
  MAVLINK_FIELD_INT32,
  MAVLINK_FIELD_INT64,
  MAVLINK_FIELD_INT8,
  MAVLINK_FIELD_UINT16,
  MAVLINK_FIELD_UINT32,
  MAVLINK_FIELD_UINT64,
  MAVLINK_FIELD_UINT8,
  MAVLINK_KEY_LEN,
  MAVLINK_MAX_FRAME,
  MAVLINK_MAX_PAYLOAD,
  MAVLINK_MAX_RETRIES,
  MAVLINK_SIGNATURE_LEN,
  MAVLINK_TYPEMASK_ACCELERATION,
  MAVLINK_TYPEMASK_FORCE,
  MAVLINK_TYPEMASK_POSITION,
  MAVLINK_TYPEMASK_VELOCITY,
  MAVLINK_TYPEMASK_YAW,
  MAVLINK_TYPEMASK_YAW_RATE,
  mavlinkCrc16Mcrf4Xx,
  mavlinkKnownCrcExtra,
  mavlinkKnownMessages,
  mavlinkMessageCrcExtra,
  mavlinkOffboardGlobalPosition,
  mavlinkOffboardLocalPosition,
  mavlinkOffboardLocalVelocity,
  mavlinkOffboardTypeMask,
  mavlinkTimestampFromUnixMicros,
} from '../index'

export {
  Dialect,
  MavlinkFrame,
  MavlinkMessage,
  MavlinkParser,
  MavlinkSigner,
  MavlinkVerifier,
  MessageSchema,
  MessageSchemaBuilder,
  MissionReceiver,
  MissionSender,
  ReceiverStep,
  SenderStep,
  type MavlinkField,
  type MavlinkFieldInfo,
  type MavlinkHeader,
}

/**
 * The number of times a request is retransmitted before a transfer is abandoned, as the
 * mission protocol recommends.
 */
export const MAX_RETRIES = MAVLINK_MAX_RETRIES

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

/**
 * The field types a message definition uses.
 *
 * Provided as a runtime object because the generated constants are plain numbers, and a
 * caller describing a dialect wants to write the type rather than remember its code.
 */
export const MavlinkFieldType = {
  /** `uint8_t`. */
  UINT8: MAVLINK_FIELD_UINT8,
  /** `int8_t`. */
  INT8: MAVLINK_FIELD_INT8,
  /** `char`; an array of these carries text. */
  CHAR: MAVLINK_FIELD_CHAR,
  /** `uint16_t`. */
  UINT16: MAVLINK_FIELD_UINT16,
  /** `int16_t`. */
  INT16: MAVLINK_FIELD_INT16,
  /** `uint32_t`. */
  UINT32: MAVLINK_FIELD_UINT32,
  /** `int32_t`. */
  INT32: MAVLINK_FIELD_INT32,
  /** `uint64_t`. */
  UINT64: MAVLINK_FIELD_UINT64,
  /** `int64_t`. */
  INT64: MAVLINK_FIELD_INT64,
  /** `float`. */
  FLOAT: MAVLINK_FIELD_FLOAT,
  /** `double`. */
  DOUBLE: MAVLINK_FIELD_DOUBLE,
} as const

/** One of the {@link MavlinkFieldType} values. */
export type MavlinkFieldTypeValue = (typeof MavlinkFieldType)[keyof typeof MavlinkFieldType]

/** What a field's value looks like outside the message: a number, an array, or text. */
export type MavlinkFieldValue = number | number[] | string

/** A whole message as plain values, keyed by field name. */
export type MavlinkFields = Record<string, MavlinkFieldValue>

/**
 * Returns the shape of a message the engine types.
 *
 * @param message - The message id or name, such as `33` or `GLOBAL_POSITION_INT`.
 * @returns The shape.
 * @throws If this build does not type that message, in which case describe it with
 *   {@link MessageSchemaBuilder}.
 *
 * @example
 * ```ts
 * const position = schemaFor('GLOBAL_POSITION_INT')
 * console.log(position.id, position.wireLen)
 * ```
 */
export function schemaFor(message: number | string): MessageSchema {
  return typeof message === 'number' ? MessageSchema.forId(message) : MessageSchema.forName(message)
}

/**
 * Returns the names of every message this build types, in message-id order.
 *
 * @returns The message names, each usable with {@link schemaFor}.
 */
export function knownMessages(): string[] {
  return mavlinkKnownMessages()
}

/**
 * Creates a message with every field zero.
 *
 * @param shape - The shape to build, or the id or name of a message the engine types.
 * @returns The zeroed message, ready for its fields to be set.
 *
 * @example
 * ```ts
 * const heartbeat = message('HEARTBEAT')
 * heartbeat.set('type', 18)
 * heartbeat.set('system_status', 4)
 * const frame = heartbeat.toFrame({ systemId: 1, componentId: 1, sequence: 0 })
 * ```
 */
export function message(shape: MessageSchema | number | string): MavlinkMessage {
  const schema = shape instanceof MessageSchema ? shape : schemaFor(shape)
  return MavlinkMessage.empty(schema)
}

/**
 * Reads a whole message as plain values, keyed by field name.
 *
 * A scalar field comes back as a number, an array field as an array, and a `char` array as
 * the text it carries, so a received message reads like an ordinary object.
 *
 * @param message - The message to read.
 * @param schema - The shape it was built from, which names its fields.
 * @returns The fields as plain values.
 */
export function toObject(message: MavlinkMessage, schema: MessageSchema): MavlinkFields {
  const values: MavlinkFields = {}
  for (const field of schema.fields) {
    if (field.arrayLen === 0) {
      values[field.name] = message.get(field.name)
    } else if (field.fieldType === MavlinkFieldType.CHAR) {
      values[field.name] = message.getText(field.name)
    } else {
      const elements: number[] = []
      for (let index = 0; index < field.arrayLen; index += 1) {
        elements.push(message.get(field.name, index))
      }
      values[field.name] = elements
    }
  }
  return values
}

/**
 * Builds a message from plain values, keyed by field name.
 *
 * A field left out stays zero, which is what a sender filling in part of a message wants.
 *
 * @param schema - The shape to build.
 * @param values - The fields to set.
 * @returns The message.
 * @throws If a name is not a field of the message, or a value does not fit its field.
 *
 * @example
 * ```ts
 * const position = schemaFor('GLOBAL_POSITION_INT')
 * const report = fromObject(position, { lat: -33856780, lon: 151215300, hdg: 18000 })
 * ```
 */
export function fromObject(schema: MessageSchema, values: MavlinkFields): MavlinkMessage {
  const built = MavlinkMessage.empty(schema)
  for (const [name, value] of Object.entries(values)) {
    if (typeof value === 'string') {
      built.setText(name, value)
    } else if (Array.isArray(value)) {
      value.forEach((element, index) => built.set(name, element, index))
    } else {
      built.set(name, value)
    }
  }
  return built
}

/**
 * The fields of a setpoint the autopilot should act on.
 *
 * Provided as a runtime object so a caller can combine flags by name; the rest of a
 * setpoint's fields are ignored.
 */
export const MavlinkTypeMask = {
  /** The position fields. */
  POSITION: MAVLINK_TYPEMASK_POSITION,
  /** The velocity fields. */
  VELOCITY: MAVLINK_TYPEMASK_VELOCITY,
  /** The acceleration fields. */
  ACCELERATION: MAVLINK_TYPEMASK_ACCELERATION,
  /** The yaw field. */
  YAW: MAVLINK_TYPEMASK_YAW,
  /** The yaw rate field. */
  YAW_RATE: MAVLINK_TYPEMASK_YAW_RATE,
  /** Treat the acceleration fields as a force. */
  FORCE: MAVLINK_TYPEMASK_FORCE,
} as const

/** What an incoming acknowledgement means for the command in flight. */
export interface AckOutcome {
  /** `unrelated`, `inProgress`, or `final`. */
  kind: 'unrelated' | 'inProgress' | 'final'
  /**
   * The progress percent when in progress (255 when the autopilot does not report one),
   * the `MAV_RESULT` when final, or `null` when unrelated.
   */
  value: number | null
}

/**
 * Tracks one command awaiting its acknowledgement.
 *
 * Wraps the generated class so an unrelated acknowledgement reports `null` rather than an
 * absent key, matching the rest of this package.
 */
export class CommandProtocol {
  readonly #native: NativeCommandProtocol

  /**
   * Starts tracking a command.
   *
   * @param command - The `MAV_CMD` id being sent.
   * @param maxRetries - How many times the command may be resent after a timeout before
   *   the caller gives up; defaults to {@link MAX_RETRIES}.
   */
  constructor(command: number, maxRetries: number = MAX_RETRIES) {
    this.#native = new NativeCommandProtocol(command, maxRetries)
  }

  /** The command id being tracked. */
  get command(): number {
    return this.#native.command
  }

  /**
   * The `confirmation` count to stamp on the command being sent: zero for the first
   * transmission, incremented on each retransmission.
   */
  get confirmation(): number {
    return this.#native.confirmation
  }

  /**
   * Classifies an incoming frame against the command in flight.
   *
   * @param frame - The frame off the link.
   * @returns The outcome, or `null` if the frame is not a `COMMAND_ACK`.
   */
  onFrame(frame: MavlinkFrame): AckOutcome | null {
    const outcome = this.#native.onFrame(frame)
    if (outcome == null) {
      return null
    }
    return { kind: outcome.kind as AckOutcome['kind'], value: outcome.value ?? null }
  }

  /**
   * Records a timeout and reports whether the command may be resent.
   *
   * @returns The new confirmation count to stamp on the resend, or `null` once the retry
   *   budget is exhausted.
   */
  onTimeout(): number | null {
    return this.#native.onTimeout() ?? null
  }
}

/** The setpoint constructors for offboard control, each returning a frame ready to send. */
export const offboard = {
  /**
   * Builds a setpoint `type_mask` from the fields to use.
   *
   * @param flags - A bitwise-or of the {@link MavlinkTypeMask} flags.
   * @returns The mask, as the `type_mask` field of a setpoint carries it.
   */
  typeMask(flags: number): number {
    return mavlinkOffboardTypeMask(flags)
  },

  /**
   * Builds a local-frame position setpoint.
   *
   * @param header - The addressing fields to stamp on the frame.
   * @param timeBootMs - The sender's boot timestamp, in milliseconds.
   * @param coordinateFrame - The `MAV_FRAME` of the setpoint.
   * @param targetSystem - The target system id.
   * @param targetComponent - The target component id.
   * @param x - The position along x, in metres in the chosen frame.
   * @param y - The position along y.
   * @param z - The position along z.
   * @returns The `SET_POSITION_TARGET_LOCAL_NED` frame.
   */
  localPosition(
    header: MavlinkHeader,
    timeBootMs: number,
    coordinateFrame: number,
    targetSystem: number,
    targetComponent: number,
    x: number,
    y: number,
    z: number,
  ): MavlinkFrame {
    return mavlinkOffboardLocalPosition(
      header,
      timeBootMs,
      coordinateFrame,
      targetSystem,
      targetComponent,
      x,
      y,
      z,
    )
  },

  /**
   * Builds a local-frame velocity setpoint.
   *
   * @param header - The addressing fields to stamp on the frame.
   * @param timeBootMs - The sender's boot timestamp, in milliseconds.
   * @param coordinateFrame - The `MAV_FRAME` of the setpoint.
   * @param targetSystem - The target system id.
   * @param targetComponent - The target component id.
   * @param vx - The velocity along x, in metres per second in the chosen frame.
   * @param vy - The velocity along y.
   * @param vz - The velocity along z.
   * @returns The `SET_POSITION_TARGET_LOCAL_NED` frame.
   */
  localVelocity(
    header: MavlinkHeader,
    timeBootMs: number,
    coordinateFrame: number,
    targetSystem: number,
    targetComponent: number,
    vx: number,
    vy: number,
    vz: number,
  ): MavlinkFrame {
    return mavlinkOffboardLocalVelocity(
      header,
      timeBootMs,
      coordinateFrame,
      targetSystem,
      targetComponent,
      vx,
      vy,
      vz,
    )
  },

  /**
   * Builds a global-frame position setpoint.
   *
   * @param header - The addressing fields to stamp on the frame.
   * @param timeBootMs - The sender's boot timestamp, in milliseconds.
   * @param coordinateFrame - The `MAV_FRAME` of the setpoint.
   * @param targetSystem - The target system id.
   * @param targetComponent - The target component id.
   * @param latInt - The latitude, in degrees times ten million.
   * @param lonInt - The longitude, in degrees times ten million.
   * @param alt - The altitude, in metres.
   * @returns The `SET_POSITION_TARGET_GLOBAL_INT` frame.
   */
  globalPosition(
    header: MavlinkHeader,
    timeBootMs: number,
    coordinateFrame: number,
    targetSystem: number,
    targetComponent: number,
    latInt: number,
    lonInt: number,
    alt: number,
  ): MavlinkFrame {
    return mavlinkOffboardGlobalPosition(
      header,
      timeBootMs,
      coordinateFrame,
      targetSystem,
      targetComponent,
      latInt,
      lonInt,
      alt,
    )
  },
} as const
