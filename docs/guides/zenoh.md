# Zenoh keys

A Zenoh key expression names a set of keys rather than one, which is how a
subscription covers a fleet instead of a node. The language is small: chunks
separated by slashes, `*` for exactly one chunk, and `**` for any number of them
including none. pamoja implements the rules and nothing else, so a gateway can
decide what a subscription covers, and whether two of them are the same, without
a Zenoh installation anywhere near it.

## What the example does

It checks what each wildcard selects, using a fleet subtree as the subject, then
canonizes an expression written with a redundant wildcard and confirms the
canonical form selects the same keys. Finally it offers a malformed expression.

It proves:

- `*` stands for exactly one chunk, so a pattern for a node's battery does not
  select a battery nested a level deeper.
- `**` stands for any number of chunks, including none, so a subtree pattern
  matches both a deep key and the bare one.
- Two expressions that select the same keys have one canonical form, which is why
  routing or comparing subscriptions canonizes first rather than comparing the
  text.
- A malformed expression is rejected rather than canonized into something
  plausible.

## Rust

<!-- snippet: examples/tests/guides/zenoh.rs#example -->
From [`examples/tests/guides/zenoh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/zenoh.rs):

```rust
use pamoja_zenoh::keyexpr::{canonize, is_canon, is_valid, matches};

// A key expression names a set of keys. `*` stands for exactly one chunk, so this
// selects the battery of any node, and not a battery nested deeper.
assert!(is_valid("fleet/*/battery"));
assert!(matches("fleet/*/battery", "fleet/n7/battery"));
assert!(!matches("fleet/*/battery", "fleet/n7/rack/battery"));

// `**` stands for any number of chunks, including none, which is what a subscription
// covering a whole subtree wants.
assert!(matches("fleet/**", "fleet/n7/rack/battery"));
assert!(matches("fleet/**/battery", "fleet/battery"));

// Two expressions that select the same keys have one canonical form. Comparing or
// routing on the written form would treat these as different subscriptions.
assert!(!is_canon("fleet/**/**/battery"));
assert_eq!(
    canonize("fleet/**/**/battery").as_deref(),
    Some("fleet/**/battery")
);
assert!(is_canon("fleet/**/battery"));

// A malformed expression is rejected rather than canonized into something plausible.
assert!(!is_valid("fleet//battery"));
assert_eq!(canonize("fleet//battery"), None);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/zenoh.ts#example -->
From [`bindings/node/guides/zenoh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/zenoh.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/zenoh.py#example -->
From [`bindings/python/guides/zenoh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/zenoh.py):

```python
from pamoja.zenoh import canonize, is_canon, is_valid, matches

# A key expression names a set of keys. `*` stands for exactly one chunk, so this
# selects the battery of any node, and not a battery nested deeper.
assert is_valid("fleet/*/battery")
assert matches("fleet/*/battery", "fleet/n7/battery")
assert not matches("fleet/*/battery", "fleet/n7/rack/battery")

# `**` stands for any number of chunks, including none, which is what a subscription
# covering a whole subtree wants.
assert matches("fleet/**", "fleet/n7/rack/battery")
assert matches("fleet/**/battery", "fleet/battery")

# Two expressions that select the same keys have one canonical form. Comparing or
# routing on the written form would treat these as different subscriptions.
assert not is_canon("fleet/**/**/battery")
assert canonize("fleet/**/**/battery") == "fleet/**/battery"
assert is_canon("fleet/**/battery")

# A malformed expression is rejected rather than canonized into something plausible.
assert not is_valid("fleet//battery")
assert canonize("fleet//battery") is None
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs):

```csharp
// A key expression names a set of keys. `*` stands for exactly one chunk, so this
// selects the battery of any node, and not a battery nested deeper.
Expect(KeyExpression.IsValid("fleet/*/battery"), "the pattern is well formed");
Expect(
    KeyExpression.Matches("fleet/*/battery", "fleet/n7/battery"),
    "one chunk stands in for the node");
Expect(
    !KeyExpression.Matches("fleet/*/battery", "fleet/n7/rack/battery"),
    "but not for two");

// `**` stands for any number of chunks, including none, which is what a
// subscription covering a whole subtree wants.
Expect(
    KeyExpression.Matches("fleet/**", "fleet/n7/rack/battery"),
    "the subtree wildcard reaches any depth");
Expect(
    KeyExpression.Matches("fleet/**/battery", "fleet/battery"),
    "including no chunks at all");

// Two expressions that select the same keys have one canonical form. Comparing or
// routing on the written form would treat these as different subscriptions.
Expect(!KeyExpression.IsCanon("fleet/**/**/battery"), "a repeated wildcard is not canonical");
Expect(
    KeyExpression.Canonize("fleet/**/**/battery") == "fleet/**/battery",
    "and collapses to the form that selects the same keys");
Expect(KeyExpression.IsCanon("fleet/**/battery"), "which is canonical");

// A malformed expression is rejected rather than canonized into something plausible.
Expect(!KeyExpression.IsValid("fleet//battery"), "an empty chunk is malformed");
Expect(KeyExpression.Canonize("fleet//battery") is null, "and has no canonical form");
```
<!-- end -->

## Reference

<!-- table: reference zenoh -->
- Rust: [`pamoja-zenoh`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html)
- TypeScript: [`@pamoja/zenoh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html)
- Python: [`pamoja.zenoh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html)
- C#: [`Pamoja.Zenoh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html)
<!-- end -->
