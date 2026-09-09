/**
 * Ergonomic facade over the generated transport binding.
 *
 * A ladder rung, a fault injector, and a degraded link all take some transport.
 * JavaScript has no way to say "any transport", so one class holds whichever
 * kind was built and dispatches to it.
 *
 * Composing consumes a transport, because the thing it is composed into owns it
 * from then on. A consumed transport is emptied rather than left aliasing what
 * now belongs to a ladder, so using one twice throws.
 *
 * @packageDocumentation
 */

export { Transport } from '@pamoja/native'
export type { TransportMessage } from '@pamoja/native'

import type { TransportMessage } from '@pamoja/native'

/**
 * The operations a link written in JavaScript supplies to stand as a transport.
 *
 * Implement this for a link pamoja does not ship, a vendor SDK, a proprietary
 * radio, or a cloud client, and hand it to `Transport.fromHandlers`. The result
 * composes like any transport: as a ladder rung, under a fault injector, or
 * driven directly. Each method may return a promise or a plain value, and is
 * called on the object, so a class instance works as it is.
 */
export interface TransportHandlers {
  /** Establishes the link. */
  connect(): void | Promise<void>
  /** Publishes a payload to a topic. */
  send(topic: string, payload: Buffer): void | Promise<void>
  /** Subscribes to a topic filter, in the syntax the link understands. */
  subscribe(topic: string): void | Promise<void>
  /**
   * Waits for the next message on a subscribed topic, or resolves to `null`
   * once the link has ended. Present, it is called again as soon as it settles,
   * from the moment the transport connects, and what it delivers is queued for
   * the receiving side. Absent, the link only sends and a ladder never listens
   * on it.
   */
  recv?(): TransportMessage | null | undefined | Promise<TransportMessage | null | undefined>
}
