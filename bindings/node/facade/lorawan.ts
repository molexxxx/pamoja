/**
 * Ergonomic facade over the generated LoRaWAN binding.
 *
 * A long-range public-band link is wide open, so LoRaWAN wraps every frame in two
 * guarantees: a message integrity code keyed to the network proves the frame is
 * authentic and intact, and the payload is encrypted to the application so only
 * its owner can read it. This builds and verifies exactly that.
 *
 * The frame direction is re-exported as a runtime {@link Direction} object,
 * because the generated enum is types-only.
 *
 * @packageDocumentation
 */

import {
  LORAWAN_MAX_FRAME,
  LORAWAN_MAX_PAYLOAD,
  LorawanDevice,
  LorawanJoinAccept,
  type LorawanOptions,
  LorawanSession,
} from '../index'

export { type LorawanOptions as Options }

/** The largest application payload, in bytes, a single frame can carry. */
export const MAX_PAYLOAD = LORAWAN_MAX_PAYLOAD

/** The largest frame, in bytes, this build accepts. */
export const MAX_FRAME = LORAWAN_MAX_FRAME

/**
 * The direction a frame travelled, which its MIC and encryption both fold in.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`Direction.Uplink`) and a type annotation.
 */
export const Direction = {
  /** From an end device up to the network. */
  Uplink: 'Uplink',
  /** From the network down to an end device. */
  Downlink: 'Downlink',
} as const

/** One of the {@link Direction} values. */
export type Direction = (typeof Direction)[keyof typeof Direction]

/** A decoded data frame, with its payload decrypted. */
export interface RxData {
  /** The direction the frame travelled. */
  direction: Direction
  /** The device address the frame carries. */
  devAddr: number
  /** The low 16 bits of the frame counter. */
  fcnt: number
  /** Whether the frame asks to be acknowledged. */
  confirmed: boolean
  /** Whether the frame takes part in adaptive data rate. */
  adr: boolean
  /** Whether the frame acknowledges the last confirmed one. */
  ack: boolean
  /** Whether the network has more downlink data waiting. */
  fpending: boolean
  /** The port the frame was sent on, or `null` when it carries only options. */
  fport: number | null
  /** The MAC commands the header carried. */
  fopts: Buffer
  /** The decrypted application payload. */
  payload: Buffer
}

/** An activated LoRaWAN session: a device address and its two session keys. */
export class Session {
  readonly #inner: LorawanSession

  /**
   * Wraps an activated session.
   *
   * @param inner - The generated session this facade delegates to.
   */
  constructor(inner: LorawanSession) {
    this.#inner = inner
  }

  /** The device address this session is bound to. */
  get devAddr(): number {
    return this.#inner.devAddr
  }

  /**
   * Encodes an uplink, encrypting the payload and appending the MIC.
   *
   * @param fcnt - The frame counter for this uplink.
   * @param fport - The port; 0 for MAC commands, otherwise an application port.
   * @param payload - The application payload to carry.
   * @param options - The header flags and frame options to set.
   * @returns The frame to transmit.
   * @throws If the payload and options do not fit a single frame.
   */
  encodeUplink(
    fcnt: number,
    fport: number,
    payload: Uint8Array,
    options?: LorawanOptions,
  ): Buffer {
    return this.#inner.encodeUplink(fcnt, fport, Buffer.from(payload), options)
  }

  /**
   * Encodes a downlink, encrypting the payload and appending the MIC.
   *
   * @param fcnt - The frame counter for this downlink.
   * @param fport - The port; 0 for MAC commands, otherwise an application port.
   * @param payload - The application payload to carry.
   * @param options - The header flags and frame options to set.
   * @returns The frame to transmit.
   * @throws If the payload and options do not fit a single frame.
   */
  encodeDownlink(
    fcnt: number,
    fport: number,
    payload: Uint8Array,
    options?: LorawanOptions,
  ): Buffer {
    return this.#inner.encodeDownlink(fcnt, fport, Buffer.from(payload), options)
  }

  /**
   * Verifies a received frame, then decrypts it.
   *
   * @param bytes - The frame exactly as it came off the radio.
   * @param fcnt - The full 32-bit counter expected for this frame; its low 16
   *   bits must match the counter the frame carries.
   * @returns The decoded frame.
   * @throws If the MIC does not verify, the counter does not match, or the frame
   *   is not a data frame.
   */
  decode(bytes: Uint8Array, fcnt: number): RxData {
    // The generated object leaves an absent port undefined; null says the same
    // thing in the shape the rest of this package uses.
    const rx = this.#inner.decode(Buffer.from(bytes), fcnt)
    return { ...rx, fport: rx.fport ?? null }
  }
}

/** An accepted join: the network settings, and the session it grants. */
export class JoinAccept {
  readonly #inner: LorawanJoinAccept

  /**
   * Wraps an accepted join.
   *
   * @param inner - The generated join this facade delegates to.
   */
  constructor(inner: LorawanJoinAccept) {
    this.#inner = inner
  }

  /** The device address the network assigned. */
  get devAddr(): number {
    return this.#inner.devAddr
  }

  /** The identifier of the network that accepted the join. */
  get netId(): number {
    return this.#inner.netId
  }

  /**
   * The downlink settings byte, carrying the second receive window data rate and
   * the first window offset.
   */
  get dlSettings(): number {
    return this.#inner.dlSettings
  }

  /** The delay before the first receive window, in seconds. */
  get rxDelay(): number {
    return this.#inner.rxDelay
  }

  /**
   * Takes the activated session this join grants.
   *
   * @returns The session, with its keys already derived.
   */
  session(): Session {
    return new Session(this.#inner.session())
  }
}

/** The root credentials over-the-air activation is built on. */
export class Device {
  readonly #inner: LorawanDevice

  /**
   * Creates a device from its two EUIs and its application key.
   *
   * @param devEui - The 8-byte device EUI.
   * @param appEui - The 8-byte application (join) EUI.
   * @param appKey - The 16-byte application key the join is secured with.
   * @throws If any credential is the wrong length.
   */
  constructor(devEui: Uint8Array, appEui: Uint8Array, appKey: Uint8Array) {
    this.#inner = new LorawanDevice(
      Buffer.from(devEui),
      Buffer.from(appEui),
      Buffer.from(appKey),
    )
  }

  /**
   * Builds the join request this device broadcasts to activate.
   *
   * @param devNonce - A nonce that must never repeat for this device, since the
   *   network rejects a replayed one.
   * @returns The join request to transmit.
   */
  joinRequest(devNonce: number): Buffer {
    return this.#inner.joinRequest(devNonce)
  }

  /**
   * Turns the join accept a network sent into the settings it grants.
   *
   * @param bytes - The join accept exactly as it arrived.
   * @param devNonce - The nonce the matching join request carried.
   * @returns The accepted join, which grants a session.
   * @throws If the MIC does not verify, or the frame is not a join accept.
   */
  acceptJoin(bytes: Uint8Array, devNonce: number): JoinAccept {
    return new JoinAccept(this.#inner.acceptJoin(Buffer.from(bytes), devNonce))
  }
}

/**
 * Creates a session for a device already activated by personalization.
 *
 * @param devAddr - The device address the network assigned.
 * @param nwkSKey - The 16-byte network session key, which authenticates frames.
 * @param appSKey - The 16-byte application session key, which encrypts payloads.
 * @returns The session, ready to encode and decode data frames.
 * @throws If either key is not 16 bytes.
 */
export function session(
  devAddr: number,
  nwkSKey: Uint8Array,
  appSKey: Uint8Array,
): Session {
  return new Session(
    new LorawanSession(devAddr, Buffer.from(nwkSKey), Buffer.from(appSKey)),
  )
}

/**
 * Creates a device holding the root credentials for over-the-air activation.
 *
 * @param devEui - The 8-byte device EUI.
 * @param appEui - The 8-byte application (join) EUI.
 * @param appKey - The 16-byte application key the join exchange is secured with.
 * @returns The device, ready to build a join request.
 * @throws If any credential is the wrong length.
 */
export function device(
  devEui: Uint8Array,
  appEui: Uint8Array,
  appKey: Uint8Array,
): Device {
  return new Device(devEui, appEui, appKey)
}
