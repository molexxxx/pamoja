/**
 * Ergonomic facade over the generated serial-framing binding.
 *
 * A serial line is a stream of bytes with no packet boundaries, so something has
 * to mark where one message ends and the next begins. SLIP and COBS are the two
 * ways to do that, and each is offered both as a one-shot call over a complete
 * frame and as a streaming decoder for the arbitrary chunks a port delivers.
 *
 * The streaming decoders are what a real read loop uses. A corrupt frame does not
 * throw, because the frames around it are still good; it is dropped and counted
 * on {@link SlipDecoder.discarded}.
 *
 * @packageDocumentation
 */

import {
  cobsDecode,
  CobsDecoder as NativeCobsDecoder,
  cobsEncode,
  cobsMaxEncodedLen,
  slipDecode,
  SlipDecoder as NativeSlipDecoder,
  slipEncode,
  slipMaxEncodedLen,
} from '@pamoja/native'

/** One of the two byte-stuffing framings this module offers. */
export interface Framing {
  /**
   * Frames a payload for the wire.
   *
   * @param payload - The bytes to send.
   * @returns The frame, delimiter included.
   */
  encode(payload: Uint8Array): Buffer

  /**
   * Reads the payload back out of a complete frame.
   *
   * @param frame - The frame as it arrived.
   * @returns The payload.
   * @throws If the frame is corrupt.
   */
  decode(frame: Uint8Array): Buffer

  /**
   * Returns the largest frame a payload of this length can produce.
   *
   * @param payloadLen - The payload length in bytes.
   * @returns The worst-case frame length.
   */
  maxEncodedLen(payloadLen: number): number
}

/** SLIP (RFC 1055): an `END` byte ends a packet, and an escape pair carries it in the data. */
export const slip: Framing = {
  encode: (payload) => slipEncode(Buffer.from(payload)),
  decode: (frame) => slipDecode(Buffer.from(frame)),
  maxEncodedLen: (payloadLen) => slipMaxEncodedLen(payloadLen),
}

/** COBS: removes the zero byte from the payload so one zero delimits packets unambiguously. */
export const cobs: Framing = {
  encode: (payload) => cobsEncode(Buffer.from(payload)),
  decode: (frame) => cobsDecode(Buffer.from(frame)),
  maxEncodedLen: (payloadLen) => cobsMaxEncodedLen(payloadLen),
}

/**
 * Reassembles whole SLIP frames from the chunks a serial port delivers.
 *
 * @example
 * ```ts
 * const decoder = new SlipDecoder()
 * port.on('data', (chunk) => {
 *   for (const frame of decoder.feed(chunk)) handle(frame)
 * })
 * ```
 */
export class SlipDecoder {
  readonly #native: NativeSlipDecoder

  /** Creates an empty decoder, ready for the first chunk. */
  constructor() {
    this.#native = new NativeSlipDecoder()
  }

  /**
   * Feeds a chunk of the stream.
   *
   * @param chunk - The bytes just read from the port.
   * @returns Every frame this chunk completed, in order, which is often none.
   */
  feed(chunk: Uint8Array): Buffer[] {
    return this.#native.feed(Buffer.from(chunk))
  }

  /** How many corrupt frames this decoder has discarded. */
  get discarded(): number {
    return this.#native.discarded
  }

  /** Discards any partly assembled frame. */
  reset(): void {
    this.#native.reset()
  }
}

/**
 * Reassembles whole COBS frames from the chunks a serial port delivers.
 *
 * The counterpart to {@link SlipDecoder}, for links where the framing overhead
 * has to stay small and predictable.
 */
export class CobsDecoder {
  readonly #native: NativeCobsDecoder

  /** Creates an empty decoder, ready for the first chunk. */
  constructor() {
    this.#native = new NativeCobsDecoder()
  }

  /**
   * Feeds a chunk of the stream.
   *
   * @param chunk - The bytes just read from the port.
   * @returns Every frame this chunk completed, in order.
   */
  feed(chunk: Uint8Array): Buffer[] {
    return this.#native.feed(Buffer.from(chunk))
  }

  /** How many corrupt frames this decoder has discarded. */
  get discarded(): number {
    return this.#native.discarded
  }

  /** Discards any partly assembled frame. */
  reset(): void {
    this.#native.reset()
  }
}
