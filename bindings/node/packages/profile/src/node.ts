/**
 * A profile run as a node: the read, decide, act, and publish loop, and the registry that
 * finds the code a custom control kind names.
 *
 * @packageDocumentation
 */

import type { PowerMode, PowerPlan, Profile, Reaction } from '@pamoja/native'

import { unresolved } from './nearest'

/**
 * A link a node publishes its readings over: a `LoopbackTransport`, an `MqttClient`, a
 * `CoapClient`, a `Ladder`, or anything else with the same `send`. Connect it first.
 */
export interface Link {
  /** Publishes one message to a topic. */
  send(topic: string, payload: Buffer | string): Promise<unknown>
}

/**
 * What decides each reading: a profile's built-in `Controller`, or a policy of the
 * program's own. A custom policy raises its own conditions as an alert of kind `Custom`
 * with a `code` and a `value`.
 */
export interface Policy {
  /** Decides what one reading calls for. */
  evaluate(reading: number): Reaction
}

/** Builds a custom kind's policy from the parameters its manifest carries beside it. */
export type PolicyFactory = (params: Record<string, number | boolean | string>) => Policy

/**
 * What resolves a profile's control kind to the code that decides it: a built-in kind to
 * its `Controller`, and a kind the library never shipped to the policy the factory
 * registered under its name builds. One program then runs any manifest its registry
 * covers.
 */
export class PolicyRegistry {
  readonly #factories = new Map<string, PolicyFactory>()

  /**
   * Registers the factory for a custom kind, replacing one of the same name.
   *
   * @param kind - the kind as a manifest names it, such as `frost_guard`.
   * @param factory - builds the policy from the parameters beside the kind.
   * @returns the registry, for chaining.
   */
  register(kind: string, factory: PolicyFactory): this {
    this.#factories.set(kind, factory)
    return this
  }

  /** The custom kinds registered, in name order. */
  get kinds(): string[] {
    return [...this.#factories.keys()].sort()
  }

  /**
   * Resolves a profile's control kind to the policy that decides it.
   *
   * @param profile - the profile whose kind is resolved.
   * @returns a fresh policy, ready to evaluate readings.
   * @throws when the kind is custom and no factory is registered for it, naming the kind
   *   and the one it was probably meant to be, or whatever the factory throws.
   */
  resolve(profile: Profile): Policy {
    const control = profile.control
    if (control.kind !== 'Custom') {
      return profile.controller()
    }
    const kind = control.customKind ?? ''
    const factory = this.#factories.get(kind)
    if (factory === undefined) {
      throw new Error(unresolved(kind, this.kinds))
    }
    return factory(control.params ?? {})
  }
}

/** One reading a node took, and what its policy decided about it. */
export interface Tick {
  /** The reading, in the unit the profile reads. */
  reading: number
  /** The output setting and the alert the policy decided on. */
  reaction: Reaction
}

/** The parts a node runs a profile with. */
export interface NodeOptions {
  /** The profile the node runs. */
  profile: Profile
  /** Takes one reading, in the unit the profile reads. */
  read: () => number | Promise<number>
  /** The link each reading is published over, connected before the node runs. */
  link: Link
  /** Switches the output a profile drives; required for a setpoint profile. */
  drive?: (on: boolean) => unknown
  /**
   * What decides each reading: the profile's own controller unless given, a policy of the
   * program's own, or a registry that resolves the profile's control kind.
   */
  policy?: Policy | PolicyRegistry
  /** Writes a reading as the payload published; the number as JSON text unless given. */
  encode?: (reading: number) => Buffer | string
}

/** How a node runs until it is told to stop. */
export interface RunOptions {
  /** Stops the loop; the tick under way finishes first. */
  signal?: AbortSignal
  /**
   * Reads the battery before each wait, as a charge from 0 to 1 or a charge and whether
   * the panel is charging; a node without one samples at the active cadence.
   */
  battery?: () =>
    | number
    | { charge: number; charging: boolean }
    | Promise<number | { charge: number; charging: boolean }>
  /** Hears each tick, such as to log it or to act on an alert. */
  onTick?: (tick: Tick) => unknown
  /**
   * Hears a tick that failed. The loop carries on at the same cadence; without one, the
   * failure ends the loop and `run` rejects with it.
   */
  onError?: (error: unknown) => unknown
  /** How many ticks to run before returning; the loop runs until stopped unless given. */
  ticks?: number
  /** Waits between ticks; a timer unless given, which a test replaces to run at once. */
  wait?: (ms: number, signal?: AbortSignal) => Promise<void>
}

/**
 * A profile assembled around the parts that make it run: each tick reads, lets the
 * profile's policy decide, switches the output when the policy calls for it, and publishes
 * the reading to the profile's topic. `run` repeats that at the cadence the profile's power
 * schedule sets for the battery's charge.
 */
export class Node {
  /** The profile the node runs. */
  readonly profile: Profile
  readonly #read: () => number | Promise<number>
  readonly #link: Link
  readonly #drive?: (on: boolean) => unknown
  readonly #policy: Policy
  readonly #encode: (reading: number) => Buffer | string
  readonly #plan: PowerPlan
  #mode: PowerMode | null = null

  /**
   * Assembles a node.
   *
   * @param options - the profile, the reading, the link, and the output.
   * @throws when the profile drives an output and no `drive` is given, or when its control
   *   kind is custom and no policy or registry decides it.
   */
  constructor(options: NodeOptions) {
    this.profile = options.profile
    this.#read = options.read
    this.#link = options.link
    this.#drive = options.drive
    this.#encode = options.encode ?? ((reading) => JSON.stringify(reading))
    const policy = options.policy
    this.#policy =
      policy instanceof PolicyRegistry
        ? policy.resolve(this.profile)
        : (policy ?? this.profile.controller())
    if (this.profile.control.kind === 'Setpoint' && this.#drive === undefined) {
      throw new Error(
        `the profile \`${this.profile.name}\` switches an output, so the node needs \`drive\``,
      )
    }
    this.#plan = this.profile.powerPlan()
  }

  /** The power mode the last `schedule` chose, or `null` before the first. */
  get powerMode(): PowerMode | null {
    return this.#mode
  }

  /**
   * Runs one read, decide, act, and publish cycle.
   *
   * @returns the reading and what the policy decided about it.
   * @throws what the reading, the output, or the link throws.
   */
  async tick(): Promise<Tick> {
    const reading = await this.#read()
    const reaction = this.#policy.evaluate(reading)
    if (reaction.actuator != null && this.#drive !== undefined) {
      await this.#drive(reaction.actuator)
    }
    await this.#link.send(this.profile.topic, this.#encode(reading))
    return { reading, reaction }
  }

  /**
   * Says what power mode the battery's charge puts the node in and how long to wait
   * before the next tick. The node remembers the mode it chose, so a charge hovering at a
   * threshold keeps the slower cadence until it clears the schedule's hysteresis.
   *
   * @param charge - the state of charge, from 0 to 1.
   * @param charging - whether the panel is delivering charge.
   * @returns the mode, and the wait in milliseconds.
   */
  schedule(charge: number, charging = false): { mode: PowerMode; waitMs: number } {
    const mode =
      this.#mode === null
        ? this.#plan.modeWhileCharging(charge, charging)
        : this.#plan.nextModeWhileCharging(this.#mode, charge, charging)
    this.#mode = mode
    return { mode, waitMs: this.#plan.intervalForUs(mode) / 1000 }
  }

  /**
   * Ticks, then waits the interval the battery's charge calls for, until stopped.
   *
   * @param options - when to stop, how to read the battery, and what to hear.
   * @returns once the signal aborts or the ticks run out.
   * @throws the failure of a tick when no `onError` hears it.
   */
  async run(options: RunOptions = {}): Promise<void> {
    const wait = options.wait ?? sleep
    for (let count = 0; options.ticks === undefined || count < options.ticks; count += 1) {
      if (options.signal?.aborted) {
        return
      }
      try {
        const tick = await this.tick()
        await options.onTick?.(tick)
      } catch (error) {
        if (options.onError === undefined) {
          throw error
        }
        await options.onError(error)
      }
      const battery = options.battery === undefined ? 1 : await options.battery()
      const { charge, charging } =
        typeof battery === 'number' ? { charge: battery, charging: false } : battery
      const { waitMs } = this.schedule(charge, charging)
      if (options.ticks === undefined || count + 1 < options.ticks) {
        await wait(waitMs, options.signal)
      }
    }
  }
}

/** Waits a number of milliseconds, returning early when the signal aborts. */
export function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal?.aborted) {
      resolve()
      return
    }
    const timer = setTimeout(done, ms)
    signal?.addEventListener('abort', done, { once: true })
    function done() {
      clearTimeout(timer)
      signal?.removeEventListener('abort', done)
      resolve()
    }
  })
}
