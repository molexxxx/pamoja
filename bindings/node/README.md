# @pamoja/core

Node.js bindings for the [pamoja](https://github.com/molexxxx/pamoja)
device SDK core, built with [napi-rs](https://napi.rs).

The generated surface is intentionally thin. A hand-written, idiomatic layer is
added on top of it so JavaScript and TypeScript callers get a native-feeling API
while all behavior stays in the Rust core.

## Build

```
npm install
npm run build
npm test
```

`npm run build` compiles the Rust core into a native Node addon and emits
`index.js` and `index.d.ts`. Both are generated artifacts, but they are
committed and drift-checked in CI, so they can never fall behind the Rust
source. `index.js` also carries the package version, so a version bump means
rebuilding and committing it.
