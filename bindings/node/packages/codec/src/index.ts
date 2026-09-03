/**
 * Ergonomic facade over the generated codec binding.
 *
 * JavaScript already has JSON, so the facade takes and returns ordinary values
 * and does the encoding itself, leaving callers to think in documents rather than
 * buffers. The conversion and the packing happen in the native core reached
 * through the generated contract.
 *
 * @packageDocumentation
 */

import {
  cborToJsonBytes,
  decodeDeltaSamples,
  encodeDeltaSamples,
  jsonToCborBytes,
  Quantizer as NativeQuantizer,
} from '@pamoja/native'

/**
 * Encodes a value as CBOR, which is typically much smaller than its JSON form.
 *
 * @param value - Any JSON-serializable value, or the raw bytes of a JSON
 * document.
 * @returns The CBOR encoding.
 * @throws If the value cannot be encoded.
 */
export function toCbor(value: unknown): Buffer {
  const json =
    value instanceof Uint8Array ? Buffer.from(value) : Buffer.from(JSON.stringify(value), 'utf8')
  return jsonToCborBytes(json)
}

/**
 * Decodes a CBOR document back into an ordinary JavaScript value.
 *
 * @param cbor - The CBOR document to decode.
 * @returns The decoded value.
 * @throws If the document is malformed, or holds a construct with no JSON
 * equivalent such as a non-string map key.
 */
export function fromCbor(cbor: Uint8Array): unknown {
  return JSON.parse(cborToJsonBytes(Buffer.from(cbor)).toString('utf8'))
}

/**
 * Delta-encodes a series of integer samples into a compact buffer.
 *
 * @param samples - The samples, in order.
 * @returns The packed encoding, far smaller than the samples for a slow-moving
 * series.
 */
export function packSamples(samples: readonly number[]): Buffer {
  return encodeDeltaSamples(samples as number[])
}

/**
 * Unpacks a buffer produced by {@link packSamples}.
 *
 * @param bytes - The packed encoding.
 * @returns The samples, in order.
 * @throws If the buffer is malformed.
 */
export function unpackSamples(bytes: Uint8Array): number[] {
  return decodeDeltaSamples(Buffer.from(bytes))
}

/**
 * Packs float readings to a fixed precision, for a link that charges per byte.
 *
 * @example
 * ```ts
 * const quantizer = new Quantizer(100) // keep two decimal places
 * const packed = quantizer.encode([20.0, 20.1, 20.2])
 * quantizer.decode(packed) // [20.0, 20.1, 20.2], to within 0.01
 * ```
 */
export class Quantizer {
  readonly #native: NativeQuantizer

  /**
   * Creates a quantizer at the given precision.
   *
   * @param scale - The multiplier applied before rounding; `100` keeps two
   * decimal places. Must be positive and finite.
   * @throws If the scale is not positive and finite.
   */
  constructor(scale: number) {
    this.#native = new NativeQuantizer(scale)
  }

  /**
   * Quantizes and packs a batch of readings.
   *
   * @param readings - The readings, in order.
   * @returns The packed encoding.
   */
  encode(readings: readonly number[]): Buffer {
    return this.#native.encode(readings as number[])
  }

  /**
   * Unpacks a batch, to within this quantizer's precision.
   *
   * @param bytes - The encoding produced by {@link encode} at the same scale.
   * @returns The readings, in order.
   * @throws If the buffer is malformed.
   */
  decode(bytes: Uint8Array): number[] {
    return this.#native.decode(Buffer.from(bytes))
  }
}
