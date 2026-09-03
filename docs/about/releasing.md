# Releasing

Every crate, the npm, PyPI, and NuGet packages, and the language bindings share
one version and ship together. A release is one pull request and one tag.

## The release pull request

1. Branch from `main`: `git switch -c release/0.1.15`.
2. Set the version everywhere: `cargo xtask version 0.1.15`. This rewrites the
   workspace manifest and its dependency pins, every crate and binding
   manifest, the npm platform packages, `pyproject.toml`,
   `Directory.Build.props`, and the version napi-rs embeds in the Node loader,
   then refreshes the cargo and npm lockfiles.
3. Move the `Unreleased` entries in `CHANGELOG.md` under a `## [0.1.15] - <date>`
   heading. The check in the next step fails until that heading exists.
4. Verify:
   - `cargo run -p xtask -- version --check 0.1.15`
   - `cargo run -p xtask -- docs --check`
   - `cargo run -p xtask -- release --dry-run`, which packages and verifies
     every crate in one `cargo publish --workspace --dry-run`
   - `cargo test --workspace`
5. Commit as `Bump the workspace to 0.1.15`, open the pull request with the
   `release` label, and merge it once every check is green.

## The tag

Wait for `ci`, `node`, `python`, `dotnet`, and `pages` to pass on the merge
commit. Then tag that commit and push the tag:

```sh
git tag -a v0.1.15 -m "pamoja 0.1.15" <merge sha>
git push origin v0.1.15
```

The tag starts five workflows. Each one runs the preflight first and publishes
only if it passes:

- `release-github` creates the GitHub Release from the `CHANGELOG.md` entry and
  the pull requests since the previous tag, grouped by label.
- `release-crates` publishes every crate in the order `cargo xtask release
  --plan` prints. A version already on crates.io is skipped, and a crate
  throttled by the new-crate limit is retried after a wait.
- `release-node` builds the native addon for each platform and publishes the
  platform packages and `@pamoja/core`.
- `release-python` builds the wheels and the sdist, installs the sdist from
  source to prove it builds alone, and publishes to PyPI.
- `release-nuget` builds the native library for each runtime and publishes
  `Pamoja.Core`.

## The preflight

`release-preflight` is a reusable workflow every release workflow calls first.
It fails, before anything is uploaded, unless:

- every manifest, lockfile, and generated file carries the tagged version
  (`cargo xtask version --check`);
- the tagged commit is on `main`;
- `ci`, `node`, `python`, and `dotnet` all have a successful run on that commit.

Rehearse it without a tag from the Actions tab: dispatch `release-preflight`
with the version you intend to release. It passes on `main` only when the tree
is ready to tag.

## Recovering

If a release workflow stops part way, dispatch it again from the Actions tab
with the version as its input. Every publish step skips what already exists, so
a rerun finishes the release rather than duplicating it.

Never re-tag a version. Registries refuse a second upload of the same version,
and npm fails hard on it. Fix forward with the next patch version instead.
