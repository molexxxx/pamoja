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

export { CoapClient } from '../index'
export type { CoapClientOptions, TransportMessage } from '../index'

/**
 * Whether a request is acknowledged and retried.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const Reliability = {
  /** Fire and forget: the request is sent once and not acknowledged. */
  NonConfirmable: 'NonConfirmable',
  /** The request is acknowledged, and retransmitted until an ACK arrives. */
  Confirmable: 'Confirmable',
} as const

/** One of the {@link Reliability} choices. */
export type Reliability = (typeof Reliability)[keyof typeof Reliability]
