/**
 * Ergonomic facade over the generated mesh binding.
 *
 * When the fixed infrastructure is gone or was never there, devices carry each
 * other's traffic: every node relays what it hears, so a message crosses an area
 * no single node can reach. This is the packet half of that, addressing and
 * integrity over radios that give you neither.
 *
 * @packageDocumentation
 */

import {
  MESH_BROADCAST,
  MESH_DEFAULT_HOP_LIMIT,
  MESH_MAX_FRAME,
  MESH_MAX_PAYLOAD,
  MESH_SEEN_DEFAULT_CAPACITY,
  type MeshFrame,
  SeenPackets,
  meshBroadcastFrame,
  meshCrc16,
  meshFrame,
  meshParseFrame,
  meshRelayed,
} from '@pamoja/native'

export { type MeshFrame, SeenPackets }

/** The destination address that means every node. */
export const BROADCAST = MESH_BROADCAST

/** The hop limit a frame starts with unless one is given. */
export const DEFAULT_HOP_LIMIT = MESH_DEFAULT_HOP_LIMIT

/** The largest payload a single frame can carry, in bytes. */
export const MAX_PAYLOAD = MESH_MAX_PAYLOAD

/** The largest frame, in bytes, including its header and checksum. */
export const MAX_FRAME = MESH_MAX_FRAME

/** A duplicate-cache size for a caller with no reason to choose one. */
export const SEEN_DEFAULT_CAPACITY = MESH_SEEN_DEFAULT_CAPACITY

/**
 * Builds a frame addressed to one node.
 *
 * @param src - The address of this node.
 * @param dst - The address the frame is for, or {@link BROADCAST}.
 * @param id - The sequence number identifying this packet from this source.
 * @param payload - The bytes to carry.
 * @param hopLimit - How many relays the frame may take, defaulting to
 *   {@link DEFAULT_HOP_LIMIT}.
 * @returns The frame, with the bytes to transmit on its `bytes` field.
 * @throws If the payload is larger than {@link MAX_PAYLOAD}.
 */
export function frame(
  src: number,
  dst: number,
  id: number,
  payload: Uint8Array,
  hopLimit?: number,
): MeshFrame {
  return meshFrame(src, dst, id, Buffer.from(payload), hopLimit)
}

/**
 * Builds a frame addressed to every node.
 *
 * @param src - The address of this node.
 * @param id - The sequence number identifying this packet from this source.
 * @param payload - The bytes to carry.
 * @param hopLimit - How many relays the frame may take, defaulting to
 *   {@link DEFAULT_HOP_LIMIT}.
 * @returns The frame, with the bytes to transmit on its `bytes` field.
 * @throws If the payload is larger than {@link MAX_PAYLOAD}.
 */
export function broadcast(
  src: number,
  id: number,
  payload: Uint8Array,
  hopLimit?: number,
): MeshFrame {
  return meshBroadcastFrame(src, id, Buffer.from(payload), hopLimit)
}

/**
 * Parses a frame received off a radio.
 *
 * @param bytes - The frame exactly as it arrived.
 * @returns The parsed frame.
 * @throws If the frame is truncated, of an unknown version, or fails its
 *   checksum, which is what a noisy radio produces.
 */
export function parse(bytes: Uint8Array): MeshFrame {
  return meshParseFrame(Buffer.from(bytes))
}

/**
 * Returns the same frame with one hop spent, ready to forward.
 *
 * @param bytes - The frame exactly as it arrived.
 * @returns The frame to forward, or `null` once its hops have run out, which is
 *   what stops a flood from circulating forever.
 * @throws If the frame cannot be parsed.
 */
export function relayed(bytes: Uint8Array): MeshFrame | null {
  return meshRelayed(Buffer.from(bytes))
}

/**
 * Computes the CRC-16 a frame carries.
 *
 * @param data - The bytes the checksum covers.
 * @returns The checksum.
 */
export function crc16(data: Uint8Array): number {
  return meshCrc16(Buffer.from(data))
}
