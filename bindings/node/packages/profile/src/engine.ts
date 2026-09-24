/**
 * A rule file run off a link: every reading that arrives on a watched topic is judged, and
 * the drives and publishes a set or cleared condition calls for are carried out.
 *
 * @packageDocumentation
 */

import { RuleEvaluator } from '@pamoja/native'
import type { RuleFired } from '@pamoja/native'

import type { Link } from './node'

/** A message a link hands over. */
export interface LinkMessage {
  /** The topic it arrived on. */
  topic: string
  /** The payload as it arrived. */
  payload: Buffer
  /** The payload as text, when it is text. */
  text?: string
  /** The payload as a number, when its text is one. */
  number?: number
}

/**
 * A link a rule engine listens on as well as publishes over: a `LoopbackTransport`, an
 * `MqttClient`, a `CoapClient`, or anything else with the same three calls. Connect it
 * first.
 */
export interface ReceivingLink extends Link {
  /** Subscribes to one topic. */
  subscribe(topic: string): Promise<unknown>
  /** Waits for the next message, or `null` when none arrives in time. */
  recv(timeoutMs?: number): Promise<LinkMessage | null>
}

/** The outputs a rule engine switches and how it reads a message. */
export interface RuleEngineOptions {
  /** The outputs the rules drive, each under the name the rule file gives it. */
  actuators?: Record<string, (on: boolean) => unknown>
  /** Reads a message's payload as a reading; its text as a number unless given. */
  decode?: (message: LinkMessage) => number
}

/** How a rule engine runs until it is told to stop. */
export interface EngineRunOptions {
  /** Stops the loop; a message under way is finished first. */
  signal?: AbortSignal
  /** Hears each message that set or cleared a rule, with what fired. */
  onFired?: (fired: RuleFired[]) => unknown
  /**
   * Hears a message that could not be judged or an action that failed. The loop carries
   * on with the next message; without one, the failure ends the loop and `run` rejects
   * with it.
   */
  onError?: (error: unknown) => unknown
  /** How many messages to handle before returning; the loop runs until stopped unless given. */
  messages?: number
  /** How long each wait for a message lasts before the signal is checked again. */
  pollMs?: number
}

/**
 * Runs a rule file off a link. `listen` checks that every output a rule drives was given
 * and subscribes to every watched topic; each `step` handles one message, switching the
 * outputs by name and publishing over the same link; `run` repeats it until stopped.
 */
export class RuleEngine {
  /** The rules, deciding each reading as the engine hands it over. */
  readonly evaluator: RuleEvaluator
  readonly #link: ReceivingLink
  readonly #actuators: Record<string, (on: boolean) => unknown>
  readonly #decode: (message: LinkMessage) => number

  /**
   * Assembles an engine.
   *
   * @param rules - the rule file's text, or an evaluator already loaded from it.
   * @param link - the link readings arrive on and actions publish over.
   * @param options - the outputs the rules drive, and how a message is read.
   * @throws when the rule file is refused, with the rule and the reason.
   */
  constructor(rules: string | RuleEvaluator, link: ReceivingLink, options: RuleEngineOptions = {}) {
    this.evaluator = typeof rules === 'string' ? RuleEvaluator.fromJson(rules) : rules
    this.#link = link
    this.#actuators = options.actuators ?? {}
    this.#decode = options.decode ?? ((message) => message.number ?? Number(message.text))
  }

  /** The topics the rules watch, each once, in name order. */
  get topics(): string[] {
    return this.evaluator.topics
  }

  /** The outputs the rules drive, each once, in name order. */
  get actuators(): string[] {
    return this.evaluator.actuators
  }

  /**
   * Whether a rule's condition holds.
   *
   * @param rule - the rule's name.
   * @returns the state, or `null` for a name no rule has.
   */
  isSet(rule: string): boolean | null {
    return this.evaluator.isSet(rule)
  }

  /**
   * Checks the engine was given every output a rule drives, and subscribes to every
   * watched topic.
   *
   * @throws when a rule drives an output the engine was not given, naming it.
   */
  async listen(): Promise<void> {
    const missing = this.actuators.filter((name) => this.#actuators[name] === undefined)
    if (missing.length > 0) {
      throw new Error(
        `a rule drives \`${missing.join('`, `')}\`, which the engine was not given under \`actuators\``,
      )
    }
    for (const topic of this.topics) {
      await this.#link.subscribe(topic)
    }
  }

  /**
   * Waits for one message and runs every rule that watches its topic.
   *
   * @param timeoutMs - how long to wait; for good unless given.
   * @returns what fired, which is empty when the message set or cleared nothing, or `null`
   *   when no message arrived in time or the link has ended.
   * @throws when a reading on a watched topic does not decode to a finite number, or an
   *   action fails.
   */
  async step(timeoutMs?: number): Promise<RuleFired[] | null> {
    const message = await this.#next(timeoutMs)
    return typeof message === 'string' ? null : this.#judge(message)
  }

  /** Receives the next message, saying whether the wait ran out or the link ended instead. */
  async #next(timeoutMs?: number): Promise<LinkMessage | 'timeout' | 'ended'> {
    try {
      return (await this.#link.recv(timeoutMs)) ?? 'ended'
    } catch (error) {
      if (timeoutMs !== undefined && String((error as Error)?.message).startsWith('no message arrived within')) {
        return 'timeout'
      }
      throw error
    }
  }

  /** Judges one message and carries out what the rules that fired call for. */
  async #judge(message: LinkMessage): Promise<RuleFired[]> {
    if (!this.evaluator.watches(message.topic)) {
      return []
    }
    const fired = this.evaluator.evaluate(message.topic, this.#decode(message))
    for (const one of fired) {
      for (const action of one.actions) {
        if (action.kind === 'drive') {
          await this.#actuators[action.actuator ?? '']?.(action.on === true)
        } else {
          await this.#link.send(action.topic ?? '', action.payload ?? '')
        }
      }
    }
    return fired
  }

  /**
   * Handles messages until stopped.
   *
   * @param options - when to stop, and what to hear.
   * @returns once the signal aborts, the messages run out, or the link ends.
   * @throws the failure of a message when no `onError` hears it.
   */
  async run(options: EngineRunOptions = {}): Promise<void> {
    const poll = options.pollMs ?? 500
    let handled = 0
    while (options.messages === undefined || handled < options.messages) {
      if (options.signal?.aborted) {
        return
      }
      try {
        const message = await this.#next(poll)
        if (message === 'ended') {
          return
        }
        if (message === 'timeout') {
          continue
        }
        handled += 1
        const fired = await this.#judge(message)
        if (fired.length > 0) {
          await options.onFired?.(fired)
        }
      } catch (error) {
        handled += 1
        if (options.onError === undefined) {
          throw error
        }
        await options.onError(error)
      }
    }
  }
}
