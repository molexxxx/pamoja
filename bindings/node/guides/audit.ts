// The audit log guide example; see docs/guides/audit.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { AuditEntry, AuditLog, verifyChain } from '@pamoja/audit'
import { DeviceIdentity } from '@pamoja/security'

// The controller signs its own log with a provisioned seed. This one is RFC 8032 test
// vector 1, so the key the records are checked against is a published constant rather
// than a value checked against itself.
const keeper = DeviceIdentity.fromSeed(
  Buffer.from('9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60', 'hex')
)
assert.equal(
  keeper.publicKey().toString('hex'),
  'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a'
)

const log = new AuditLog(keeper)
const lit = log.append(Buffer.from('burner=on'))
const stopped = log.append(Buffer.from('burner=off'))

// A record's digest is SHA-256 over its little-endian index, the digest of the record
// before it, and its payload, so the first record hashes forty zero bytes and then what
// it carries.
assert.equal(lit.index, 0)
assert.equal(
  lit.digest.toString('hex'),
  'e50c6a7a944fab6dd13ffdb760ca190e14ea00c168ba7c948745ba0af146c159'
)
assert.deepEqual(stopped.previous, lit.digest)
assert.ok(verifyChain(keeper.publicKey(), [lit, stopped]))

// Editing a stored record changes the digest its signature covers.
const edited = stopped.toBytes()
edited[edited.length - 1] ^= 0xff
const tampered = AuditEntry.fromBytes(edited)
assert.ok(!verifyChain(keeper.publicKey(), [lit, tampered]))

// Dropping the record before it leaves the survivor chained to a link that is no longer
// there, so a shortened log is caught as readily as an edited one.
assert.ok(!verifyChain(keeper.publicKey(), [stopped]))
// ANCHOR_END: example
