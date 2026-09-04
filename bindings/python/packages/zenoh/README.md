# pamoja-zenoh

Zenoh key expressions: validity, canonical form, and wildcard matching. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html)
[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/zenoh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)

## Install

```sh
pip install pamoja-zenoh
```

```python
from pamoja import zenoh
```

This pulls in `pamoja-native`, the compiled engine. `pip install pamoja` is the whole framework in one package.

## Example

The script the test suite runs, spliced here as it ran.

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-zenoh`](https://crates.io/crates/pamoja-zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html), [docs.rs](https://docs.rs/pamoja-zenoh) |
| TypeScript | [`@pamoja/zenoh`](https://www.npmjs.com/package/@pamoja/zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html) |
| Python | [`pamoja-zenoh`](https://pypi.org/project/pamoja-zenoh/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html) |
| C# | [`Pamoja.Zenoh`](https://www.nuget.org/packages/Pamoja.Zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html) |

## Documentation

- [`pamoja.zenoh` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html), every class and function in this module.
- [The Zenoh keys guide](https://pamoja.molex.cloud/docs/guides/zenoh.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
