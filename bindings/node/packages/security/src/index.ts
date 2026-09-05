/**
 * Ergonomic facade over the generated device-identity binding.
 *
 * Adds string-or-bytes payloads and a named constructor, without adding
 * behavior; the signing and verifying happen in the native core reached through
 * the generated contract.
 *
 * @packageDocumentation
 */

import {
  DeviceIdentity as NativeDeviceIdentity,
  fingerprint as nativeFingerprint,
  verify as nativeVerify,
  verifyMessage as nativeVerifyMessage,
} from '@pamoja/native'

/** A payload to sign or verify; strings are encoded as UTF-8. */
export type Payload = string | Uint8Array

/**
 * Encodes a payload to bytes, so callers may pass either text or raw data.
 *
 * @param payload - The value to encode.
 * @returns The payload as a buffer.
 */
function bytes(payload: Payload): Buffer {
  return typeof payload === 'string' ? Buffer.from(payload, 'utf8') : Buffer.from(payload)
}

/**
 * A device's private signing identity.
 *
 * A reading that drives a health or billing decision has to be provably from the
 * device that claims to have sent it, and provably unaltered on the way. Sign it
 * here, and any holder of {@link publicKey} can check it with {@link verify}.
 *
 * @example
 * ```ts
 * const device = DeviceIdentity.fromSeed(seed)
 * const signature = device.sign('21.5')
 * verify(device.publicKey(), '21.5', signature) // true
 * ```
 */
export class DeviceIdentity {
  readonly #native: NativeDeviceIdentity

  /**
   * Creates an identity from a provisioned 32-byte secret seed.
   *
   * @param seed - The device's 32-byte secret, held on the device only.
   */
  constructor(seed: Uint8Array) {
    this.#native = new NativeDeviceIdentity(Buffer.from(seed))
  }

  /**
   * Creates an identity from a provisioned 32-byte secret seed.
   *
   * @param seed - The device's 32-byte secret, held on the device only.
   * @returns The identity that seed determines.
   */
  static fromSeed(seed: Uint8Array): DeviceIdentity {
    return new DeviceIdentity(seed)
  }

  /**
   * Hands the generated identity to another capability facade.
   *
   * The audit and update capabilities sign with an identity this class holds,
   * and the generated bindings take the generated type. This is how the two
   * meet without a caller ever seeing it.
   *
   * @internal
   */
  static native(identity: DeviceIdentity): NativeDeviceIdentity {
    return identity.#native
  }

  /**
   * Returns the public key matching this identity, which is safe to share.
   *
   * @returns The 32-byte public key.
   */
  publicKey(): Buffer {
    return this.#native.publicKey()
  }

  /**
   * Returns the short hex fingerprint of this identity, for logs and displays.
   *
   * @returns A 16-character lowercase hex label.
   */
  fingerprint(): string {
    return this.#native.fingerprint()
  }

  /**
   * Signs a payload.
   *
   * @param payload - The bytes to cover; strings are encoded as UTF-8.
   * @returns The 64-byte detached signature.
   */
  sign(payload: Payload): Buffer {
    return this.#native.sign(bytes(payload))
  }

  /**
   * Signs a payload and returns one message carrying both.
   *
   * The message is the signature followed by the payload, which is usually what goes
   * on a link: one blob to send, rather than a payload and a detached signature to
   * keep together and split correctly at the far end. {@link verifyMessage} reverses
   * it.
   *
   * @param payload - The bytes to cover; strings are encoded as UTF-8.
   * @returns The signature followed by the payload.
   */
  signMessage(payload: Payload): Buffer {
    return this.#native.signMessage(bytes(payload))
  }
}

/**
 * Verifies a signed message and returns the payload it carries.
 *
 * @param publicKey - The 32-byte public key of the claimed signer.
 * @param message - The signature followed by the payload, as `signMessage` wrote it.
 * @returns The payload if the message is authentic, and `null` if it is too short to
 * hold a signature, was altered, or was signed by a different device.
 * @throws If the key is not the expected length.
 */
export function verifyMessage(publicKey: Uint8Array, message: Uint8Array): Buffer | null {
  return nativeVerifyMessage(Buffer.from(publicKey), Buffer.from(message))
}

/**
 * Verifies that a signature covers a payload and was made by a public key.
 *
 * @param publicKey - The 32-byte public key of the claimed signer.
 * @param payload - The bytes the signature should cover.
 * @param signature - The 64-byte detached signature.
 * @returns `true` if the signature is authentic, and `false` if the payload was
 * altered or was signed by a different device.
 * @throws If an argument is not the expected length.
 */
export function verify(publicKey: Uint8Array, payload: Payload, signature: Uint8Array): boolean {
  return nativeVerify(Buffer.from(publicKey), bytes(payload), Buffer.from(signature))
}

/**
 * Returns the short hex fingerprint of a public key.
 *
 * @param publicKey - The 32-byte public key to label.
 * @returns A 16-character lowercase hex label.
 * @throws If the key is not a valid 32-byte public key.
 */
export function fingerprint(publicKey: Uint8Array): string {
  return nativeFingerprint(Buffer.from(publicKey))
}
