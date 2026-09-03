# pamoja workflows. Run `just` to list available recipes.

# show all recipes
default:
    @just --list

# install required toolchain components
setup:
    rustup component add rustfmt clippy

# format the whole workspace
fmt:
    cargo fmt --all

# check formatting without writing changes
fmt-check:
    cargo fmt --all -- --check

# type-check the workspace
check:
    cargo check --workspace --all-targets

# lint with clippy, warnings treated as errors
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# run the test suite
test:
    cargo test --workspace

# build the workspace
build:
    cargo build --workspace

# build the feature-gated no_std crates without std, as CI does
nostd:
    cargo build --no-default-features -p pamoja-core -p pamoja-codec -p pamoja-security -p pamoja-audit -p pamoja-zenoh -p pamoja-ros2 -p pamoja-mavlink
    cargo build --no-default-features -p pamoja-kit --features "robotics geo"

# run the dashboard guards: mock tests, i18n bundles, footprint budgets, tier builds
dashboard-checks:
    cargo test -p pamoja-dashboard --features mock
    cargo run -p xtask -- dashboard i18n --check
    cargo run -p xtask -- dashboard footprint
    cargo build -p pamoja-dashboard --no-default-features --features "serve,tier-c"
    cargo build -p pamoja-dashboard --no-default-features --features "serve,tier-b,locale-sw"

# verify the generated crate READMEs, the site navigation, and the doc regions are in sync
docs-check:
    cargo run -p xtask -- docs --check

# run the guide examples in every language (the code the documentation site shows)
guides:
    cargo test -p pamoja-examples --test guides
    cd bindings/node && npm run test:guides
    cd bindings/python && python -m pytest tests/test_guides.py
    dotnet run --project bindings/dotnet/samples/Pamoja.Guides -c Release

# audit dependencies against deny.toml (needs cargo-deny installed)
deny:
    cargo deny check

# verify every manifest, lockfile, and generated loader carries the workspace version
version-check:
    cargo run -p xtask -- version --check

# run everything the main CI job runs
ci: fmt-check lint nostd test dashboard-checks docs-check version-check release-plan

# set the workspace version everywhere and refresh the lockfiles (just bump 0.2.0)
bump version:
    cargo xtask version {{version}}

# print the crates.io publish order derived from cargo metadata
release-plan:
    cargo run -p xtask -- release --plan

# publish every workspace crate to crates.io in dependency order
release:
    cargo xtask release

# package and verify every crate without uploading
release-dry:
    cargo xtask release --dry-run
