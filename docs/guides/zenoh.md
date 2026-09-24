# Zenoh keys

A Zenoh key expression names a set of keys rather than one, which is how a
subscription covers a fleet instead of a node. The language is small: chunks
separated by slashes, `*` for exactly one chunk, `**` for any number of them
including none, `$*` for part of a chunk, and chunks that start with `@`, which no
wildcard reaches. pamoja implements the rules the Zenoh RFC sets out, tested
against Zenoh's own implementation, so a gateway can decide what a subscription
covers, whether two of them overlap, and whether one already covers the other,
without a Zenoh installation anywhere near it.

## What the example does

It takes the keys a wind farm publishes under, a turbine's power, a turbine in a
row, the substation, and an alarm at any level, and asks what each wildcard
selects. Then it rewrites expressions into the one spelling a Zenoh session
accepts, joins a key beneath a prefix, and refuses three malformed expressions.

The second part asks two questions of a pair of expressions: whether they share a
key, which is how a router decides to forward, and whether one covers every key of
the other, which is how a bridge knows a new subscription adds nothing. The third
puts a second payload version under a verbatim `@v2` chunk and shows that no
wildcard reaches it.

It proves:

- `*` stands for exactly one chunk, whatever it holds, so `farm/*/power` covers
  `farm/t7/power` and `farm/substation/power` but not `farm/row2/t14/power`.
- `**` stands for any number of chunks, so `farm/**/power` covers
  `farm/row2/t14/power`, and `farm/**/alarm` covers `farm/alarm`, where it stands
  for none.
- `$*` selects on part of a chunk: `farm/t$*/power` covers `farm/t7/power` and not
  `farm/substation/power`.
- `farm/**/*/power` is spelled `farm/*/**/power` and `farm/**/**/power` is spelled
  `farm/**/power`, and a join canonizes the seam it makes.
- An empty chunk, a `*` inside a chunk, and a `?` each make an expression
  malformed, and canonizing one yields nothing.
- `farm/*/power` and `farm/t7/**` share a key, `farm/t7/power`, while
  `farm/*/power` and `farm/*/alarm` share none, and `farm/**` covers every key of
  `farm/*/power` but not the other way round.
- Two spellings of one set include each other.
- `farm/**` misses `farm/@v2/t7/power`, `farm/@v2/**` covers it, and
  `farm/@v1/**` and `farm/@v2/**` share no key.

## Run it

The example below is a program CI runs on every change, in each language, from a clone of the
repository:

<!-- table: run -->
<div class="run">
<div class="run-row"><p class="run-head"><span class="run-lang">Rust</span><button class="copy" type="button" data-copy="cargo run -p pamoja-examples --example zenoh" aria-label="Copy the command that runs the Rust example">copy</button></p><code class="run-cmd">cargo run -p pamoja-examples --example zenoh</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">TypeScript</span><button class="copy" type="button" data-copy="npm --prefix bindings/node run guides -- zenoh" aria-label="Copy the command that runs the TypeScript example">copy</button></p><code class="run-cmd">npm --prefix bindings/node run guides -- zenoh</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">Python</span><button class="copy" type="button" data-copy="python bindings/python/guides/zenoh.py" aria-label="Copy the command that runs the Python example">copy</button></p><code class="run-cmd">python bindings/python/guides/zenoh.py</code></div>
<div class="run-row"><p class="run-head"><span class="run-lang">C#</span><button class="copy" type="button" data-copy="dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- zenoh" aria-label="Copy the command that runs the C# example">copy</button></p><code class="run-cmd">dotnet run --project bindings/dotnet/samples/Pamoja.Guides -- zenoh</code></div>
</div>
<!-- end -->

## Rust

In Rust, `pamoja-zenoh` is `no_std`, and `keyexpr` is a set of free functions over
`&str`. The checks return `bool`, and `canonize` and `join` return
`Option<String>`, `None` for an expression they cannot make valid. Every function
reads any spelling as the set it names, so
`includes("farm/**/*/power", "farm/*/**/power")` holds without canonizing first. The
`runtime` feature adds `ZenohTransport`, a Zenoh session behind the core
`Transport` and `Receive` traits. `ZenohConfig::new()` is a peer that finds others
by multicast scouting, and `listen_on`, `connect_to`, and
`multicast_scouting(false)` pin a link to known endpoints. The transport hands a
key to Zenoh as written, so canonize it first.

<!-- snippet: examples/guides/zenoh.rs#example -->
From [`examples/guides/zenoh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/zenoh.rs):

```rust
use pamoja_zenoh::keyexpr::{canonize, is_canon, is_valid, join, matches};

// A key expression names a set of keys. Chunks sit between slashes, `*` stands for
// exactly one chunk, whatever it holds, and `**` for any number of them, including none.
let selects = |pattern: &str, key: &str| {
    let verdict = if matches(pattern, key) {
        "covers"
    } else {
        "misses"
    };
    format!("{verdict:<10}{pattern} {verdict} {key}")
};
println!("{}", selects("farm/*/power", "farm/t7/power"));
println!("{}", selects("farm/*/power", "farm/substation/power"));
println!(
    "{}, since * is exactly one chunk",
    selects("farm/*/power", "farm/row2/t14/power")
);
println!("{}", selects("farm/**/power", "farm/row2/t14/power"));
println!(
    "{}, where ** is no chunk at all",
    selects("farm/**/alarm", "farm/alarm")
);

// `$*` stands for any run of characters inside one chunk, so it selects on part of a name.
println!("{}", selects("farm/t$*/power", "farm/t7/power"));
println!("{}", selects("farm/t$*/power", "farm/substation/power"));

// One set of keys has one canonical spelling, and a Zenoh session accepts no other.
for written in ["farm/*/**/power", "farm/**/*/power", "farm/**/**/power"] {
    let canonical = canonize(written).expect("a valid expression");
    if is_canon(written) {
        println!("canonical {written}, as written");
    } else {
        println!("rewritten {written} is spelled {canonical}");
    }
}

// Joining places one expression beneath another, and canonizes the seam between them.
for (prefix, suffix) in [("farm/t7", "power"), ("farm/**", "*/power")] {
    let joined = join(prefix, suffix).expect("two valid halves");
    println!("joined    {prefix} and {suffix} make {joined}");
}

// A malformed expression is refused rather than repaired into something plausible.
for (written, why) in [
    ("farm//power", "a chunk is empty"),
    ("farm/t7*/power", "* stands alone in its chunk, or after $"),
    ("farm/t7/power?", "? and # are reserved"),
] {
    if !is_valid(written) && canonize(written).is_none() {
        println!("malformed {written}, since {why}");
    }
}
```
<!-- end -->

How two expressions relate, continuing from above:

<!-- snippet: examples/guides/zenoh.rs#relations -->
From [`examples/guides/zenoh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/zenoh.rs):

```rust
use pamoja_zenoh::keyexpr::{includes, intersects};

// Two expressions intersect when some key belongs to both. That is the question a router
// asks before it forwards a publication on one to a subscriber on the other.
let overlap = |a: &str, b: &str| {
    if intersects(a, b) {
        format!("overlap   {a} and {b} share a key")
    } else {
        format!("disjoint  {a} and {b} share no key")
    }
};
println!("{}", overlap("farm/*/power", "farm/t7/**"));
println!("{}", overlap("farm/*/power", "farm/*/alarm"));

// One includes the other when every key of the second belongs to the first, so a bridge
// that already holds the wider subscription declares nothing new for the narrower one.
let covers = |a: &str, b: &str| {
    if includes(a, b) {
        format!("included  {a} covers every key of {b}")
    } else {
        format!("wider     {b} holds keys {a} does not")
    }
};
println!("{}", covers("farm/**", "farm/*/power"));
println!("{}", covers("farm/*/power", "farm/**"));

// Two spellings of one set include each other, which compares expressions nobody
// canonized.
let (one, other) = ("farm/**/*/power", "farm/*/**/power");
if one != other && includes(one, other) && includes(other, one) {
    println!("same      {one} and {other} select the same keys");
}
```
<!-- end -->

The chunks no wildcard reaches, continuing from above:

<!-- snippet: examples/guides/zenoh.rs#sealed -->
From [`examples/guides/zenoh.rs`](https://github.com/molexxxx/pamoja/blob/main/examples/guides/zenoh.rs):

```rust
// A chunk that starts with @ is verbatim: no wildcard selects it, and only the same chunk
// matches it. A second payload version under @v2 stays out of every subscription that
// does not name it, the way Zenoh keeps its own administration space out of **.
let current = "farm/@v2/t7/power";
println!(
    "{}, since no wildcard selects a chunk that starts with @",
    selects("farm/**", current)
);
println!("{}", selects("farm/@v2/**", current));
println!(
    "{}, so a reader of one version never sees the other",
    overlap("farm/@v1/**", "farm/@v2/**")
);
```
<!-- end -->

## TypeScript

In TypeScript, `@pamoja/zenoh` exports one `keyexpr` object whose functions take
strings. The checks return a `boolean`, and `canonize` and `join` return
`string | null`. The package carries the rules and not a session: the zenoh stack
is std-only Rust and would land in every install, so a live session stays in the
Rust crate.

<!-- snippet: bindings/node/guides/zenoh.ts#example -->
From [`bindings/node/guides/zenoh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/zenoh.ts):

```typescript
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
```
<!-- end -->

How two expressions relate, continuing from above:

<!-- snippet: bindings/node/guides/zenoh.ts#relations -->
From [`bindings/node/guides/zenoh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/zenoh.ts):

```typescript
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
```
<!-- end -->

The chunks no wildcard reaches, continuing from above:

<!-- snippet: bindings/node/guides/zenoh.ts#sealed -->
From [`bindings/node/guides/zenoh.ts`](https://github.com/molexxxx/pamoja/blob/main/bindings/node/guides/zenoh.ts):

```typescript
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
```
<!-- end -->

## Python

In Python, `pamoja.zenoh` exports the same seven functions. The checks return a
`bool`, and `canonize` and `join` return `str | None`. The wheel carries the rules
and not a session, which stays in the Rust crate.

<!-- snippet: bindings/python/guides/zenoh.py#example -->
From [`bindings/python/guides/zenoh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/zenoh.py):

```python
from pamoja.zenoh import canonize, is_canon, is_valid, join, matches


# A key expression names a set of keys. Chunks sit between slashes, `*` stands for exactly one
# chunk, whatever it holds, and `**` for any number of them, including none.
def selects(pattern: str, key: str) -> str:
    verdict = "covers" if matches(pattern, key) else "misses"
    return f"{verdict:<10}{pattern} {verdict} {key}"


print(selects("farm/*/power", "farm/t7/power"))
print(selects("farm/*/power", "farm/substation/power"))
print(f"{selects('farm/*/power', 'farm/row2/t14/power')}, since * is exactly one chunk")
print(selects("farm/**/power", "farm/row2/t14/power"))
print(f"{selects('farm/**/alarm', 'farm/alarm')}, where ** is no chunk at all")

# `$*` stands for any run of characters inside one chunk, so it selects on part of a name.
print(selects("farm/t$*/power", "farm/t7/power"))
print(selects("farm/t$*/power", "farm/substation/power"))

# One set of keys has one canonical spelling, and a Zenoh session accepts no other.
for written in ("farm/*/**/power", "farm/**/*/power", "farm/**/**/power"):
    canonical = canonize(written)
    if is_canon(written):
        print(f"canonical {written}, as written")
    else:
        print(f"rewritten {written} is spelled {canonical}")

# Joining places one expression beneath another, and canonizes the seam between them.
for prefix, suffix in (("farm/t7", "power"), ("farm/**", "*/power")):
    print(f"joined    {prefix} and {suffix} make {join(prefix, suffix)}")

# A malformed expression is refused rather than repaired into something plausible.
for written, why in (
    ("farm//power", "a chunk is empty"),
    ("farm/t7*/power", "* stands alone in its chunk, or after $"),
    ("farm/t7/power?", "? and # are reserved"),
):
    if not is_valid(written) and canonize(written) is None:
        print(f"malformed {written}, since {why}")
```
<!-- end -->

How two expressions relate, continuing from above:

<!-- snippet: bindings/python/guides/zenoh.py#relations -->
From [`bindings/python/guides/zenoh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/zenoh.py):

```python
from pamoja.zenoh import includes, intersects


# Two expressions intersect when some key belongs to both. That is the question a router asks
# before it forwards a publication on one to a subscriber on the other.
def overlap(a: str, b: str) -> str:
    if intersects(a, b):
        return f"overlap   {a} and {b} share a key"
    return f"disjoint  {a} and {b} share no key"


print(overlap("farm/*/power", "farm/t7/**"))
print(overlap("farm/*/power", "farm/*/alarm"))


# One includes the other when every key of the second belongs to the first, so a bridge that
# already holds the wider subscription declares nothing new for the narrower one.
def covers(a: str, b: str) -> str:
    if includes(a, b):
        return f"included  {a} covers every key of {b}"
    return f"wider     {b} holds keys {a} does not"


print(covers("farm/**", "farm/*/power"))
print(covers("farm/*/power", "farm/**"))

# Two spellings of one set include each other, which compares expressions nobody canonized.
one, other = "farm/**/*/power", "farm/*/**/power"
if one != other and includes(one, other) and includes(other, one):
    print(f"same      {one} and {other} select the same keys")
```
<!-- end -->

The chunks no wildcard reaches, continuing from above:

<!-- snippet: bindings/python/guides/zenoh.py#sealed -->
From [`bindings/python/guides/zenoh.py`](https://github.com/molexxxx/pamoja/blob/main/bindings/python/guides/zenoh.py):

```python
# A chunk that starts with @ is verbatim: no wildcard selects it, and only the same chunk
# matches it. A second payload version under @v2 stays out of every subscription that does not
# name it, the way Zenoh keeps its own administration space out of **.
current = "farm/@v2/t7/power"
print(f"{selects('farm/**', current)}, since no wildcard selects a chunk that starts with @")
print(selects("farm/@v2/**", current))
print(f"{overlap('farm/@v1/**', 'farm/@v2/**')}, so a reader of one version never sees the other")
```
<!-- end -->

## C#

In C#, `Pamoja.Zenoh.KeyExpression` is a static class. The checks return `bool`,
and `Canonize` and `Join` return `string?`, `null` for an expression they cannot
make valid. The package carries the rules and not a session, which stays in the
Rust crate.

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs#example -->
From [`bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs):

```csharp
// A key expression names a set of keys. Chunks sit between slashes, `*` stands for
// exactly one chunk, whatever it holds, and `**` for any number of them, including none.
static string Selects(string pattern, string key)
{
    string verdict = KeyExpression.Matches(pattern, key) ? "covers" : "misses";
    return $"{verdict,-10}{pattern} {verdict} {key}";
}
Console.WriteLine(Selects("farm/*/power", "farm/t7/power"));
Console.WriteLine(Selects("farm/*/power", "farm/substation/power"));
Console.WriteLine(
    $"{Selects("farm/*/power", "farm/row2/t14/power")}, since * is exactly one chunk");
Console.WriteLine(Selects("farm/**/power", "farm/row2/t14/power"));
Console.WriteLine(
    $"{Selects("farm/**/alarm", "farm/alarm")}, where ** is no chunk at all");

// `$*` stands for any run of characters inside one chunk, so it selects on part of a
// name.
Console.WriteLine(Selects("farm/t$*/power", "farm/t7/power"));
Console.WriteLine(Selects("farm/t$*/power", "farm/substation/power"));

// One set of keys has one canonical spelling, and a Zenoh session accepts no other.
foreach (string written in new[] { "farm/*/**/power", "farm/**/*/power", "farm/**/**/power" })
{
    string? canonical = KeyExpression.Canonize(written);
    if (KeyExpression.IsCanon(written))
    {
        Console.WriteLine($"canonical {written}, as written");
    }
    else
    {
        Console.WriteLine($"rewritten {written} is spelled {canonical}");
    }
}

// Joining places one expression beneath another, and canonizes the seam between them.
foreach ((string prefix, string suffix) in new[] { ("farm/t7", "power"), ("farm/**", "*/power") })
{
    Console.WriteLine(
        $"joined    {prefix} and {suffix} make {KeyExpression.Join(prefix, suffix)}");
}

// A malformed expression is refused rather than repaired into something plausible.
foreach ((string written, string why) in new[]
{
    ("farm//power", "a chunk is empty"),
    ("farm/t7*/power", "* stands alone in its chunk, or after $"),
    ("farm/t7/power?", "? and # are reserved"),
})
{
    if (!KeyExpression.IsValid(written) && KeyExpression.Canonize(written) is null)
    {
        Console.WriteLine($"malformed {written}, since {why}");
    }
}
```
<!-- end -->

How two expressions relate, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs#relations -->
From [`bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs):

```csharp
// Two expressions intersect when some key belongs to both. That is the question a
// router asks before it forwards a publication on one to a subscriber on the other.
static string Overlap(string a, string b) =>
    KeyExpression.Intersects(a, b)
        ? $"overlap   {a} and {b} share a key"
        : $"disjoint  {a} and {b} share no key";
Console.WriteLine(Overlap("farm/*/power", "farm/t7/**"));
Console.WriteLine(Overlap("farm/*/power", "farm/*/alarm"));

// One includes the other when every key of the second belongs to the first, so a
// bridge that already holds the wider subscription declares nothing new for the
// narrower one.
static string Covers(string a, string b) =>
    KeyExpression.Includes(a, b)
        ? $"included  {a} covers every key of {b}"
        : $"wider     {b} holds keys {a} does not";
Console.WriteLine(Covers("farm/**", "farm/*/power"));
Console.WriteLine(Covers("farm/*/power", "farm/**"));

// Two spellings of one set include each other, which compares expressions nobody
// canonized.
(string one, string other) = ("farm/**/*/power", "farm/*/**/power");
if (one != other && KeyExpression.Includes(one, other) && KeyExpression.Includes(other, one))
{
    Console.WriteLine($"same      {one} and {other} select the same keys");
}
```
<!-- end -->

The chunks no wildcard reaches, continuing from above:

<!-- snippet: bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs#sealed -->
From [`bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs`](https://github.com/molexxxx/pamoja/blob/main/bindings/dotnet/samples/Pamoja.Guides/ZenohGuide.cs):

```csharp
// A chunk that starts with @ is verbatim: no wildcard selects it, and only the same
// chunk matches it. A second payload version under @v2 stays out of every
// subscription that does not name it, the way Zenoh keeps its own administration
// space out of **.
const string Current = "farm/@v2/t7/power";
Console.WriteLine(
    $"{Selects("farm/**", Current)}, since no wildcard selects a chunk that starts with @");
Console.WriteLine(Selects("farm/@v2/**", Current));
Console.WriteLine(
    $"{Overlap("farm/@v1/**", "farm/@v2/**")}, so a reader of one version never sees the other");
```
<!-- end -->

## Values at a glance

**The chunk forms,** from the Key Expressions RFC:

| Chunk | Stands for | Pattern | Covers | Misses |
| --- | --- | --- | --- | --- |
| a literal such as `t7` | exactly that chunk | `farm/t7/power` | `farm/t7/power` | `farm/t8/power` |
| `*` | one chunk, whatever it holds | `farm/*/power` | `farm/t7/power`, `farm/substation/power` | `farm/power`, `farm/row2/t14/power` |
| `**` | any number of chunks, including none | `farm/**/power` | `farm/power`, `farm/row2/t14/power` | `farm/t7/power/avg` |
| `$*` inside a chunk | any run of characters in that chunk, including none | `farm/t$*/power` | `farm/t/power`, `farm/t14/power` | `farm/substation/power` |
| `@` and a name | only that same chunk | `farm/@v2/**` | `farm/@v2/t7/power` | `farm/t7/power` |

No wildcard selects a chunk that starts with `@`, so `farm/**` misses
`farm/@v2/t7/power`, and Zenoh keeps its own administration space under a leading
`@` chunk for the same reason.

**What makes an expression malformed:**

| Rule | Breaks it |
| --- | --- |
| not empty, with no leading or trailing `/` | `""`, `/farm/power`, `farm/power/` |
| no empty chunk | `farm//power` |
| `*` only as a whole chunk, as `**`, or in `$*` | `farm/t7*/power`, `farm/**x/power` |
| `$` only in `$*` | `farm/$t7/power` |
| no `?` or `#` | `farm/t7/power?`, `farm/#` |

**The one spelling,** by the RFC's canonical-form rules:

| Rule | Written | Canonical |
| --- | --- | --- |
| a run of `**` chunks is one `**` | `farm/**/**/power` | `farm/**/power` |
| `**/*` becomes `*/**` | `farm/**/*/power` | `farm/*/**/power` |
| a run of `$*` is one `$*` | `farm/t$*$*/power` | `farm/t$*/power` |
| a chunk that is only `$*` is `*` | `farm/$*/power` | `farm/*/power` |

A Zenoh session accepts only the canonical form, so `is_canon` is the check a key
passes before it goes on the wire, and `canonize` the repair.

**Two expressions:**

| Question | Call | Holds when | Asked by |
| --- | --- | --- | --- |
| does a pattern select this key | `matches(pattern, key)` | the key, which carries no wildcard, is one of the pattern's keys | a subscriber, of each publication |
| do two patterns share a key | `intersects(a, b)` | at least one key belongs to both, either way round | a router, before it forwards |
| does one pattern cover another | `includes(a, b)` | every key of `b` belongs to `a` | a bridge, before it declares a second subscription |
| are two patterns one set | `includes` both ways, or `canonize` both and compare | both hold, or the two spellings match | anything that compares expressions as text |

Every call answers false, or nothing, for an expression that is not valid.

**The calls in each language:**

### Rust

| To | Call |
| --- | --- |
| check an expression | `keyexpr::is_valid(ke)`, `keyexpr::is_canon(ke)` |
| spell it the one way | `keyexpr::canonize(ke)`, `keyexpr::join(prefix, suffix)` |
| route a key | `keyexpr::matches(pattern, key)` |
| relate two expressions | `keyexpr::intersects(a, b)`, `keyexpr::includes(a, b)` |
| open a session, with the `runtime` feature | `ZenohTransport::new(ZenohConfig::new().connect_to(endpoint))`, then `connect`, `subscribe`, `send`, and `recv` |

### TypeScript

| To | Call |
| --- | --- |
| check an expression | `keyexpr.isValid(ke)`, `keyexpr.isCanon(ke)` |
| spell it the one way | `keyexpr.canonize(ke)`, `keyexpr.join(prefix, suffix)` |
| route a key | `keyexpr.matches(pattern, key)` |
| relate two expressions | `keyexpr.intersects(a, b)`, `keyexpr.includes(a, b)` |

### Python

| To | Call |
| --- | --- |
| check an expression | `is_valid(ke)`, `is_canon(ke)` |
| spell it the one way | `canonize(ke)`, `join(prefix, suffix)` |
| route a key | `matches(pattern, key)` |
| relate two expressions | `intersects(a, b)`, `includes(a, b)` |

### C#

| To | Call |
| --- | --- |
| check an expression | `KeyExpression.IsValid(key)`, `KeyExpression.IsCanon(key)` |
| spell it the one way | `KeyExpression.Canonize(key)`, `KeyExpression.Join(prefix, suffix)` |
| route a key | `KeyExpression.Matches(pattern, key)` |
| relate two expressions | `KeyExpression.Intersects(a, b)`, `KeyExpression.Includes(a, b)` |

<!-- languages end -->

## When it goes wrong

A malformed expression comes back as false or nothing rather than as an error.
What gets past the rules shows up as a subscriber that hears nothing, or hears more
than it meant to. The ones that cost an afternoon:

- **A publication is refused, or a connection drops.** A Zenoh session accepts
  only the canonical form: its API refuses `farm/**/*/power`, and a router that
  receives a non-canonical expression drops the message and closes the
  connection. Canonize an expression built from parts, or build it with `join`.
- **A subscription on `**` never hears some keys.** No wildcard selects a chunk
  that starts with `@`, so `**` misses everything beneath one, Zenoh's own
  administration space included. Name the chunk: `farm/@v2/**`.
- **`*` misses a key one level deeper.** `*` is exactly one chunk, so
  `farm/*/power` misses `farm/row2/t14/power`. Use `**` for any depth, including
  none.
- **A pattern tested as a key matches nothing.** `matches` takes a concrete key on
  its right, and answers false for one that carries a wildcard. Ask `intersects`
  or `includes` of two patterns.
- **A typo subscribes to nothing.** Every call answers false for an expression
  that is not valid, so a subscription on `farm//power` is quietly empty. Check
  `is_valid` where an expression enters the program, from a file or from another
  node.
- **Two spellings count as two subscriptions.** Compared as text,
  `farm/**/*/power` and `farm/*/**/power` differ. Canonize both, or ask `includes`
  both ways.
- **Matching gets slow.** The RFC warns that `$*` costs a Zenoh network more than a
  whole-chunk wildcard. Give each variable part of a key its own chunk,
  `farm/row/2/turbine/14` rather than `farm/row2-t14`, and select with `*`.

## Where next

<!-- table: next zenoh -->
- [ROS 2 rules](ros2.md): ROS 2 names, RIHS01 type hashes, CDR encoding, and rmw_zenoh key assembly, with no ROS 2 installed.
- [MQTT](mqtt.md): An MQTT client with the topic and wildcard rules, as the core transport.
- [Engine surface](transport.md): The transport every link shares (send, receive, subscribe, and a faulty wrapper for tests) and the runtime version.
- Also in Profiles and robotics: [Device profiles](profile.md), [Rules](rules.md), [Robot motion](motion.md).
<!-- end -->

## Reference

<!-- table: reference zenoh -->
- Rust: [`pamoja-zenoh`](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-zenoh)
- TypeScript: [`@pamoja/zenoh`](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-zenoh)
- Python: [`pamoja.zenoh`](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-zenoh)
- C#: [`Pamoja.Zenoh`](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-zenoh)
<!-- end -->
