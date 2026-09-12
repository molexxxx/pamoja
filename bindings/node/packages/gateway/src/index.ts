/**
 * Ergonomic facade over the generated LoRaWAN gateway binding.
 *
 * A gateway hears packets from every node in range and hands them to a network server, which
 * hands back the packets to transmit. The protocol between them is a plain exchange of UDP
 * datagrams, and this speaks it from both sides: build a datagram, {@link encode} it onto a
 * socket of your own, and {@link parse} whatever arrives.
 *
 * Frequencies are in hertz, payloads are buffers rather than base64, and a reception time is
 * a count of microseconds, so nothing has to be formatted by hand.
 *
 * @packageDocumentation
 */

import type {
  GatewayCrc as NativeGatewayCrc,
  GatewayPacketKind as NativeGatewayPacketKind,
} from '@pamoja/native'

export type {
  GatewayNetworkEvent as NetworkEvent,
  GatewayNetworkWindows as NetworkWindows,
  GatewayPacket as Packet,
  GatewayRxpk as Rxpk,
  GatewaySlot as Slot,
  GatewayStat as Stat,
  GatewayTxpk as Txpk,
} from '@pamoja/native'

export {
  /**
   * The network side of one site: what a server does with what a gateway forwarded.
   *
   * It holds the devices it admits, the sessions it has granted and the counters it has
   * seen, so a packet handed to it comes back as a join it answered, a frame it decrypted,
   * or traffic belonging to a network this site never granted.
   */
  GatewayNetwork as Network,
} from '@pamoja/native'

export {
  /**
   * Writes a datagram to send over a socket.
   *
   * @example
   * ```ts
   * const datagram = encode({ kind: PacketKind.PullData, token: 0x1234, gateway: 'b827ebfffe010203' })
   * socket.send(datagram, 1700, 'router.example.org')
   * ```
   */
  gatewayEncode as encode,
  /** Reads a datagram that arrived, throwing when it is not one this protocol defines. */
  gatewayParse as parse,
  /** Returns the acknowledgment a server owes a datagram, or `null` for one that needs none. */
  gatewayAcknowledgment as acknowledgment,
} from '@pamoja/native'

/**
 * Which kind of datagram.
 *
 * Provided as a runtime object plus a matching string-union type, so it works as both a value
 * (`PacketKind.PushData`) and a type annotation.
 */
export const PacketKind = {
  /** The gateway forwarding what it heard. */
  PushData: 'PushData' as NativeGatewayPacketKind,
  /** The server acknowledging a PUSH_DATA. */
  PushAck: 'PushAck' as NativeGatewayPacketKind,
  /** The gateway holding its route open through any address translation in front of it. */
  PullData: 'PullData' as NativeGatewayPacketKind,
  /** The server sending a packet to transmit. */
  PullResp: 'PullResp' as NativeGatewayPacketKind,
  /** The server acknowledging a PULL_DATA. */
  PullAck: 'PullAck' as NativeGatewayPacketKind,
  /** The gateway reporting what became of a PULL_RESP. */
  TxAck: 'TxAck' as NativeGatewayPacketKind,
} as const

/** One of the {@link PacketKind} values. */
export type PacketKind = NativeGatewayPacketKind

/** What the CRC of a received packet said. */
export const Crc = {
  /** The CRC checked. */
  Ok: 'Ok' as NativeGatewayCrc,
  /** The CRC failed. */
  Failed: 'Failed' as NativeGatewayCrc,
  /** The packet carried no CRC. */
  Absent: 'Absent' as NativeGatewayCrc,
} as const

/** One of the {@link Crc} values. */
export type Crc = NativeGatewayCrc

/**
 * What became of a downlink the server asked for, in the protocol's own words.
 *
 * A TX_ACK carries one of these, and `NONE` is how the protocol says nothing failed.
 */
export const TxStatus = {
  /** It was scheduled. */
  None: 'NONE',
  /** It arrived too late to schedule. */
  TooLate: 'TOO_LATE',
  /** Its timestamp is too far ahead. */
  TooEarly: 'TOO_EARLY',
  /** Another packet was already scheduled then. */
  CollisionPacket: 'COLLISION_PACKET',
  /** A beacon was already scheduled then. */
  CollisionBeacon: 'COLLISION_BEACON',
  /** The radio chain cannot reach that frequency. */
  TxFreq: 'TX_FREQ',
  /** The gateway cannot transmit at that power. */
  TxPower: 'TX_POWER',
  /** A GPS timestamp was asked for while the GPS is unlocked. */
  GpsUnlocked: 'GPS_UNLOCKED',
} as const

/** One of the {@link TxStatus} values. */
export type TxStatus = (typeof TxStatus)[keyof typeof TxStatus]

/** The port a packet forwarder sends to by convention, which the protocol itself does not fix. */
export const DEFAULT_PORT = 1700
