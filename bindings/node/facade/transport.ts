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

export { Transport } from '../index'
export type { TransportMessage } from '../index'
