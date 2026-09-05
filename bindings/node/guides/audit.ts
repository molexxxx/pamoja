// The audit log guide example; see docs/guides/audit.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { AuditEntry, AuditLog, verifyChain } from '@pamoja/audit'
import { DeviceIdentity } from '@pamoja/security'

// The controller signs its own log with a provisioned seed and an auditor holds only the
// public half, so a log can be checked anywhere without the device present.
const keeper = DeviceIdentity.fromSeed(Buffer.alloc(32, 7))
const auditor = keeper.publicKey()

const log = new AuditLog(keeper)
const lit = log.append(Buffer.from('burner=on'))
const stopped = log.append(Buffer.from('burner=off'))
console.log(`recorded  ${lit.index} then ${stopped.index}`)

// Each record hashes its own index, the digest of the record before it, and what it
// carries, so the chain fixes the order as well as the contents.
console.log(`chained   ${stopped.previous.equals(lit.digest)}`)
verifyChain(auditor, [lit, stopped])
console.log('verified  the whole log is authentic and in order')

// Editing a stored record changes the digest its signature covers.
const edited = Buffer.from(stopped.toBytes())
edited[edited.length - 1] ^= 0xff
const tampered = AuditEntry.fromBytes(edited)
try {
  verifyChain(auditor, [lit, tampered])
  console.log('an edited record verified, which should never happen')
} catch (error) {
  console.log(`edited    caught: ${(error as Error).message}`)
}

// Dropping the first record leaves the survivor chained to a link that is no longer there,
// so a shortened log is caught as readily as an edited one.
try {
  verifyChain(auditor, [stopped])
  console.log('a shortened log verified, which should never happen')
} catch (error) {
  console.log(`shortened caught: ${(error as Error).message}`)
}
// ANCHOR_END: example

assert.doesNotThrow(() => verifyChain(auditor, [lit, stopped]))
assert.throws(() => verifyChain(auditor, [lit, tampered]))
assert.throws(() => verifyChain(auditor, [stopped]))
