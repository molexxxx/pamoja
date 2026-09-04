# Pamoja.Native

The compiled pamoja engine for .NET, bundled for `win-x64`, `linux-x64`, `linux-arm64`, `osx-x64`, and `osx-arm64`, and the P/Invoke contract every `Pamoja` package builds on: `Pamoja.Native.Interop.NativeMethods` mirrors the generated C header one-to-one. It also carries the marshalling a facade needs to use that contract: the safe handle type, the status helpers, owned strings, and `PamojaException`, which every failed native call raises and which sits in the root `Pamoja` namespace so a facade sees it without a using. It is one library that carries every capability; the capability packages are facades over it, so picking packages narrows the API you depend on, not the size of the engine.

You do not install this package directly. Every `Pamoja.<Capability>` package and the `Pamoja` metapackage depend on it. The interop layer stays available for anything a facade does not cover.

## Documentation

- [The guides](https://pamoja.molex.cloud/docs/) and the [C# reference](https://pamoja.molex.cloud/docs/reference/dotnet/index.html).

## License

MIT
