/**
 * Ergonomic facade over the generated update binding.
 *
 * A device that cannot be fixed in the field is a device that has to be visited,
 * and some of them are a day's travel away. Signed updates make that a network
 * operation instead: a release carries a manifest naming who it is for and what
 * it hashes to, a device refuses anything not signed by the key it trusts, and
 * an image that fails to confirm itself is rolled back to the one that worked.
 *
 * The slot state and boot action are re-exported as runtime objects, because
 * the generated enums are types-only.
 *
 * @packageDocumentation
 */

import {
  type Delegation,
  type Manifest,
  signDelegation as nativeSignDelegation,
  signManifest as nativeSignManifest,
} from '@pamoja/native'
import { DeviceIdentity } from '@pamoja/security'

export {
  decodeManifest,
  encodeManifest,
  envelopeBody,
  ImageVerifier,
  openDelegation,
  UPDATE_FORMAT_RAW as FORMAT_RAW,
  UPDATE_STRUCTURE_VERSION as STRUCTURE_VERSION,
  Updater,
  verifyEnvelope,
} from '@pamoja/native'

export type { Boot, Delegation, Manifest, Progress, SlotRecord } from '@pamoja/native'

/**
 * Signs a manifest into the envelope that is offered to a device.
 *
 * @param manifest - What the release says about itself.
 * @param author - The identity signing the release.
 * @returns The signed envelope.
 */
export function signManifest(manifest: Manifest, author: DeviceIdentity): Buffer {
  return nativeSignManifest(manifest, DeviceIdentity.native(author))
}

/**
 * Signs a delegation, naming a release key the anchor stands behind.
 *
 * Keeping the anchor offline and rotating a release key under it is the
 * arrangement to prefer, because the key that signs day to day is the one most
 * likely to be stolen.
 *
 * @param delegation - The statement to sign.
 * @param anchor - The anchor identity, which is the root of the trust.
 * @returns The signed delegation envelope.
 */
export function signDelegation(
  delegation: Delegation,
  anchor: DeviceIdentity,
): Buffer {
  return nativeSignDelegation(delegation, DeviceIdentity.native(anchor))
}

/**
 * What a device believes about one slot.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const SlotState = {
  /** Nothing has been written here. */
  Empty: 'Empty',
  /** An image is arriving, and `written` says how much of it has. */
  Receiving: 'Receiving',
  /** A complete image that matched its manifest, not yet tried. */
  Staged: 'Staged',
  /** Being tried for the first time; it reverts unless it confirms. */
  Pending: 'Pending',
  /** Tried and confirmed working. */
  Confirmed: 'Confirmed',
  /** Tried and did not confirm, so it will not be tried again. */
  Failed: 'Failed',
} as const

/** One of the {@link SlotState} choices. */
export type SlotState = (typeof SlotState)[keyof typeof SlotState]

/**
 * What a bootloader should do with what it found.
 *
 * Provided as a runtime object plus a matching string-union type.
 */
export const BootAction = {
  /** Nothing new to try; run the confirmed image. */
  Confirmed: 'Confirmed',
  /** A staged image is being tried for the first time. */
  Trying: 'Trying',
  /** A pending image never confirmed, so it was failed. */
  Reverted: 'Reverted',
} as const

/** One of the {@link BootAction} choices. */
export type BootAction = (typeof BootAction)[keyof typeof BootAction]
