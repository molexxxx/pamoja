// The Zenoh key expression guide example; see docs/guides/zenoh.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { keyexpr } from '@pamoja/zenoh'

// A key expression names a set of keys. `*` stands for exactly one chunk, so this selects
// the battery of any node, and not a battery nested deeper.
const anyNode = 'fleet/*/battery'
for (const key of ['fleet/n7/battery', 'fleet/n7/rack/battery']) {
  console.log(`${anyNode} covers ${key}: ${keyexpr.matches(anyNode, key)}`)
}

// `**` stands for any number of chunks, including none, which is what a subscription
// covering a whole subtree wants.
console.log(`fleet/** covers a nested key: ${keyexpr.matches('fleet/**', 'fleet/n7/rack/battery')}`)
console.log(
  `fleet/**/battery covers fleet/battery: ${keyexpr.matches('fleet/**/battery', 'fleet/battery')}`,
)

// Two expressions that select the same keys have one canonical form. Comparing or routing
// on the written form would treat these as different subscriptions.
const written = 'fleet/**/**/battery'
const canonical = keyexpr.canonize(written)
console.log(`${written} is canonical: ${keyexpr.isCanon(written)}, and canonizes to ${canonical}`)

// A malformed expression is rejected rather than canonized into something plausible.
const malformed = 'fleet//battery'
console.log(
  `${malformed} is valid: ${keyexpr.isValid(malformed)},` +
    ` canonizes to ${keyexpr.canonize(malformed)}`,
)
// ANCHOR_END: example

assert.ok(keyexpr.isValid(anyNode))
assert.ok(keyexpr.matches(anyNode, 'fleet/n7/battery'))
assert.ok(!keyexpr.matches(anyNode, 'fleet/n7/rack/battery'))
assert.ok(keyexpr.matches('fleet/**', 'fleet/n7/rack/battery'))
assert.ok(keyexpr.matches('fleet/**/battery', 'fleet/battery'))
assert.ok(!keyexpr.isCanon(written))
assert.equal(canonical, 'fleet/**/battery')
assert.ok(keyexpr.isCanon('fleet/**/battery'))
assert.ok(!keyexpr.isValid(malformed))
assert.equal(keyexpr.canonize(malformed), null)
