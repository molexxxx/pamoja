/**
 * Ergonomic facade over the generated audit binding.
 *
 * A log that can be edited after the fact proves nothing. Each record here is
 * signed and carries the hash of the one before it, so altering a record,
 * dropping one, or reordering two breaks the chain at that point and at every
 * point after it. The index is part of what is signed, which is what makes a
 * record removed from the end detectable too.
 *
 * @packageDocumentation
 */

import { AuditEntry, AuditLog as NativeLog, verifyAuditChain } from '@pamoja/native'
import { DeviceIdentity } from '@pamoja/security'

export { AuditEntry, AuditVerifier } from '@pamoja/native'

/** A log that signs what it is given and chains it onto what came before. */
export class AuditLog {
  readonly #inner: NativeLog

  /**
   * Creates a log that signs with a device identity.
   *
   * @param identity - The identity whose signature each record will carry.
   * @param last - The final record of an existing log to carry on from, or
   *   omitted to start empty.
   */
  constructor(identity: DeviceIdentity, last?: AuditEntry) {
    const signer = DeviceIdentity.native(identity)
    this.#inner =
      last === undefined ? new NativeLog(signer) : NativeLog.resume(signer, last)
  }

  /**
   * Creates a log that carries on from the last record an earlier one wrote.
   *
   * This is what a device does after a restart: the chain continues at the next
   * index and hashes onto the record it left off at, so a reboot leaves no gap
   * for a record to be removed through.
   *
   * @param identity - The identity whose signature each record will carry.
   * @param last - The final record of the existing log.
   */
  static resume(identity: DeviceIdentity, last: AuditEntry): AuditLog {
    return new AuditLog(identity, last)
  }

  /**
   * Appends a payload, signing it and chaining it onto the last record.
   *
   * @param payload - The record to store.
   * @returns The new entry.
   */
  append(payload: Uint8Array): AuditEntry {
    return this.#inner.append(Buffer.from(payload))
  }
}

/**
 * Checks a whole chain that has already arrived.
 *
 * @param publicKey - The 32-byte key the records were signed with.
 * @param entries - The records, in the order they were written.
 * @returns `true` when every record follows the one before it and carries a
 *   signature that holds.
 */
export function verifyChain(publicKey: Uint8Array, entries: AuditEntry[]): boolean {
  return verifyAuditChain(Buffer.from(publicKey), entries)
}
