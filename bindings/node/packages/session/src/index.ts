/**
 * Ergonomic facade over the generated session binding.
 *
 * Two devices that already know each other's public keys can agree on a session
 * key without ever sending it, and then exchange messages that are confidential,
 * cannot be altered undetected, and cannot be replayed. That is the whole of
 * what a small device usually needs from transport security, at a fraction of
 * what a TLS stack costs it.
 *
 * The role is re-exported as a runtime {@link Role} object, because the
 * generated enum is types-only.
 *
 * @packageDocumentation
 */

import type { Role as RoleName } from '@pamoja/native'

import {
  AgreementKey as NativeAgreementKey,
  hkdfSha256Expand,
  hmacSha256Digest,
  Role as NativeRole,
  type SealedMessage,
  Session as NativeSession,
} from '@pamoja/native'

export { AgreementKey } from '@pamoja/native'
export type { SealedMessage }

/**
 * Which side of a session a device is on.
 *
 * The two devices must choose opposite roles: the role decides the order the
 * public keys are mixed in and which direction each side tags its messages
 * with, so a session where both sides claim the same role opens nothing.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const Role = {
  /** The device that opens the session. */
  Initiator: 'Initiator' as RoleName,
  /** The device that answers. */
  Responder: 'Responder' as RoleName,
} as const

/** One of the {@link Role} choices. */
export type Role = RoleName

/** A confidential, tamper-evident, replay-protected channel with one peer. */
export class Session {
  readonly #inner: NativeSession

  /**
   * Establishes a session with a peer.
   *
   * @param local - This device's key-agreement secret.
   * @param peerPublicKey - The peer's 32-byte public key, already authenticated
   *   by pinning or by a signature.
   * @param salt - A fresh per-session salt both sides share, exchanged in the
   *   clear. Reusing one with the same pair of keys reuses the session key, so
   *   it must change each session.
   * @param role - Whether this device opens the session or answers.
   */
  constructor(
    local: NativeAgreementKey,
    peerPublicKey: Buffer,
    salt: Buffer,
    role: Role,
  ) {
    this.#inner = new NativeSession(local, peerPublicKey, salt, role as NativeRole)
  }

  /**
   * Seals a message for the peer.
   *
   * @param plaintext - The message to protect.
   * @param aad - Data authenticated but not encrypted, so it stays readable on
   *   the wire yet cannot be altered: a device identifier or a routing header
   *   belongs here.
   * @returns The ciphertext, with the counter and tag to send beside it.
   */
  seal(plaintext: Buffer, aad?: Buffer): SealedMessage {
    return this.#inner.seal(plaintext, aad)
  }

  /**
   * Opens a message from the peer.
   *
   * @param sealed - The ciphertext with the counter and tag that arrived with
   *   it.
   * @param aad - The same associated data the sender authenticated.
   * @returns The plaintext.
   * @throws When the counter repeats or is older than the replay window still
   *   tracks, and when the tag does not authenticate. Nothing readable is ever
   *   returned from a message that failed either check.
   */
  open(sealed: SealedMessage, aad?: Buffer): Buffer {
    return this.#inner.open(sealed, aad)
  }
}

/**
 * Computes a keyed hash over a message.
 *
 * This is the primitive a host uses to authenticate a pairing exchange or a
 * single command, where a whole session would be more than the job needs.
 */
export function hmacSha256(key: Buffer, message: Buffer): Buffer {
  return hmacSha256Digest(key, message)
}

/** Expands input keying material into `length` bytes bound to `info`. */
export function hkdfSha256(
  salt: Buffer,
  ikm: Buffer,
  info: Buffer,
  length: number,
): Buffer {
  return hkdfSha256Expand(salt, ikm, info, length)
}
