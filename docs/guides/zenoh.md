# Zenoh keys

A Zenoh key expression names a set of keys rather than one, which is how a
subscription covers a fleet instead of a node. The language is small: chunks
separated by slashes, `*` for exactly one chunk, and `**` for any number of them
including none. pamoja implements the rules and nothing else, so a gateway can
decide what a subscription covers, and whether two of them are the same, without
a Zenoh installation anywhere near it.

## What the example does

It asks what each wildcard selects, using keys under a fleet subtree as the
subject, then canonizes an expression written with a repeated wildcard, and
finally offers a malformed one.

The canonical form is not written out. `canonize` derives `fleet/**/battery`
from the redundant `fleet/**/**/battery`, and the example asks the library
whether what comes back is canonical rather than assuming it.

It proves:

- `*` stands for exactly one chunk, so `fleet/*/battery` covers
  `fleet/n7/battery` and not `fleet/n7/rack/battery`.
- `**` stands for any number of chunks, so `fleet/**` covers
  `fleet/n7/rack/battery`, and `fleet/**/battery` covers `fleet/battery`, where
  it stands for none at all.
- A repeated wildcard is not canonical. `fleet/**/**/battery` canonizes to
  `fleet/**/battery`, and that form is canonical, so a router compares
  subscriptions in it rather than as written.
- An empty chunk makes `fleet//battery` invalid, and canonizing it yields
  nothing rather than a repaired expression.

## Rust

<!-- snippet: examples/tests/guides/zenoh.rs#example -->
From [`examples/tests/guides/zenoh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/tests/guides/zenoh.rs):

```rust
use pamoja_zenoh::keyexpr::{canonize, is_canon, is_valid, matches};

// A key expression names a set of keys. `*` stands for exactly one chunk, so this
// selects the battery of any node, and not a battery nested deeper.
let any_node = "fleet/*/battery";
for key in ["fleet/n7/battery", "fleet/n7/rack/battery"] {
    println!("{any_node} covers {key}: {}", matches(any_node, key));
}

// `**` stands for any number of chunks, including none, which is what a subscription
// covering a whole subtree wants.
println!(
    "fleet/** covers a nested key: {}",
    matches("fleet/**", "fleet/n7/rack/battery")
);
println!(
    "fleet/**/battery covers fleet/battery: {}",
    matches("fleet/**/battery", "fleet/battery")
);

// Two expressions that select the same keys have one canonical form. Comparing or
// routing on the written form would treat these as different subscriptions.
let written = "fleet/**/**/battery";
let canonical = canonize(written);
println!(
    "{written} is canonical: {}, and canonizes to {canonical:?}",
    is_canon(written)
);

// A malformed expression is rejected rather than canonized into something plausible.
let malformed = "fleet//battery";
println!(
    "{malformed} is valid: {}, canonizes to {:?}",
    is_valid(malformed),
    canonize(malformed)
);
```
<!-- end -->

## TypeScript

<!-- snippet: bindings/node/guides/zenoh.ts#example -->
From [`bindings/node/guides/zenoh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/zenoh.ts):

```typescript
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
```
<!-- end -->

## Python

<!-- snippet: bindings/python/guides/zenoh.py#example -->
From [`bindings/python/guides/zenoh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/zenoh.py):

```python
from pamoja.zenoh import canonize, is_canon, is_valid, matches

# A key expression names a set of keys. `*` stands for exactly one chunk, so this selects
# the battery of any node, and not a battery nested deeper.
any_node = "fleet/*/battery"
for key in ("fleet/n7/battery", "fleet/n7/rack/battery"):
    print(f"{any_node} covers {key}: {matches(any_node, key)}")

# `**` stands for any number of chunks, including none, which is what a subscription
# covering a whole subtree wants.
print(f"fleet/** covers a nested key: {matches('fleet/**', 'fleet/n7/rack/battery')}")
print(f"fleet/**/battery covers fleet/battery: {matches('fleet/**/battery', 'fleet/battery')}")

# Two expressions that select the same keys have one canonical form. Comparing or routing
# on the written form would treat these as different subscriptions.
written = "fleet/**/**/battery"
canonical = canonize(written)
print(f"{written} is canonical: {is_canon(written)}, and canonizes to {canonical}")

# A malformed expression is rejected rather than canonized into something plausible.
malformed = "fleet//battery"
print(f"{malformed} is valid: {is_valid(malformed)}, canonizes to {canonize(malformed)}")
```
<!-- end -->

## C#

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs):

```csharp
// A key expression names a set of keys. `*` stands for exactly one chunk, so this
// selects the battery of any node, and not a battery nested deeper.
const string AnyNode = "fleet/*/battery";
foreach (string key in new[] { "fleet/n7/battery", "fleet/n7/rack/battery" })
{
    Console.WriteLine($"{AnyNode} covers {key}: {KeyExpression.Matches(AnyNode, key)}");
}

// `**` stands for any number of chunks, including none, which is what a
// subscription covering a whole subtree wants.
Console.WriteLine(
    "fleet/** covers a nested key: "
    + KeyExpression.Matches("fleet/**", "fleet/n7/rack/battery"));
Console.WriteLine(
    "fleet/**/battery covers fleet/battery: "
    + KeyExpression.Matches("fleet/**/battery", "fleet/battery"));

// Two expressions that select the same keys have one canonical form. Comparing or
// routing on the written form would treat these as different subscriptions.
const string Written = "fleet/**/**/battery";
string? canonical = KeyExpression.Canonize(Written);
Console.WriteLine(
    $"{Written} is canonical: {KeyExpression.IsCanon(Written)},"
    + $" and canonizes to {canonical}");

// A malformed expression is rejected rather than canonized into something
// plausible.
const string Malformed = "fleet//battery";
Console.WriteLine(
    $"{Malformed} is valid: {KeyExpression.IsValid(Malformed)},"
    + $" canonizes to {KeyExpression.Canonize(Malformed) ?? "nothing"}");
```
<!-- end -->

## Reference

<!-- table: reference zenoh -->
- Rust: [`pamoja-zenoh`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html)
- TypeScript: [`@pamoja/zenoh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html)
- Python: [`pamoja.zenoh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html)
- C#: [`Pamoja.Zenoh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html)
<!-- end -->
