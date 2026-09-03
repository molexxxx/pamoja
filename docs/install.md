# Install

Install the core, plus only the capabilities you need. Every binding wraps the
same Rust engine, so the same concepts carry across languages.

## Rust

Each capability is its own crate. Add the core and the ones you use:

```sh
cargo add pamoja-core pamoja-codec pamoja-kit
```

Most capability crates are `no_std`, so the same code runs on a gateway and on a
microcontroller. Their READMEs on [crates.io](https://crates.io/users/tonywied17)
say which features each one takes.

## TypeScript and Node

One package carries every capability, each behind its own subpath:

```sh
npm install @pamoja/core
```

```ts
import { DeviceIdentity } from '@pamoja/core/security'
import { toCbor, fromCbor } from '@pamoja/core/codec'
```

The native addon is prebuilt for Linux (x64 and arm64), macOS (x64 and
arm64), and Windows (x64); npm picks the right one. Node 16 or later.

## Python

```sh
pip install pamoja-core
```

```python
from pamoja import security, codec
```

Wheels exist for the same platforms as the Node addon, for Python 3.10 and
later; on any other platform `pip` builds the extension from the sdist, which
needs a Rust toolchain.

## C# and .NET

```sh
dotnet add package Pamoja.Core
```

```csharp
using Pamoja.Core;
```

The package carries the native library for `win-x64`, `linux-x64`,
`linux-arm64`, `osx-x64`, and `osx-arm64` and targets .NET 8.

## Versions

Every crate, package, and binding shares one version and is released together,
so `0.1.15` of the Node package wraps `0.1.15` of every crate. The
[changelog](https://github.com/molexxxx/pamoja/blob/main/CHANGELOG.md) covers all
of them in one entry.
