/**
 * Ergonomic facade over the generated ladder binding.
 *
 * A ladder is the answer to a node with more than one way to reach the network
 * and no single one that always works: rungs are tried in the order they were
 * added, cheapest first, and a message no rung accepts goes into a buffer
 * rather than being lost.
 *
 * The delivery outcome is re-exported as a runtime {@link Delivery} object,
 * because the generated enum is types-only.
 *
 * @packageDocumentation
 */

import type { Delivery as DeliveryName } from '@pamoja/native'

export { Ladder } from '@pamoja/native'

/**
 * What became of a message handed to a ladder.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const Delivery = {
  /** A rung took the message and it is on its way. */
  Sent: 'Sent' as DeliveryName,
  /** No rung would take it, so it is in the buffer awaiting a flush. */
  Buffered: 'Buffered' as DeliveryName,
} as const

/** One of the {@link Delivery} choices. */
export type Delivery = DeliveryName
