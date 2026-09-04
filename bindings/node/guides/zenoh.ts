// The Zenoh keys guide example; see docs/guides/zenoh.md.

// ANCHOR: example
import assert from 'node:assert/strict'

import { keyexpr } from '@pamoja/zenoh'

// A key expression names a set of keys. `*` stands for exactly one chunk, so this
// selects the battery of any node, and not a battery nested deeper.
assert.ok(keyexpr.isValid('fleet/*/battery'))
assert.ok(keyexpr.matches('fleet/*/battery', 'fleet/n7/battery'))
assert.ok(!keyexpr.matches('fleet/*/battery', 'fleet/n7/rack/battery'))

// `**` stands for any number of chunks, including none, which is what a subscription
// covering a whole subtree wants.
assert.ok(keyexpr.matches('fleet/**', 'fleet/n7/rack/battery'))
assert.ok(keyexpr.matches('fleet/**/battery', 'fleet/battery'))

// Two expressions that select the same keys have one canonical form. Comparing or
// routing on the written form would treat these as different subscriptions.
assert.ok(!keyexpr.isCanon('fleet/**/**/battery'))
assert.equal(keyexpr.canonize('fleet/**/**/battery'), 'fleet/**/battery')
assert.ok(keyexpr.isCanon('fleet/**/battery'))

// A malformed expression is rejected rather than canonized into something plausible.
assert.ok(!keyexpr.isValid('fleet//battery'))
assert.equal(keyexpr.canonize('fleet//battery'), null)
// ANCHOR_END: example
