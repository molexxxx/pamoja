# The .NET binding

One project per package under `src/`, shaped like the crates:

```
src/Pamoja.Native/      Pamoja.Native: the P/Invoke contract (Interop/), mirroring the
                        generated C header, the native library per runtime identifier, and
                        the marshalling every facade needs (the handle type, the status
                        helpers, owned strings, and PamojaException, which sits in the root
                        Pamoja namespace so a facade sees it without a using); every other
                        package depends on it
src/Pamoja.Core/        Pamoja.Core: the engine's own surface, the runtime version and the
                        transport every link implements. A capability like the others: only
                        the transports depend on it
src/Pamoja.<Name>/      Pamoja.<Name>: the hand-written facade for one capability, in the
                        namespace of the same name; the project file and README are generated
src/Pamoja.<Domain>/    Pamoja.<Domain>: one per chapter of the guides holding more than one
                        capability. It ships no assembly, since C# cannot re-export a
                        namespace; it brings in its capability packages, all generated
src/Pamoja/             Pamoja: a metapackage that installs every package
tests/                  the smoke tests and the cross-language conformance suite
samples/                the examples the documentation site splices
docs/                   the DocFX build of every package
```

`cargo xtask docs` renders each package's project file and README from
`docs/capabilities.toml`, deriving project references from the package's own
`using` directives; `cargo xtask docs --check` fails if any of them is stale.

## Build and test

```
cargo build -p pamoja-ffi --release            # the native library and pamoja.h
dotnet build Pamoja.sln -c Release             # every package
dotnet run --project tests/Pamoja.Smoke -c Release
```

The first command builds `pamoja_ffi` and regenerates the committed header the
interop layer mirrors; CI checks both against the Rust source. The smoke project
runs the smoke tests and the conformance suite; it copies the native library
from `target/release` next to its executable, the way the release package
bundles it under `runtimes/`.
