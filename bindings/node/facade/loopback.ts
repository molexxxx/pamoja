/**
 * Ergonomic facade over the generated loopback binding.
 *
 * An in-process broker: publish on one link, receive on another, with no broker
 * process, no network, and no hardware. It is what makes a message flow
 * testable from a unit test rather than only from a deployment.
 *
 * @packageDocumentation
 */

export { LoopbackBroker, LoopbackTransport } from '../index'
export type { TransportMessage } from '../index'
