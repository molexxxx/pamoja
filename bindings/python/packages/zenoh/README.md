# pamoja-zenoh

Zenoh key expressions: validity, canonical form, matching, and whether two expressions share or cover keys. One capability of [pamoja](https://github.com/molexxxx/pamoja), one memory-safe Rust core with bindings for TypeScript, Python, and C#.

[![read the guide](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-guide.svg)](https://pamoja.molex.cloud/docs/guides/zenoh.html)
[![documentation](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-docs.svg)](https://pamoja.molex.cloud/docs/)
[![API reference](https://raw.githubusercontent.com/molexxxx/pamoja/main/.github/badges/btn-api.svg)](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html)

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

## The same capability in every language

| Language | Package | Reference |
| --- | --- | --- |
| Rust | [`pamoja-zenoh`](https://crates.io/crates/pamoja-zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/rust/pamoja_zenoh/index.html), [docs.rs](https://docs.rs/pamoja-zenoh), [install](https://pamoja.molex.cloud/docs/reference/rust.html#rust-zenoh) |
| TypeScript | [`@pamoja/zenoh`](https://www.npmjs.com/package/@pamoja/zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/node/modules/_pamoja_zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/node.html#node-zenoh) |
| Python | [`pamoja-zenoh`](https://pypi.org/project/pamoja-zenoh/) | [reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/python.html#python-zenoh) |
| C# | [`Pamoja.Zenoh`](https://www.nuget.org/packages/Pamoja.Zenoh) | [reference](https://pamoja.molex.cloud/docs/reference/dotnet/api/Pamoja.Zenoh.html), [install](https://pamoja.molex.cloud/docs/reference/dotnet.html#dotnet-zenoh) |

## Documentation

- [`pamoja.zenoh` reference](https://pamoja.molex.cloud/docs/reference/python/pamoja/zenoh.html), every class and function in this module.
- [The Zenoh keys guide](https://pamoja.molex.cloud/docs/guides/zenoh.html), with the same example in Rust, TypeScript, and C#.
- [Every capability](https://pamoja.molex.cloud/docs/), and the [install page](https://pamoja.molex.cloud/docs/install.html).

## License

MIT
