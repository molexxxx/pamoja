/**
 * Ergonomic facade over the generated CoAP binding.
 *
 * CoAP is the transport for links where MQTT is more than the budget allows: it
 * runs over UDP, its headers are a handful of bytes, and a node can fire a
 * reading and forget it rather than holding a session open.
 *
 * The reliability choice is re-exported as a runtime {@link Reliability}
 * object, because the generated enum is types-only.
 *
 * @packageDocumentation
 */

import type { Reliability as ReliabilityName } from '@pamoja/native'

export { CoapClient } from '@pamoja/native'
export type {
  CoapClientOptions,
  Reliability as ReliabilityName,
  TransportMessage,
} from '@pamoja/native'

/**
 * Whether a request is acknowledged and retried.
 *
 * Provided as a runtime object plus a matching string-union type, so it works as
 * both a value and a type annotation. Each value carries the contract's own type,
 * so it is assignable wherever the generated binding takes the enum.
 */
export const Reliability = {
  /** Fire and forget: the request is sent once and not acknowledged. */
  NonConfirmable: 'NonConfirmable' as ReliabilityName,
  /** The request is acknowledged, and retransmitted until an ACK arrives. */
  Confirmable: 'Confirmable' as ReliabilityName,
} as const

/** One of the {@link Reliability} choices. */
export type Reliability = ReliabilityName
