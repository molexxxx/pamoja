/**
 * Ergonomic facade over the generated MQTT binding.
 *
 * Adds rejected promises for errors, an async iterator over incoming messages,
 * and string-or-bytes payloads, without adding behavior; all real work happens in
 * the native core reached through the generated contract.
 *
 * @packageDocumentation
 */

import type { Qos as QosName } from '@pamoja/native'

import {
  MqttClient as NativeMqttClient,
  type MqttClientOptions as NativeMqttClientOptions,
  type MqttMessage,
} from '@pamoja/native'

export type { MqttMessage }

/**
 * MQTT delivery guarantee, mirroring the protocol's quality-of-service levels.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`Qos.AtLeastOnce`) and a type annotation.
 */
export const Qos = {
  /** Fire and forget; the broker does not acknowledge delivery. */
  AtMostOnce: 'AtMostOnce' as QosName,
  /** Delivered at least once and acknowledged. */
  AtLeastOnce: 'AtLeastOnce' as QosName,
  /** Delivered exactly once via a four-step handshake. */
  ExactlyOnce: 'ExactlyOnce' as QosName,
} as const

/** One of the {@link Qos} levels. */
export type Qos = QosName

/** Connection settings for an {@link MqttClient}. */
export interface MqttClientOptions {
  /** The MQTT client identifier presented to the broker. */
  clientId: string
  /** The broker hostname or IP address. */
  host: string
  /** The broker TCP port, conventionally 1883 for plaintext MQTT. */
  port: number
  /** Keep-alive interval in seconds. Defaults to 30 when omitted. */
  keepAliveSecs?: number
  /** Bound on outstanding client requests. Defaults to 64 when omitted. */
  capacity?: number
  /** Default quality of service. Defaults to `AtLeastOnce` when omitted. */
  qos?: Qos
}

/**
 * An MQTT client transport.
 *
 * Construct it with broker settings, {@link connect}, then {@link publish},
 * {@link subscribe}, and read inbound messages with {@link recv} or by iterating
 * the client with `for await`.
 *
 * @example
 * ```ts
 * const client = new MqttClient({ clientId: 'sensor-1', host: 'localhost', port: 1883 })
 * await client.connect()
 * await client.subscribe('sensors/+/temperature')
 * await client.publish('sensors/1/temperature', '21.5')
 * for await (const message of client) {
 *   console.log(message.topic, message.payload.toString())
 * }
 * ```
 */
export class MqttClient {
  readonly #native: NativeMqttClient

  /**
   * Creates a disconnected client from the given options.
   *
   * @param options - The broker connection settings.
   */
  constructor(options: MqttClientOptions) {
    this.#native = new NativeMqttClient(options as unknown as NativeMqttClientOptions)
  }

  /**
   * Connects to the broker and starts the background event loop.
   *
   * @returns A promise that resolves once connected and rejects on failure.
   */
  connect(): Promise<void> {
    return this.#native.connect()
  }

  /**
   * Publishes a payload to a topic.
   *
   * @param topic - The destination topic.
   * @param payload - The message body; strings are encoded as UTF-8.
   * @returns A promise that resolves once the payload is handed to the transport.
   */
  publish(topic: string, payload: string | Uint8Array): Promise<void> {
    const bytes = typeof payload === 'string' ? Buffer.from(payload, 'utf8') : Buffer.from(payload)
    return this.#native.publish(topic, bytes)
  }

  /**
   * Subscribes to a topic filter.
   *
   * @param topic - The topic or wildcard filter to subscribe to.
   * @returns A promise that resolves once the subscription is registered.
   */
  subscribe(topic: string): Promise<void> {
    return this.#native.subscribe(topic)
  }

  /**
   * Awaits the next message from any subscribed topic.
   *
   * @returns The next message, or `null` once the connection has ended.
   */
  recv(): Promise<MqttMessage | null> {
    return this.#native.recv()
  }

  /**
   * Reports whether the client currently holds an active connection.
   *
   * @returns A promise resolving to the connection state.
   */
  isConnected(): Promise<boolean> {
    return this.#native.isConnected()
  }

  /**
   * Closes the connection and stops the background event loop.
   *
   * @returns A promise that resolves once the client has disconnected.
   */
  disconnect(): Promise<void> {
    return this.#native.disconnect()
  }

  /**
   * Yields messages from subscribed topics until the connection ends.
   *
   * @returns An async generator over incoming messages.
   */
  async *messages(): AsyncGenerator<MqttMessage, void, unknown> {
    for (;;) {
      const message = await this.#native.recv()
      if (message === null) {
        return
      }
      yield message
    }
  }

  /** Iterates incoming messages, so a client can be used with `for await`. */
  [Symbol.asyncIterator](): AsyncGenerator<MqttMessage, void, unknown> {
    return this.messages()
  }
}
