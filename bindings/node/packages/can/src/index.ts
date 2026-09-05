/**
 * Ergonomic facade over the generated CAN binding.
 *
 * CAN is how the moving parts of a machine talk to each other: motor controllers,
 * servos, battery management, and the engines and farm equipment that speak J1939
 * on top of it. This is the identifier and payload layer; the controller hardware
 * handles the wire itself.
 *
 * @packageDocumentation
 */

import {
  type CanFrame,
  J1939_BROADCAST_ADDRESS,
  J1939_NOT_AVAILABLE,
  J1939_PRIORITY_CONTROL,
  J1939_PRIORITY_DEFAULT,
  J1939_PRIORITY_LOWEST,
  Signals,
  canDlcToLen,
  canFdFrame,
  canFrame,
  canLenToDlc,
  canRemoteFrame,
  j1939Broadcast,
  j1939Compose,
  j1939Decode,
} from '@pamoja/native'

export { type CanFrame, Signals }

/** The byte a J1939 sender writes for a signal it is not reporting. */
export const NOT_AVAILABLE = J1939_NOT_AVAILABLE

/** The destination address every node on the bus reads. */
export const BROADCAST_ADDRESS = J1939_BROADCAST_ADDRESS

/** The priorities J1939 publishes, so a caller does not write the number out. */
export const priority = {
  /** Ahead of ordinary traffic, for a message that controls something. */
  control: J1939_PRIORITY_CONTROL,
  /** What ordinary traffic uses. */
  default: J1939_PRIORITY_DEFAULT,
  /** Yields to everything else on the bus. */
  lowest: J1939_PRIORITY_LOWEST,
} as const

/**
 * Builds a J1939 payload with every signal marked not available.
 *
 * @returns Eight bytes a controller writes only its own signals into.
 */
export function signals(): Signals {
  return new Signals()
}

/**
 * Reads the eight data bytes of a frame that arrived off the bus.
 *
 * @param data - The frame's payload.
 * @returns The payload, ready for its signals to be read out.
 * @throws If the payload is not exactly eight bytes.
 */
export function signalsFrom(data: Uint8Array): Signals {
  return Signals.fromBytes(Buffer.from(data))
}

/** The fields J1939 packs into an extended CAN identifier. */
export interface J1939Message {
  /** The parameter group number, which names what the message carries. */
  pgn: number
  /** The message priority, 0 (highest) to 7. */
  priority: number
  /** The source address: the node that sent the message. */
  source: number
  /** The PDU format byte of the parameter group. */
  pduFormat: number
  /**
   * The destination address for an addressed (PDU1) message, or `null` for a
   * broadcast (PDU2) one.
   */
  destination: number | null
  /** Whether the message is a broadcast. */
  broadcast: boolean
}

/**
 * Builds a classic CAN 2.0 frame.
 *
 * @param id - The arbitration identifier, masked to the width `extended` selects.
 * @param data - The payload, at most eight bytes.
 * @param extended - Whether the identifier is a 29-bit extended one.
 * @returns The frame.
 * @throws If the payload is longer than a classic frame carries.
 */
export function frame(id: number, data: Uint8Array, extended = false): CanFrame {
  return canFrame(id, extended, Buffer.from(data))
}

/**
 * Builds a CAN-FD frame, which carries up to 64 bytes.
 *
 * @param id - The arbitration identifier.
 * @param data - The payload, at one of the discrete CAN-FD lengths: 0 to 8, then
 * 12, 16, 20, 24, 32, 48, or 64 bytes.
 * @param extended - Whether the identifier is a 29-bit extended one.
 * @returns The frame.
 * @throws If the payload length is not one CAN-FD can carry.
 */
export function fdFrame(id: number, data: Uint8Array, extended = false): CanFrame {
  return canFdFrame(id, extended, Buffer.from(data))
}

/**
 * Builds a remote transmission request, which asks another node to send.
 *
 * @param id - The arbitration identifier.
 * @param len - The data length being requested, clamped to eight bytes.
 * @param extended - Whether the identifier is a 29-bit extended one.
 * @returns The frame, which carries no payload of its own.
 */
export function remoteFrame(id: number, len: number, extended = false): CanFrame {
  return canRemoteFrame(id, extended, len)
}

/**
 * Returns the data length code that encodes a payload length.
 *
 * @param len - The payload length in bytes.
 * @returns The code, rounding up to the next length CAN-FD can carry.
 */
export function lenToDlc(len: number): number {
  return canLenToDlc(len)
}

/**
 * Returns the payload length a data length code encodes.
 *
 * @param dlc - The data length code.
 * @returns The length in bytes.
 */
export function dlcToLen(dlc: number): number {
  return canDlcToLen(dlc)
}

/**
 * Decodes the J1939 fields out of an extended CAN identifier.
 *
 * @param id - The identifier as it arrived.
 * @param extended - Whether it is a 29-bit extended identifier.
 * @returns The decoded message, or `null` for a standard identifier, which J1939
 * does not use.
 */
export function decodeJ1939(id: number, extended = true): J1939Message | null {
  const message = j1939Decode(id, extended)
  if (message === null || message === undefined) return null
  // The generated object leaves an absent destination undefined; null says the
  // same thing in the shape the other bindings use.
  return {
    pgn: message.pgn,
    priority: message.priority,
    source: message.source,
    pduFormat: message.pduFormat,
    destination: message.destination ?? null,
    broadcast: message.broadcast,
  }
}

/**
 * Composes the extended CAN identifier a set of J1939 fields describes.
 *
 * @param priority - The message priority, 0 (highest) to 7.
 * @param pgn - The parameter group number.
 * @param source - The address of the sending node.
 * @param destination - The destination address, used only for an addressed (PDU1)
 * parameter group and ignored for a broadcast (PDU2) one.
 * @returns The 29-bit identifier.
 */
export function composeJ1939(
  priority: number,
  pgn: number,
  source: number,
  destination = 0,
): number {
  return j1939Compose(priority, pgn, source, destination)
}

/**
 * Composes the identifier of a J1939 broadcast, which every node on the bus reads.
 *
 * Most parameter groups are broadcast, so this is the common case; it saves a
 * caller knowing that a broadcast is addressed to `0xFF`.
 *
 * @param priority - The message priority, 0 (highest) to 7.
 * @param pgn - The parameter group number.
 * @param source - The address of the sending node.
 * @returns The 29-bit identifier.
 */
export function broadcastJ1939(priority: number, pgn: number, source: number): number {
  return j1939Broadcast(priority, pgn, source)
}
