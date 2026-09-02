/**
 * Ergonomic facade over the generated routing binding.
 *
 * Flooding always works but costs every node airtime and power on every packet.
 * Once a mesh has settled, most traffic goes to a few known places, and a node
 * that remembers the way can forward to one neighbour instead of shouting at the
 * whole network. Routing is that optimisation, and it falls back to flooding
 * rather than failing whenever it does not know the way.
 *
 * The routing action is re-exported as a runtime {@link ForwardAction} object,
 * because the generated enum is types-only.
 *
 * @packageDocumentation
 */

import { ROUTING_TABLE_CAPACITY, Router as NativeRouter } from '../index'

/** The number of routes a routing table holds. */
export const TABLE_CAPACITY = ROUTING_TABLE_CAPACITY

/**
 * What to do with a packet bound for a given node.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`ForwardAction.Relay`) and a type annotation.
 */
export const ForwardAction = {
  /** The packet is for this node; hand it to the application. */
  Deliver: 'Deliver',
  /** A route is known; unicast the packet to the next hop reported alongside. */
  Relay: 'Relay',
  /** No route is known; fall back to flooding the packet. */
  Flood: 'Flood',
} as const

/** One of the {@link ForwardAction} choices. */
export type ForwardAction = (typeof ForwardAction)[keyof typeof ForwardAction]

/** A routing decision, and the neighbour it names when there is one. */
export interface ForwardDecision {
  /** What to do with the packet. */
  action: ForwardAction
  /** The neighbour to unicast to, or `null` unless the action is `Relay`. */
  nextHop: number | null
}

/** A learned way to reach one node. */
export interface Route {
  /** The node this route reaches. */
  dst: number
  /** The neighbour to send a packet to on the way there. */
  nextHop: number
  /** What the route costs, usually in hops. */
  cost: number
}

/**
 * One node routing table, learned from the traffic the node hears.
 *
 * The core table is generic over its size, which cannot cross the native
 * boundary, so this is built at one documented size: {@link TABLE_CAPACITY}.
 */
export class Router {
  readonly #inner: NativeRouter

  /**
   * Creates an empty routing table for a node.
   *
   * @param address - The address of this node, which is what a routing decision
   *   recognises as a local delivery.
   */
  constructor(address: number) {
    this.#inner = new NativeRouter(address)
  }

  /** The address this router answers for. */
  get address(): number {
    return this.#inner.address
  }

  /** How many routes the table currently holds. */
  get size(): number {
    return this.#inner.len
  }

  /** Whether the table has learned nothing yet. */
  get isEmpty(): boolean {
    return this.#inner.isEmpty
  }

  /** How many routes the table can hold. */
  get capacity(): number {
    return this.#inner.capacity
  }

  /**
   * Learns a route from a packet that arrived.
   *
   * @param origin - The node the packet came from.
   * @param via - The neighbour it arrived through.
   * @param cost - What that path costs, usually a hop count.
   * @returns Whether the table changed. It keeps the cheapest way it knows to
   *   each node, and when full gives up the most expensive route to make room.
   */
  observe(origin: number, via: number, cost: number): boolean {
    return this.#inner.observe(origin, via, cost)
  }

  /**
   * Returns the neighbour on the way to a node.
   *
   * @param dst - The node to reach.
   * @returns The next hop, or `null` when no route is known.
   */
  nextHop(dst: number): number | null {
    return this.#inner.nextHop(dst) ?? null
  }

  /**
   * Returns what the known route to a node costs.
   *
   * @param dst - The node to reach.
   * @returns The cost, or `null` when no route is known.
   */
  cost(dst: number): number | null {
    return this.#inner.cost(dst) ?? null
  }

  /**
   * Returns the whole route to a node.
   *
   * @param dst - The node to reach.
   * @returns The route, or `null` when no route is known.
   */
  route(dst: number): Route | null {
    return this.#inner.route(dst) ?? null
  }

  /**
   * Decides what to do with a packet bound for a node.
   *
   * @param dst - The node the packet is addressed to.
   * @returns The decision, carrying a next hop only when it says to relay.
   */
  forward(dst: number): ForwardDecision {
    // The generated object leaves an absent next hop undefined; null says the
    // same thing in the shape the rest of this package uses.
    const decision = this.#inner.forward(dst)
    return { action: decision.action, nextHop: decision.nextHop ?? null }
  }

  /**
   * Forgets the route to a node, for example after it stops answering.
   *
   * @param dst - The node to forget.
   */
  forget(dst: number): void {
    this.#inner.forget(dst)
  }
}

/**
 * Creates an empty routing table for a node.
 *
 * @param address - The address of this node, which is what a routing decision
 *   recognises as a local delivery.
 * @returns The routing table, ready to learn from the traffic the node hears.
 */
export function router(address: number): Router {
  return new Router(address)
}
