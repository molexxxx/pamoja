# @pamoja/native

The compiled pamoja engine for Node, prebuilt for Linux (x64, arm64), macOS (x64, arm64), and Windows (x64), and the generated napi-rs contract every `@pamoja` package builds on. It is one binary that carries every capability; the capability packages are facades over it, so picking packages narrows the API you depend on, not the size of the engine.

You do not install this package directly. Every `@pamoja/<capability>` package and the `pamoja` bundle depend on it. `index.d.ts` types the contract for anything a facade does not cover.

## Documentation

- [The guides](https://pamoja.molex.cloud/docs/) and the [TypeScript reference](https://pamoja.molex.cloud/docs/reference/node/index.html).

## License

MIT
