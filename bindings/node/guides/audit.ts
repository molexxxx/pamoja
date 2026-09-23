// The audit log guide example; see docs/guides/audit.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { AuditEntry, AuditLog, verifyChain } from '@pamoja/audit'
import { DeviceIdentity } from '@pamoja/security'

// The controller signs its own log with a provisioned seed and an auditor holds only the
// public half, so a log can be checked anywhere without the device present.
const seed = Buffer.alloc(32, 7)
const keeper = DeviceIdentity.fromSeed(seed)
const auditor = keeper.publicKey()

const log = new AuditLog(keeper)
const lit = log.append(Buffer.from('burner=on'))
const stopped = log.append(Buffer.from('burner=off'))
console.log(`recorded  burner=on as record ${lit.index} and burner=off as record ${stopped.index}`)

// Each record hashes its own index, the digest of the record before it, and what it
// carries, so the chain fixes the order as well as the contents.
const linked = stopped.previous.equals(lit.digest) ? 'carries' : 'does not carry'
console.log(`chained   record ${stopped.index} ${linked} the digest of record ${lit.index}`)
try {
  verifyChain(auditor, [lit, stopped])
  console.log('verified  the whole log is authentic and in order')
} catch (error) {
  console.log(`rejected  ${(error as Error).message}`)
}

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

// Dropping the first record, or swapping the two, leaves a record where its index says it
// cannot be, so a shortened or reordered log is caught as readily as an edited one.
try {
  verifyChain(auditor, [stopped])
  console.log('a shortened log verified, which should never happen')
} catch (error) {
  console.log(`shortened caught: ${(error as Error).message}`)
}
try {
  verifyChain(auditor, [stopped, lit])
  console.log('a reordered log verified, which should never happen')
} catch (error) {
  console.log(`reordered caught: ${(error as Error).message}`)
}

// A log checked against another device's key fails on the first signature.
const stranger = DeviceIdentity.fromSeed(Buffer.alloc(32, 8)).publicKey()
try {
  verifyChain(stranger, [lit, stopped])
  console.log("another device's key verified the log, which should never happen")
} catch (error) {
  console.log(`stranger  caught: ${(error as Error).message}`)
}

// After a restart the controller loads its seed again and resumes from the last record in
// storage, so the log carries on as one chain rather than starting a second.
const resumed = AuditLog.resume(DeviceIdentity.fromSeed(seed), stopped)
const relit = resumed.append(Buffer.from('burner=on'))
try {
  verifyChain(auditor, [lit, stopped, relit])
  console.log(`resumed   burner=on again as record ${relit.index}, and the whole log still verifies`)
} catch (error) {
  console.log(`rejected  ${(error as Error).message}`)
}

// What a chain cannot show is a record cut from its end, because what is left is still a
// valid chain. The auditor catches it against the last index the device reported.
const reported = relit.index
const cut = [lit, stopped]
verifyChain(auditor, cut)
const ends = cut[cut.length - 1].index
const verdict = ends < reported ? 'a record is missing' : 'nothing is missing'
console.log(
  `cut       the log verifies but ends at record ${ends}, and the device reported record ${reported}: ${verdict}`,
)
// ANCHOR_END: example

assert.doesNotThrow(() => verifyChain(auditor, [lit, stopped, relit]))
assert.throws(() => verifyChain(auditor, [lit, tampered]))
assert.throws(() => verifyChain(auditor, [stopped]))
assert.throws(() => verifyChain(auditor, [stopped, lit]))
assert.throws(() => verifyChain(stranger, [lit, stopped]))
