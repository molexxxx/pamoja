// The Zenoh key expression guide example; see docs/guides/zenoh.md.

import assert from 'node:assert/strict'

// ANCHOR: example
import { keyexpr } from '@pamoja/zenoh'

// A key expression names a set of keys. Chunks sit between slashes, `*` stands for exactly one
// chunk, whatever it holds, and `**` for any number of them, including none.
const selects = (pattern: string, key: string): string => {
  const verdict = keyexpr.matches(pattern, key) ? 'covers' : 'misses'
  return `${verdict.padEnd(10)}${pattern} ${verdict} ${key}`
}
console.log(selects('farm/*/power', 'farm/t7/power'))
console.log(selects('farm/*/power', 'farm/substation/power'))
console.log(`${selects('farm/*/power', 'farm/row2/t14/power')}, since * is exactly one chunk`)
console.log(selects('farm/**/power', 'farm/row2/t14/power'))
console.log(`${selects('farm/**/alarm', 'farm/alarm')}, where ** is no chunk at all`)

// `$*` stands for any run of characters inside one chunk, so it selects on part of a name.
console.log(selects('farm/t$*/power', 'farm/t7/power'))
console.log(selects('farm/t$*/power', 'farm/substation/power'))

// One set of keys has one canonical spelling, and a Zenoh session accepts no other.
for (const written of ['farm/*/**/power', 'farm/**/*/power', 'farm/**/**/power']) {
  const canonical = keyexpr.canonize(written)
  if (keyexpr.isCanon(written)) {
    console.log(`canonical ${written}, as written`)
  } else {
    console.log(`rewritten ${written} is spelled ${canonical}`)
  }
}

// Joining places one expression beneath another, and canonizes the seam between them.
for (const [prefix, suffix] of [
  ['farm/t7', 'power'],
  ['farm/**', '*/power'],
]) {
  console.log(`joined    ${prefix} and ${suffix} make ${keyexpr.join(prefix, suffix)}`)
}

// A malformed expression is refused rather than repaired into something plausible.
for (const [written, why] of [
  ['farm//power', 'a chunk is empty'],
  ['farm/t7*/power', '* stands alone in its chunk, or after $'],
  ['farm/t7/power?', '? and # are reserved'],
]) {
  if (!keyexpr.isValid(written) && keyexpr.canonize(written) === null) {
    console.log(`malformed ${written}, since ${why}`)
  }
}
// ANCHOR_END: example

assert.ok(keyexpr.matches('farm/*/power', 'farm/substation/power'))
assert.ok(!keyexpr.matches('farm/*/power', 'farm/row2/t14/power'))
assert.ok(keyexpr.matches('farm/**/alarm', 'farm/alarm'))
assert.ok(!keyexpr.matches('farm/t$*/power', 'farm/substation/power'))
assert.equal(keyexpr.canonize('farm/**/*/power'), 'farm/*/**/power')
assert.equal(keyexpr.join('farm/**', '*/power'), 'farm/*/**/power')

// ANCHOR: relations
// Two expressions intersect when some key belongs to both. That is the question a router asks
// before it forwards a publication on one to a subscriber on the other.
const overlap = (a: string, b: string): string =>
  keyexpr.intersects(a, b)
    ? `overlap   ${a} and ${b} share a key`
    : `disjoint  ${a} and ${b} share no key`
console.log(overlap('farm/*/power', 'farm/t7/**'))
console.log(overlap('farm/*/power', 'farm/*/alarm'))

// One includes the other when every key of the second belongs to the first, so a bridge that
// already holds the wider subscription declares nothing new for the narrower one.
const covers = (a: string, b: string): string =>
  keyexpr.includes(a, b)
    ? `included  ${a} covers every key of ${b}`
    : `wider     ${b} holds keys ${a} does not`
console.log(covers('farm/**', 'farm/*/power'))
console.log(covers('farm/*/power', 'farm/**'))

// Two spellings of one set include each other, which compares expressions nobody canonized.
const [one, other] = ['farm/**/*/power', 'farm/*/**/power']
if (one !== other && keyexpr.includes(one, other) && keyexpr.includes(other, one)) {
  console.log(`same      ${one} and ${other} select the same keys`)
}
// ANCHOR_END: relations

assert.ok(keyexpr.intersects('farm/*/power', 'farm/t7/**'))
assert.ok(!keyexpr.intersects('farm/*/power', 'farm/*/alarm'))
assert.ok(keyexpr.includes('farm/**', 'farm/*/power'))
assert.ok(!keyexpr.includes('farm/*/power', 'farm/**'))

// ANCHOR: sealed
// A chunk that starts with @ is verbatim: no wildcard selects it, and only the same chunk
// matches it. A second payload version under @v2 stays out of every subscription that does not
// name it, the way Zenoh keeps its own administration space out of **.
const current = 'farm/@v2/t7/power'
console.log(
  `${selects('farm/**', current)}, since no wildcard selects a chunk that starts with @`,
)
console.log(selects('farm/@v2/**', current))
console.log(
  `${overlap('farm/@v1/**', 'farm/@v2/**')}, so a reader of one version never sees the other`,
)
// ANCHOR_END: sealed

assert.ok(!keyexpr.matches('farm/**', current))
assert.ok(keyexpr.matches('farm/@v2/**', current))
assert.ok(!keyexpr.intersects('farm/@v1/**', 'farm/@v2/**'))
