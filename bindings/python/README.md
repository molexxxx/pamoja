# The Python binding

One distribution per package under `packages/`, shaped like the crates:

```
packages/native/        pamoja-native: the PyO3 crate, built by maturin into the
                        extension module pamoja._native, with its committed type
                        stub and pamoja.raw; every other distribution depends on it
packages/core/          pamoja-core: pamoja/core, the engine's surface
packages/<capability>/  pamoja-<capability>: pamoja/<capability>, the hand-written
                        facade; pyproject.toml, README.md, and py.typed are generated
packages/pamoja/        pamoja: a metapackage that installs every distribution
tests/                  the smoke tests and the cross-language conformance suite
guides/                 the examples the documentation site splices
```

`pamoja` is a namespace package (PEP 420): each distribution ships one
`pamoja/<name>/` directory and they merge on import, so `pip install pamoja-mqtt`
gives `pamoja.mqtt` and pulls in `pamoja-native` and nothing else.

`cargo xtask docs` renders each package's `pyproject.toml` and README from
`docs/capabilities.toml`, deriving dependencies from the package's own imports;
`cargo xtask docs --check` fails if any of them is stale.

## Build and test

```
python -m venv .venv
.venv/bin/pip install maturin pytest
.venv/bin/maturin develop -m packages/native/Cargo.toml
.venv/bin/pip install $(find packages -mindepth 1 -maxdepth 1 -type d ! -name native)
.venv/bin/python -m pytest
```

`maturin develop` compiles the engine and installs `pamoja-native` into the
environment; the second `pip install` adds every pure distribution. `pytest`
runs the smoke tests, the conformance suite, the facades' doctests, and the
guide examples. `cargo run --bin stub_gen --manifest-path packages/native/Cargo.toml`
regenerates the committed stub `packages/native/python/pamoja/_native/__init__.pyi`,
which CI checks against the Rust source.
